use crate::recording::{Cycle, Recording, Reference};
use ball_filter::tracker::Tracker;
use color_eyre::{Result, eyre::ensure};
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2, Point3};
use ros_z::time::Time;
use serde::Serialize;
use types::{ball_position::BallPosition, parameters::BallFilterParameters};

pub const MISSING_PENALTY_MULTIPLIER: f64 = 1.25;
pub const OUT_OF_FIELD_DECAY_METRES: f64 = 0.3;

/// Stored with every report so scores from different objectives are not confused.
#[derive(Serialize)]
pub struct Objective {
    pub version: &'static str,
    pub position_loss: &'static str,
    pub missing_loss: &'static str,
    pub false_track_loss: &'static str,
    pub normalization: &'static str,
    pub reference_weight: &'static str,
    pub out_of_field_decay_metres: f64,
}

pub const OBJECTIVE: Objective = Objective {
    version: "single_ball_distance_weighted_v3",
    position_loss: "p^2 * d^2 / (p^2 + d^2); d = distance to the single labelled ball",
    missing_loss: "1.25 * p^2",
    false_track_loss: "p^2",
    normalization: "weighted time integral divided by weighted_seconds; p = penalty_metres",
    reference_weight: "exp(-distance_outside_field_metres / out_of_field_decay_metres); distance outside Field rectangle expanded by ball radius; absent or unknown Field pose weight=1",
    out_of_field_decay_metres: OUT_OF_FIELD_DECAY_METRES,
};

#[derive(Default, Debug, Serialize)]
pub struct Score {
    pub loss: f64,
    pub position_rmse_metres: Option<f64>,
    pub labelled_seconds: f64,
    pub weighted_seconds: f64,
    pub out_of_field_seconds: f64,
    pub unlabelled_seconds: f64,
    pub present_seconds: f64,
    pub missing_seconds: f64,
    /// Includes initial acquisition and unavailable field transforms, not just lost tracks.
    pub missing_runs: u64,
    pub longest_missing_seconds: f64,
    pub close_range_position_rmse_metres: Option<f64>,
    pub close_range_present_seconds: f64,
    pub close_range_missing_seconds: f64,
    /// Signed along the ball's motion: negative means the estimate is behind.
    pub along_motion_error_metres: Option<f64>,
    /// Equivalent spatial lag, positive behind. Not measured processing latency.
    pub motion_lag_seconds: Option<f64>,
    pub moving_reference_seconds: f64,
    pub absent_seconds: f64,
    pub false_track_seconds: f64,
    pub missing_transform_seconds: f64,
    #[serde(skip)]
    squared_error: f64,
    #[serde(skip)]
    current_missing_seconds: f64,
    #[serde(skip)]
    close_range_squared_error: f64,
    #[serde(skip)]
    along_motion_error_integral: f64,
    #[serde(skip)]
    motion_lag_integral: f64,
}

impl Score {
    fn observe_cycle(
        &mut self,
        cycle: &Cycle,
        estimate: Option<BallPosition<Ground>>,
        penalty: f64,
    ) {
        let loss_before = self.loss;
        match &cycle.reference {
            Some(Reference::Ground(reference)) => {
                self.observe(Some(reference), estimate, cycle.seconds, penalty)
            }
            Some(Reference::Field(reference)) => self.observe_field(
                reference,
                estimate,
                cycle.ground_to_field,
                cycle.seconds,
                penalty,
            ),
            None => self.observe::<Ground>(None, estimate, cycle.seconds, penalty),
        }
        if cycle.reference.is_some() {
            let distance = reference_distance_outside_field(cycle);
            let weight = (-distance / OUT_OF_FIELD_DECAY_METRES).exp();
            self.weighted_seconds += cycle.seconds * weight;
            if distance > 0.0 {
                self.out_of_field_seconds += cycle.seconds;
            }
            // Only the optimization objective is weighted. Raw spatial errors,
            // missing durations and uninterrupted gap lengths remain unchanged.
            self.loss = loss_before + (self.loss - loss_before) * weight;
        }
    }

    fn observe_diagnostics(
        &mut self,
        cycle: &Cycle,
        estimate: Option<BallPosition<Ground>>,
        previous: &mut Option<(Time, Point2<Field>)>,
    ) {
        // Diagnose a ball the robot could kick now, within 1 m of its Ground origin.
        let mut close_squared = None::<f64>;
        let mut close_present = false;
        let mut observe_ground = |truth: Point2<Ground>| {
            if truth.coords().norm_squared() <= 1.0 {
                close_present = true;
                if let Some(estimate) = estimate {
                    let squared = f64::from((truth - estimate.position).norm_squared());
                    close_squared = Some(close_squared.map_or(squared, |old| old.min(squared)));
                }
            }
        };
        let single_field = match &cycle.reference {
            Some(Reference::Ground(points)) => {
                for point in points {
                    observe_ground(point.xy());
                }
                match points.as_slice() {
                    [point] => cycle.ground_to_field.map(|pose| pose * point.xy()),
                    _ => None,
                }
            }
            Some(Reference::Field(points)) => {
                if let Some(pose) = cycle.ground_to_field {
                    let inverse = pose.inverse();
                    for point in points {
                        observe_ground(inverse * point.xy());
                    }
                }
                match points.as_slice() {
                    [point] => Some(point.xy()),
                    _ => None,
                }
            }
            None => None,
        };
        if close_present {
            self.close_range_present_seconds += cycle.seconds;
            if let Some(squared) = close_squared {
                self.close_range_squared_error += squared * cycle.seconds;
            } else {
                self.close_range_missing_seconds += cycle.seconds;
            }
        }
        // Never infer identities between multiple balls. Field coordinates are
        // essential: differences in Ground include robot translation/rotation.
        let Some(truth) = single_field else {
            *previous = None;
            return;
        };
        let old = previous.replace((cycle.time, truth));
        let Some((time, position)) = old else { return };
        let dt = (cycle.time.as_nanos() - time.as_nanos()) as f64 * 1e-9;
        if !(0.0 < dt && dt <= 0.1) {
            return;
        }
        let velocity = (truth - position) / dt as f32;
        let speed = velocity.norm();
        // Exclude stationary numerical jitter and implausible discontinuities.
        if !(0.5..=15.0).contains(&speed) {
            return;
        }
        if let Some(estimate) = estimate.zip(cycle.ground_to_field).map(|(b, p)| p * b) {
            let along = f64::from((estimate.position - truth).dot(&velocity) / speed);
            self.along_motion_error_integral += along * cycle.seconds;
            self.motion_lag_integral += -along / f64::from(speed) * cycle.seconds;
            self.moving_reference_seconds += cycle.seconds;
        }
    }

    fn observe_field(
        &mut self,
        reference: &[Point3<Field>],
        estimate: Option<BallPosition<Ground>>,
        pose: Option<Isometry2<Ground, Field>>,
        seconds: f64,
        penalty: f64,
    ) {
        if pose.is_none() {
            self.missing_transform_seconds += seconds;
        }
        if reference.is_empty() {
            // A false track is still a false track if geometry is missing.
            self.observe::<Ground>(Some(&[]), estimate, seconds, penalty);
        } else {
            self.observe(
                Some(reference),
                estimate.and_then(|ball| pose.map(|pose| pose * ball)),
                seconds,
                penalty,
            );
        }
    }

    fn observe<Frame>(
        &mut self,
        reference: Option<&[Point3<Frame>]>,
        estimate: Option<BallPosition<Frame>>,
        seconds: f64,
        penalty: f64,
    ) {
        let Some(reference) = reference else {
            self.unlabelled_seconds += seconds;
            self.current_missing_seconds = 0.0;
            return;
        };
        self.labelled_seconds += seconds;
        if reference.is_empty() || estimate.is_some() {
            self.current_missing_seconds = 0.0;
        }
        match (reference.first(), estimate) {
            (Some(truth), Some(estimate)) => {
                self.present_seconds += seconds;
                // Optimization recordings have exactly one target when present.
                let squared = f64::from((truth.xy() - estimate.position).norm_squared());
                self.squared_error += seconds * squared;
                // Keep the raw RMSE above for honesty about spatial accuracy.
                // Bound only the optimization loss, retaining a gradient even
                // for distant predictions. Dropping a difficult track must not
                // beat any finite position error at that instant.
                let cap = penalty.powi(2);
                self.loss += seconds * cap * (squared / (cap + squared));
            }
            (Some(_), None) => {
                self.present_seconds += seconds;
                self.missing_seconds += seconds;
                if self.current_missing_seconds == 0.0 && seconds > 0.0 {
                    self.missing_runs += 1;
                }
                self.current_missing_seconds += seconds;
                self.longest_missing_seconds = self
                    .longest_missing_seconds
                    .max(self.current_missing_seconds);
                self.loss += seconds * MISSING_PENALTY_MULTIPLIER * penalty.powi(2);
            }
            (None, Some(_)) => {
                self.absent_seconds += seconds;
                self.false_track_seconds += seconds;
                self.loss += seconds * penalty.powi(2);
            }
            (None, None) => {
                self.absent_seconds += seconds;
            }
        }
    }
}

/// Scenario likelihood comes from reference truth, never a candidate estimate.
/// Expanding the rectangle by ball radius keeps a straddling ball at full weight.
/// Ground-only labels without a Field transform cannot establish field bounds.
fn reference_distance_outside_field(cycle: &Cycle) -> f64 {
    let point = match &cycle.reference {
        Some(Reference::Field(points)) => points.first().map(|point| point.xy()),
        Some(Reference::Ground(points)) => points
            .first()
            .zip(cycle.ground_to_field)
            .map(|(point, pose)| pose * point.xy()),
        None => None,
    };
    let Some(point) = point else { return 0.0 };
    let dimensions = &cycle.dimensions;
    let outside_x = (point.x().abs() - dimensions.length / 2.0).max(0.0);
    let outside_y = (point.y().abs() - dimensions.width / 2.0).max(0.0);
    f64::from((outside_x.hypot(outside_y) - dimensions.ball_radius).max(0.0))
}

/// Verify replay against live outputs before trying any candidates. Missing input,
/// parameter mismatch and scheduling drift must not silently change the experiment.
pub fn verify(recording: &Recording, parameters: &BallFilterParameters) -> Result<()> {
    let mut tracker = Tracker::default();
    for cycle in &recording.cycles {
        for input in &cycle.inputs {
            tracker.advance(
                input.time,
                input.odometry,
                input.detections.as_deref(),
                input.camera.as_ref(),
                parameters,
                &cycle.dimensions,
            )?;
        }
        let replay = tracker.finish_with_field_pose(
            cycle.time,
            parameters,
            &cycle.dimensions,
            cycle.field_prior_pose,
        );
        let matches = match (replay, cycle.recorded_estimate) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                (a.position - b.position).norm() < 1e-4
                    && (a.velocity - b.velocity).norm() < 1e-4
                    && a.last_seen == b.last_seen
            }
            _ => false,
        };
        ensure!(
            matches,
            "replay differs from live filter at {:?} in {}; verify baseline parameters and capture completeness",
            cycle.time,
            recording.path
        );
    }
    Ok(())
}

pub fn evaluate(
    recordings: &[Recording],
    parameters: &BallFilterParameters,
    penalty: f64,
) -> Result<Score> {
    let mut score = Score::default();
    for recording in recordings {
        // Separate recordings do not establish a continuous observation gap.
        score.current_missing_seconds = 0.0;
        let mut previous_reference = None;
        let mut tracker = Tracker::default();
        for cycle in &recording.cycles {
            if let Some(reference) = &cycle.reference {
                reference.validate_for_optimization()?;
            }
            for input in &cycle.inputs {
                tracker.advance(
                    input.time,
                    input.odometry,
                    input.detections.as_deref(),
                    input.camera.as_ref(),
                    parameters,
                    &cycle.dimensions,
                )?;
            }
            let estimate = tracker.finish_with_field_pose(
                cycle.time,
                parameters,
                &cycle.dimensions,
                cycle.field_prior_pose,
            );
            score.observe_diagnostics(cycle, estimate, &mut previous_reference);
            score.observe_cycle(cycle, estimate, penalty);
        }
    }
    score.loss /= score.weighted_seconds;
    let matched = score.present_seconds - score.missing_seconds;
    score.position_rmse_metres = (matched > 0.0).then(|| (score.squared_error / matched).sqrt());
    let close_matched = score.close_range_present_seconds - score.close_range_missing_seconds;
    score.close_range_position_rmse_metres =
        (close_matched > 0.0).then(|| (score.close_range_squared_error / close_matched).sqrt());
    score.along_motion_error_metres = (score.moving_reference_seconds > 0.0)
        .then(|| score.along_motion_error_integral / score.moving_reference_seconds);
    score.motion_lag_seconds = (score.moving_reference_seconds > 0.0)
        .then(|| score.motion_lag_integral / score.moving_reference_seconds);
    Ok(score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{Vector2, point};
    use ros_z::time::Time;

    fn diagnostic_cycle(time_ms: i64, x: f32, robot_x: f32) -> Cycle {
        Cycle {
            inputs: Vec::new(),
            time: Time::from_nanos(time_ms * 1_000_000),
            dimensions: Default::default(),
            reference: Some(Reference::Field(vec![point![x, 0.0, 0.1]])),
            ground_to_field: Some(Isometry2::from(linear_algebra::vector![robot_x, 0.0])),
            recorded_estimate: None,
            field_prior_pose: None,
            seconds: 0.05,
        }
    }

    #[test]
    fn field_weight_decays_with_whole_ball_distance_and_uses_field_coordinates() {
        let mut cycle = diagnostic_cycle(0, 0.0, 0.0);
        cycle.dimensions = types::field_dimensions::FieldDimensions::SPL_2025;
        let edge = cycle.dimensions.length / 2.0 + cycle.dimensions.ball_radius;
        let mut previous_weight = 1.0;
        for distance in [0.0_f32, 0.3, 1.0, 2.0] {
            cycle.reference = Some(Reference::Field(vec![point![edge + distance, 0.0, 0.1]]));
            let measured = reference_distance_outside_field(&cycle);
            assert!((measured - f64::from(distance)).abs() < 1e-6);
            let weight = (-measured / OUT_OF_FIELD_DECAY_METRES).exp();
            assert!(weight <= previous_weight);
            previous_weight = weight;
        }
        cycle.reference = Some(Reference::Field(vec![point![edge - 0.01, 0.0, 0.1]]));
        assert_eq!(reference_distance_outside_field(&cycle), 0.0);
        cycle.reference = Some(Reference::Field(vec![point![
            cycle.dimensions.length / 2.0 + 0.3,
            cycle.dimensions.width / 2.0 + 0.4,
            0.1
        ]]));
        assert!(
            (reference_distance_outside_field(&cycle)
                - f64::from(0.5 - cycle.dimensions.ball_radius))
            .abs()
                < 1e-6
        );

        // A distant ball in Ground can still be inside the field when the robot
        // is translated and rotated. Geometry must not follow robot coordinates.
        let pose = Isometry2::<Ground, Field>::from_parts(linear_algebra::vector![8.0, -5.0], 1.2);
        for truth in [point![0.0, 0.0], point![edge + 0.3, 0.0]] {
            let ground = pose.inverse() * truth;
            cycle.reference = Some(Reference::Ground(vec![point![ground.x(), ground.y(), 0.1]]));
            cycle.ground_to_field = Some(pose);
            let expected = (truth.x().abs() - edge).max(0.0);
            let actual = reference_distance_outside_field(&cycle);
            // Inverting and reapplying a float32 pose several metres away
            // introduces a few micrometres of roundoff.
            assert!(
                (actual - f64::from(expected)).abs() < 5e-6,
                "distance {actual}, expected {expected}"
            );
        }
        cycle.ground_to_field = None;
        assert_eq!(reference_distance_outside_field(&cycle), 0.0);
    }

    #[test]
    fn outside_weight_changes_only_loss_and_its_denominator_not_raw_metrics_or_gaps() {
        let mut inside = diagnostic_cycle(0, 1.0, 0.0);
        inside.dimensions = types::field_dimensions::FieldDimensions::SPL_2025;
        inside.seconds = 1.0;
        let edge = inside.dimensions.length / 2.0 + inside.dimensions.ball_radius;
        let mut outside = diagnostic_cycle(0, edge + 0.3, 0.0);
        outside.dimensions = inside.dimensions;
        outside.seconds = 1.0;
        let weight =
            (-reference_distance_outside_field(&outside) / OUT_OF_FIELD_DECAY_METRES).exp();
        for error in [None, Some(0.0), Some(0.5)] {
            let estimate = |truth: f32| {
                error.map(|offset| BallPosition::<Ground> {
                    position: point![truth + offset, 0.0],
                    velocity: Vector2::zeros(),
                    last_seen: Time::zero(),
                })
            };
            let mut a = Score::default();
            let mut b = Score::default();
            a.observe_cycle(&inside, estimate(1.0), 2.0);
            b.observe_cycle(&outside, estimate(edge + 0.3), 2.0);
            assert!((b.loss - a.loss * weight).abs() < 1e-6);
            assert!((b.squared_error - a.squared_error).abs() < 1e-6);
            assert_eq!(a.missing_seconds, b.missing_seconds);
            assert_eq!(a.labelled_seconds, b.labelled_seconds);
            assert_eq!(b.out_of_field_seconds, 1.0);
            assert_eq!(b.weighted_seconds, weight);
        }
        let mut gaps = Score::default();
        gaps.observe_cycle(&inside, None, 2.0);
        gaps.observe_cycle(&outside, None, 2.0);
        assert_eq!(gaps.missing_runs, 1);
        assert_eq!(gaps.longest_missing_seconds, 2.0);
        assert_eq!(gaps.missing_seconds, 2.0);

        outside.reference = Some(Reference::Field(Vec::new()));
        let mut absent = Score::default();
        absent.observe_cycle(&outside, None, 2.0);
        assert_eq!(absent.weighted_seconds, 1.0);
        assert_eq!(absent.out_of_field_seconds, 0.0);
        outside.reference = None;
        absent.observe_cycle(&outside, None, 2.0);
        assert_eq!(absent.weighted_seconds, 1.0);
        assert_eq!(absent.unlabelled_seconds, 1.0);
    }

    #[test]
    fn spatial_lag_has_signed_direction_and_ignores_robot_translation() {
        let estimate = BallPosition::<Ground> {
            position: point![0.2, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };
        let mut score = Score::default();
        let mut previous = None;
        score.observe_diagnostics(&diagnostic_cycle(0, 0.5, 0.0), None, &mut previous);
        // Ball moves +0.1 m in 50 ms = 2 m/s; robot itself moved +0.2 m.
        // Estimate world x=.4 trails truth x=.6 by .2 m = .1 s spatial lag.
        score.observe_diagnostics(
            &diagnostic_cycle(50, 0.6, 0.2),
            Some(estimate),
            &mut previous,
        );
        assert!((score.along_motion_error_integral / 0.05 + 0.2).abs() < 1e-6);
        assert!((score.motion_lag_integral / 0.05 - 0.1).abs() < 1e-6);
        assert_eq!(score.moving_reference_seconds, 0.05);
        assert!((score.close_range_squared_error / 0.05 - 0.04).abs() < 1e-6);
        assert_eq!(score.close_range_present_seconds, 0.1);
        assert_eq!(score.close_range_missing_seconds, 0.05);

        let mut ahead = Score::default();
        let mut previous = Some((Time::zero(), point![0.5, 0.0]));
        ahead.observe_diagnostics(
            &diagnostic_cycle(50, 0.6, 0.6),
            Some(estimate),
            &mut previous,
        );
        assert!(ahead.along_motion_error_integral > 0.0);
        assert!(ahead.motion_lag_integral < 0.0);
    }

    #[test]
    fn lag_skips_multiple_balls_stationary_balls_and_reference_gaps() {
        let estimate = BallPosition::<Ground> {
            position: point![0.5, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };
        let mut score = Score::default();
        let mut previous = None;
        let mut cycle = diagnostic_cycle(0, 0.5, 0.0);
        score.observe_diagnostics(&cycle, Some(estimate), &mut previous);
        cycle = diagnostic_cycle(50, 0.5, 0.2);
        score.observe_diagnostics(&cycle, Some(estimate), &mut previous);
        cycle.reference = Some(Reference::Field(vec![
            point![0.6, 0.0, 0.1],
            point![2.0, 0.0, 0.1],
        ]));
        score.observe_diagnostics(&cycle, Some(estimate), &mut previous);
        assert!(previous.is_none());
        score.observe_diagnostics(
            &diagnostic_cycle(100, 0.7, 0.2),
            Some(estimate),
            &mut previous,
        );
        score.observe_diagnostics(
            &diagnostic_cycle(500, 1.0, 0.2),
            Some(estimate),
            &mut previous,
        );
        assert_eq!(score.moving_reference_seconds, 0.0);
    }

    #[test]
    fn dropping_a_difficult_prediction_never_improves_instantaneous_loss() {
        let reference = [point![0.0, 0.0, 0.1]];
        let mut missing = Score::default();
        missing.observe::<Ground>(Some(&reference), None, 1.0, 2.0);
        assert_eq!(missing.loss, 5.0);
        let mut previous_loss = -1.0;
        for error in [0.0, 0.5, 2.0, 5.0, 20.0, 1000.0] {
            let estimate = BallPosition::<Ground> {
                position: point![error, 0.0],
                velocity: Vector2::zeros(),
                last_seen: Time::zero(),
            };
            let mut predicted = Score::default();
            predicted.observe(Some(&reference), Some(estimate), 1.0, 2.0);
            assert!(predicted.loss < missing.loss);
            assert!(predicted.loss < 4.0);
            assert!(predicted.loss > previous_loss);
            assert_eq!(predicted.squared_error, f64::from(error * error));
            previous_loss = predicted.loss;
        }
    }

    #[test]
    fn retaining_a_track_after_the_ball_is_absent_is_penalized() {
        let mut missing = Score::default();
        let mut false_track = Score::default();
        let estimate = BallPosition::<Ground> {
            position: point![0.0, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };
        missing.observe::<Ground>(Some(&[]), None, 1.0, 2.0);
        false_track.observe(Some(&[]), Some(estimate), 1.0, 2.0);
        assert_eq!(missing.loss, 0.0);
        assert_eq!(false_track.loss, 4.0);
        assert_eq!(false_track.missing_runs, 0);
    }

    #[test]
    fn gaps_end_at_estimates_absence_or_unknown_labels() {
        let reference = [point![0.0, 0.0, 0.1]];
        let estimate = BallPosition::<Ground> {
            position: point![0.0, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };
        let mut score = Score::default();
        score.observe::<Ground>(Some(&reference), None, 0.0, 2.0);
        assert_eq!(score.missing_runs, 0);
        score.observe::<Ground>(Some(&reference), None, 0.25, 2.0);
        score.observe::<Ground>(Some(&reference), None, 0.75, 2.0);
        assert_eq!(score.missing_runs, 1);
        assert_eq!(score.longest_missing_seconds, 1.0);
        score.observe(Some(&reference), Some(estimate), 0.5, 2.0);
        score.observe::<Ground>(Some(&reference), None, 0.5, 2.0);
        score.observe::<Ground>(Some(&[]), None, 5.0, 2.0);
        score.observe::<Ground>(Some(&reference), None, 0.75, 2.0);
        score.observe::<Ground>(None, None, 5.0, 2.0);
        score.observe::<Ground>(Some(&reference), None, 1.5, 2.0);
        assert_eq!(score.missing_runs, 4);
        assert_eq!(score.longest_missing_seconds, 1.5);
        assert_eq!(score.missing_seconds, 3.75);
    }

    #[test]
    fn spatial_score_uses_the_single_labelled_target() {
        let balls = [point![2.0, 0.0, 0.1]];
        let mut score = Score::default();
        let mut estimate = BallPosition::<Ground> {
            position: point![2.0, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };
        score.observe(Some(&balls), Some(estimate), 1.0, 2.0);
        assert_eq!(score.loss, 0.0);
        estimate.position = point![0.0, 0.0];
        score.observe(Some(&balls), Some(estimate), 1.0, 2.0);
        assert_eq!(score.loss, 2.0);
    }
    #[test]
    fn field_score_includes_ground_pose_and_never_hides_false_tracks_when_pose_is_missing() {
        let ball = BallPosition::<Ground> {
            position: point![1.0, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };
        let pose = Isometry2::<Ground, Field>::from(linear_algebra::vector![2.0, 1.0]);
        let mut score = Score::default();
        score.observe_field(&[point![3.0, 1.0, 0.105]], Some(ball), Some(pose), 1.0, 2.0);
        assert!(score.loss < 1e-9);
        score.observe_field(&[point![3.0, 1.0, 0.105]], Some(ball), None, 2.0, 2.0);
        score.observe_field(&[], Some(ball), None, 3.0, 2.0);
        assert_eq!(score.loss, 22.0);
        assert_eq!(score.missing_seconds, 2.0);
        assert_eq!(score.false_track_seconds, 3.0);
        assert_eq!(score.missing_transform_seconds, 5.0);
    }

    #[test]
    fn missing_and_false_tracks_are_penalized_unknown_is_not_absent() {
        let mut score = Score::default();
        let ball: BallPosition<Ground> = BallPosition {
            position: point![1.0, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };
        score.observe(None, Some(ball), 2.0, 2.0);
        score.observe::<Ground>(Some(&[point![1.0, 0.0, 0.105]]), None, 3.0, 2.0);
        score.observe(Some(&[]), Some(ball), 4.0, 2.0);
        assert_eq!(score.loss, 31.0);
        assert_eq!(score.labelled_seconds, 7.0);
        assert_eq!(score.unlabelled_seconds, 2.0);
        assert_eq!(score.missing_seconds, 3.0);
        assert_eq!(score.false_track_seconds, 4.0);
    }
}

impl From<&Score> for types::ball_filter_tuning::Metrics {
    fn from(score: &Score) -> Self {
        Self {
            loss: score.loss,
            position_rmse_metres: score.position_rmse_metres,
            missing_seconds: score.missing_seconds,
            missing_runs: score.missing_runs,
            longest_missing_seconds: score.longest_missing_seconds,
            close_range_position_rmse_metres: score.close_range_position_rmse_metres,
            close_range_present_seconds: score.close_range_present_seconds,
            close_range_missing_seconds: score.close_range_missing_seconds,
            along_motion_error_metres: score.along_motion_error_metres,
            motion_lag_seconds: score.motion_lag_seconds,
            moving_reference_seconds: score.moving_reference_seconds,
            false_track_seconds: score.false_track_seconds,
            missing_transform_seconds: score.missing_transform_seconds,
        }
    }
}

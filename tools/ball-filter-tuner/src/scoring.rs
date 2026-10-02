use crate::recording::{Recording, Reference};
use ball_filter::tracker::Tracker;
use color_eyre::{Result, eyre::ensure};
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point3};
use serde::Serialize;
use types::{ball_position::BallPosition, parameters::BallFilterParameters};

#[derive(Default, Debug, Serialize)]
pub struct Score {
    pub loss: f64,
    pub position_rmse_metres: Option<f64>,
    pub labelled_seconds: f64,
    pub unlabelled_seconds: f64,
    pub present_seconds: f64,
    pub missing_seconds: f64,
    pub absent_seconds: f64,
    pub false_track_seconds: f64,
    pub missing_transform_seconds: f64,
    #[serde(skip)]
    squared_error: f64,
}

impl Score {
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
            return;
        };
        self.labelled_seconds += seconds;
        match (reference.first(), estimate) {
            (Some(_), Some(estimate)) => {
                self.present_seconds += seconds;
                // The production output is one selected ball. Any real ball is
                // a valid target; vector ordering must not penalize switching
                // between real balls or accidentally reward a point between them.
                let squared = reference
                    .iter()
                    .map(|truth| f64::from((truth.xy() - estimate.position).norm_squared()))
                    .fold(f64::INFINITY, f64::min);
                self.squared_error += seconds * squared;
                self.loss += seconds * squared;
            }
            (Some(_), None) => {
                self.present_seconds += seconds;
                self.missing_seconds += seconds;
                self.loss += seconds * penalty.powi(2);
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
        let replay = tracker.finish(cycle.time, parameters, &cycle.dimensions);
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
            let estimate = tracker.finish(cycle.time, parameters, &cycle.dimensions);
            match &cycle.reference {
                Some(Reference::Ground(reference)) => {
                    score.observe(Some(reference), estimate, cycle.seconds, penalty)
                }
                Some(Reference::Field(reference)) => score.observe_field(
                    reference,
                    estimate,
                    cycle.ground_to_field,
                    cycle.seconds,
                    penalty,
                ),
                None => score.observe::<Ground>(None, estimate, cycle.seconds, penalty),
            }
        }
    }
    score.loss /= score.labelled_seconds;
    let matched = score.present_seconds - score.missing_seconds;
    score.position_rmse_metres = (matched > 0.0).then(|| (score.squared_error / matched).sqrt());
    Ok(score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{Vector2, point};
    use ros_z::time::Time;
    #[test]
    fn multiple_balls_score_against_a_real_ball_not_the_first_or_centroid() {
        let balls = [point![-2.0, 0.0, 0.1], point![2.0, 0.0, 0.1]];
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
        assert_eq!(score.loss, 4.0);
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
        assert_eq!(score.loss, 20.0);
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
        assert_eq!(score.loss, 28.0);
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
            false_track_seconds: score.false_track_seconds,
            missing_transform_seconds: score.missing_transform_seconds,
        }
    }
}

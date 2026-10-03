//! Field-boundary confidence prior and optional elapsed-time validity decay.
use std::{collections::BTreeMap, time::Duration};

use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use ros_z::time::Time;
use types::{field_dimensions::FieldDimensions, parameters::BallFilterParameters};

use crate::BallHypothesis;

const POSE_HISTORY_CAPACITY: usize = 512;
const MAXIMUM_POSE_TIME_DIFFERENCE: Duration = Duration::from_millis(20);

#[derive(Default)]
pub(crate) struct FieldPoseHistory {
    poses: BTreeMap<Time, Isometry2<Ground, Field>>,
}

impl FieldPoseHistory {
    pub fn insert(&mut self, time: Time, pose: Isometry2<Ground, Field>) {
        self.poses.insert(time, pose);
        while self.poses.len() > POSE_HISTORY_CAPACITY {
            self.poses.pop_first();
        }
    }

    pub fn at(&self, time: Time) -> Option<Isometry2<Ground, Field>> {
        let before = self.poses.range(..=time).next_back();
        let after = self.poses.range(time..).next();
        [before, after]
            .into_iter()
            .flatten()
            .min_by_key(|(stamp, _)| stamp.as_nanos().abs_diff(time.as_nanos()))
            .filter(|(stamp, _)| {
                u128::from(stamp.as_nanos().abs_diff(time.as_nanos()))
                    <= MAXIMUM_POSE_TIME_DIFFERENCE.as_nanos()
            })
            .map(|(_, pose)| *pose)
    }
}

// A long processing/pose gap is not evidence that the ball stayed outside the
// field throughout that interval. Ordinary 500 Hz fusion and 25 Hz replay are
// integrated in seconds rather than charged once per cycle.
const MAXIMUM_VALIDITY_DECAY_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Default)]
pub(crate) struct ValidityDecayClock {
    previous: Option<(Time, bool)>,
}

impl ValidityDecayClock {
    pub fn reset(&mut self) {
        self.previous = None;
    }

    pub fn elapsed(&mut self, time: Time, valid_pose: bool) -> Duration {
        let elapsed = match self.previous {
            Some((previous, _)) if time <= previous => return Duration::ZERO,
            Some((previous, true)) if valid_pose => {
                let elapsed = time.duration_since(previous);
                if elapsed <= MAXIMUM_VALIDITY_DECAY_INTERVAL {
                    elapsed
                } else {
                    Duration::ZERO
                }
            }
            _ => Duration::ZERO,
        };
        self.previous = Some((time, valid_pose));
        elapsed
    }
}

pub(crate) fn decay_stored_validity(
    hypotheses: &mut [BallHypothesis],
    elapsed: Duration,
    ground_to_field: Option<Isometry2<Ground, Field>>,
    dimensions: &FieldDimensions,
    parameters: &BallFilterParameters,
) {
    let maximum_rate = parameters.field_boundary_validity_decay_rate;
    if !parameters.good_localization
        || elapsed.is_zero()
        || !maximum_rate.is_finite()
        || maximum_rate <= 0.0
    {
        return;
    }
    for hypothesis in hypotheses {
        let weight = confidence_weight(hypothesis, ground_to_field, dimensions, parameters);
        let rate = maximum_rate * (1.0 - weight.clamp(0.0, 1.0));
        hypothesis.validity *= (-rate * elapsed.as_secs_f32()).exp();
    }
}

pub(crate) fn effective_validity(
    hypothesis: &BallHypothesis,
    ground_to_field: Option<Isometry2<Ground, Field>>,
    dimensions: &FieldDimensions,
    parameters: &BallFilterParameters,
) -> f32 {
    hypothesis.validity * confidence_weight(hypothesis, ground_to_field, dimensions, parameters)
}

pub(crate) fn confidence_weight(
    hypothesis: &BallHypothesis,
    ground_to_field: Option<Isometry2<Ground, Field>>,
    dimensions: &FieldDimensions,
    parameters: &BallFilterParameters,
) -> f32 {
    if !parameters.good_localization {
        return 1.0;
    }
    let decay_distance = parameters.field_boundary_confidence_decay_distance;
    let Some(ground_to_field) =
        ground_to_field.filter(|_| decay_distance.is_finite() && decay_distance > 0.0)
    else {
        return 1.0;
    };
    let position = ground_to_field * hypothesis.position().position;
    if !position.x().is_finite() || !position.y().is_finite() {
        return 1.0;
    }
    // A ball is wholly out only once its nearest edge has crossed the field
    // rectangle. Border strips are outside the playing field, too.
    let dx = (position.x().abs() - dimensions.length / 2.0).max(0.0);
    let dy = (position.y().abs() - dimensions.width / 2.0).max(0.0);
    let whole_ball_distance = (dx.hypot(dy) - dimensions.ball_radius).max(0.0);
    // Allow localization/radius uncertainty beyond the existing whole-ball
    // clearance. The margin delays both ranking and stored-validity penalties;
    // their exponential shape remains unchanged outside that buffer.
    let margin = parameters.field_boundary_margin;
    let margin = if margin.is_finite() && margin > 0.0 {
        margin
    } else {
        0.0
    };
    let penalized_distance = (whole_ball_distance - margin).max(0.0);
    (-penalized_distance / decay_distance).exp()
}

#[cfg(test)]
mod tests {
    use linear_algebra::{point, vector};
    use nalgebra::Matrix4;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    use crate::{BallFilter, BallMode, tracker::Tracker};

    use super::*;

    fn parameters() -> BallFilterParameters {
        let mut parameters = BallFilterParameters::default();
        parameters.field_boundary_confidence_decay_distance = 0.3;
        parameters.validity_output_threshold = 0.5;
        parameters.hypothesis_timeout = Duration::from_secs(20);
        parameters.maximum_number_of_hypotheses = 15;
        parameters
    }

    fn hypothesis(x: f32, y: f32, validity: f32) -> BallHypothesis {
        BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::vector![x, y, 0.3, -0.2],
                covariance: Matrix4::identity(),
            }),
            last_seen: Time::zero(),
            validity,
            motion_evidence: None,
            negative_evidence: None,
            validity_decay_evidence: None,
            leadership_evidence: None,
            merge_observation_start: None,
        }
    }

    fn run_stored_decay(x: f32, step_millis: i64) -> f32 {
        let mut parameters = parameters();
        parameters.field_boundary_validity_decay_rate = 2.0;
        let dimensions = FieldDimensions::SPL_2025;
        let mut tracker = Tracker::default();
        tracker.filter.hypotheses.push(hypothesis(x, 0.0, 25.0));
        for millis in (0..=1000).step_by(step_millis as usize) {
            tracker.finish_with_field_pose(
                Time::from_nanos(millis * 1_000_000),
                &parameters,
                &dimensions,
                Some(Isometry2::identity()),
            );
        }
        tracker.filter.hypotheses[0].validity
    }

    #[test]
    fn untrusted_localization_preserves_candidate_confidence_and_output_selection() {
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = parameters();
        assert!(
            parameters.good_localization,
            "preserve existing default behavior"
        );
        parameters.good_localization = false;
        parameters.field_boundary_validity_decay_rate = 2.0;
        parameters.validity_discard_threshold = 1.0;
        let outside = hypothesis(dimensions.length / 2.0 + 1.5, 0.0, 25.0);
        let original = outside.position();
        let mut tracker = Tracker::default();
        tracker.filter.hypotheses = vec![outside, hypothesis(1.0, 0.0, 4.0)];
        for millis in (0..=2000).step_by(40) {
            let pose = if millis < 1000 {
                Some(Isometry2::identity())
            } else {
                Some(Isometry2::from_parts(vector![9.0, -5.0], 1.0))
            };
            let selected = tracker
                .finish_with_field_pose(
                    Time::from_nanos(millis * 1_000_000),
                    &parameters,
                    &dimensions,
                    pose,
                )
                .unwrap();
            assert_eq!(tracker.filter.hypotheses.len(), 2);
            assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
            assert_eq!(tracker.filter.hypotheses[1].validity, 4.0);
            assert_eq!(selected.position, original.position);
            assert_eq!(selected.velocity, original.velocity);
            assert_eq!(selected.last_seen, original.last_seen);
            assert!(crate::hypothetical_ball_positions(
                &tracker.filter, &parameters, &dimensions, pose,
            ).is_empty());
        }
        parameters.good_localization = true;
        let selected = tracker
            .finish_with_field_pose(
                Time::from_nanos(2_040_000_000),
                &parameters,
                &dimensions,
                Some(Isometry2::identity()),
            )
            .unwrap();
        assert_eq!(selected.position, point![1.0, 0.0]);
        assert_eq!(
            tracker.filter.hypotheses[0].validity, 25.0,
            "the first trusted pose must not charge disabled time"
        );
        assert_eq!(
            crate::hypothetical_ball_positions(
                &tracker.filter,
                &parameters,
                &dimensions,
                Some(Isometry2::identity()),
            )
            .len(),
            1
        );
    }

    #[test]
    fn disabling_localization_resets_decay_even_at_duplicate_or_older_finish_times() {
        let dimensions = FieldDimensions::SPL_2025;
        let pose = Some(Isometry2::identity());
        for disabled_millis in [20, 40, 60] {
            let mut parameters = parameters();
            parameters.field_boundary_validity_decay_rate = 2.0;
            let mut tracker = Tracker::default();
            tracker
                .filter
                .hypotheses
                .push(hypothesis(dimensions.length / 2.0 + 0.6, 0.0, 25.0));
            for millis in [0, 40] {
                tracker.finish_with_field_pose(
                    Time::from_nanos(millis * 1_000_000),
                    &parameters,
                    &dimensions,
                    pose,
                );
            }
            let before = tracker.filter.hypotheses[0].validity;
            assert!(before < 25.0);
            parameters.good_localization = false;
            tracker.finish_with_field_pose(
                Time::from_nanos(disabled_millis * 1_000_000),
                &parameters,
                &dimensions,
                pose,
            );
            assert_eq!(tracker.filter.hypotheses[0].validity, before);
            parameters.good_localization = true;
            tracker.finish_with_field_pose(
                Time::from_nanos(80_000_000),
                &parameters,
                &dimensions,
                pose,
            );
            assert_eq!(tracker.filter.hypotheses[0].validity, before);
            tracker.finish_with_field_pose(
                Time::from_nanos(120_000_000),
                &parameters,
                &dimensions,
                pose,
            );
            assert!(tracker.filter.hypotheses[0].validity < before);
        }
    }

    #[test]
    fn direct_field_decay_and_ranking_are_neutral_when_localization_is_untrusted() {
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = parameters();
        parameters.good_localization = false;
        parameters.field_boundary_margin = 0.5;
        parameters.field_boundary_validity_decay_rate = 2.0;
        let mut balls = vec![hypothesis(1.0, 0.0, 2.0)];
        let pose = Some(Isometry2::from_parts(vector![100.0, 0.0], 0.0));
        decay_stored_validity(
            &mut balls,
            Duration::from_secs(5),
            pose,
            &dimensions,
            &parameters,
        );
        assert_eq!(balls[0].validity, 2.0);
        assert_eq!(
            effective_validity(&balls[0], pose, &dimensions, &parameters),
            2.0
        );
    }

    #[test]
    fn stored_field_decay_integrates_seconds_independently_of_finish_frequency() {
        let dimensions = FieldDimensions::SPL_2025;
        let x = dimensions.length / 2.0
            + dimensions.ball_radius
            + parameters().field_boundary_confidence_decay_distance;
        let expected = 25.0 * (-2.0_f32 * (1.0 - (-1.0_f32).exp())).exp();
        for step in [2, 20, 40, 100] {
            let validity = run_stored_decay(x, step);
            assert!(
                (validity - expected).abs() < 0.001,
                "{step} ms: {validity} vs {expected}"
            );
        }
    }

    #[test]
    fn stored_decay_is_neutral_inside_and_stronger_outside_with_a_bounded_rate() {
        let dimensions = FieldDimensions::SPL_2025;
        let edge = dimensions.length / 2.0 + dimensions.ball_radius;
        assert_eq!(run_stored_decay(0.0, 20), 25.0);
        assert!((run_stored_decay(edge, 20) - 25.0).abs() < 0.001);
        let near = run_stored_decay(edge + 0.1, 20);
        let farther = run_stored_decay(edge + 0.6, 20);
        assert!(farther < near && near < 25.0);
        assert!(
            farther >= 25.0 * (-2.0_f32).exp(),
            "maximum rate is two per second"
        );
    }

    #[test]
    fn boundary_margin_protects_nearby_balls_and_preserves_decay_beyond_it() {
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = parameters();
        parameters.field_boundary_margin = 0.5;
        parameters.field_boundary_validity_decay_rate = 2.0;
        let edge = dimensions.length / 2.0 + dimensions.ball_radius;
        let pose = Some(Isometry2::identity());
        for distance in [0.0, 0.1, 0.3, 0.49] {
            let mut balls = [hypothesis(edge + distance, 0.0, 25.0)];
            assert_eq!(
                confidence_weight(&balls[0], pose, &dimensions, &parameters),
                1.0
            );
            decay_stored_validity(
                &mut balls,
                Duration::from_secs(1),
                pose,
                &dimensions,
                &parameters,
            );
            assert_eq!(balls[0].validity, 25.0);
        }
        let buffered_edge = hypothesis(edge + 0.5, 0.0, 25.0);
        assert!(
            (confidence_weight(&buffered_edge, pose, &dimensions, &parameters) - 1.0).abs() < 1e-5
        );
        for distance in [0.1_f32, 0.3, 1.0] {
            let mut balls = [hypothesis(edge + 0.5 + distance, 0.0, 25.0)];
            let expected_weight = (-distance / 0.3).exp();
            assert!(
                (confidence_weight(&balls[0], pose, &dimensions, &parameters) - expected_weight)
                    .abs()
                    < 1e-5
            );
            decay_stored_validity(
                &mut balls,
                Duration::from_secs(1),
                pose,
                &dimensions,
                &parameters,
            );
            let expected_validity = 25.0 * (-2.0 * (1.0 - expected_weight)).exp();
            assert!((balls[0].validity - expected_validity).abs() < 1e-4);
        }
    }

    #[test]
    fn boundary_margin_handles_corner_distance_and_transformed_ground_positions() {
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = parameters();
        parameters.field_boundary_margin = 0.5;
        let pose = Isometry2::from_parts(vector![2.0, -1.0], 1.4);
        let corner = point![dimensions.length / 2.0, dimensions.width / 2.0];
        for clearance in [0.4_f32, 0.8] {
            let offset = (dimensions.ball_radius + clearance) / 2.0_f32.sqrt();
            let ground_position = pose.inverse() * (corner + vector![offset, offset]);
            let ball = hypothesis(ground_position.x(), ground_position.y(), 25.0);
            let expected = (-(clearance - 0.5).max(0.0) / 0.3).exp();
            assert!(
                (confidence_weight(&ball, Some(pose), &dimensions, &parameters) - expected).abs()
                    < 1e-5
            );
        }
    }

    #[test]
    fn localization_wobble_near_the_sideline_does_not_decay_a_track() {
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = parameters();
        parameters.field_boundary_margin = 0.5;
        parameters.field_boundary_validity_decay_rate = 2.0;
        let mut tracker = Tracker::default();
        tracker.filter.hypotheses.push(hypothesis(
            dimensions.length / 2.0 + dimensions.ball_radius,
            0.0,
            25.0,
        ));
        for tick in 0..=50 {
            let offset = if tick % 2 == 0 { -0.1 } else { 0.1 };
            assert!(
                tracker
                    .finish_with_field_pose(
                        Time::from_nanos(tick * 20_000_000),
                        &parameters,
                        &dimensions,
                        Some(Isometry2::from_parts(vector![offset, 0.0], 0.0)),
                    )
                    .is_some()
            );
        }
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
    }

    #[test]
    fn boundary_buffer_adds_no_decay_for_hidden_or_visible_hypotheses() {
        use crate::{negative_evidence::Visibility, validity_decay::Evidence};

        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = parameters();
        parameters.field_boundary_margin = 0.5;
        parameters.field_boundary_validity_decay_rate = 2.0;
        parameters.hidden_validity_decay_rate = Some(0.0);
        parameters.visible_missed_validity_decay_rate = Some(1.0);
        for visibility in [Visibility::Hidden, Visibility::Visible] {
            let mut tracker = Tracker::default();
            let mut ball = hypothesis(
                dimensions.length / 2.0 + dimensions.ball_radius + 0.3,
                0.0,
                25.0,
            );
            ball.validity_decay_evidence = Some(Evidence {
                time: Time::zero(),
                visibility,
            });
            tracker.filter.hypotheses.push(ball);
            // Finishes between detector exposures may apply field decay, but
            // cannot treat visibility or an occluded opponent as a new miss.
            for tick in 0..=50 {
                assert!(
                    tracker
                        .finish_with_field_pose(
                            Time::from_nanos(tick * 20_000_000),
                            &parameters,
                            &dimensions,
                            Some(Isometry2::identity()),
                        )
                        .is_some()
                );
            }
            assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        }
    }

    #[test]
    fn default_or_invalid_boundary_margin_preserves_legacy_geometry() {
        let mut parameters = parameters();
        assert_eq!(parameters.field_boundary_margin, 0.0);
        let dimensions = FieldDimensions::SPL_2025;
        let ball = hypothesis(
            dimensions.length / 2.0 + dimensions.ball_radius + 0.3,
            0.0,
            25.0,
        );
        for margin in [0.0, -0.5, f32::NAN, f32::INFINITY] {
            parameters.field_boundary_margin = margin;
            assert!(
                (confidence_weight(&ball, Some(Isometry2::identity()), &dimensions, &parameters)
                    - (-1.0_f32).exp())
                .abs()
                    < 1e-5
            );
        }
    }

    #[test]
    fn missing_pose_sensor_gaps_and_nonmonotonic_finishes_do_not_charge_unknown_time() {
        let mut parameters = parameters();
        parameters.field_boundary_validity_decay_rate = 2.0;
        let dimensions = FieldDimensions::SPL_2025;
        let mut tracker = Tracker::default();
        tracker
            .filter
            .hypotheses
            .push(hypothesis(dimensions.length / 2.0 + 0.6, 0.0, 25.0));
        for (millis, pose) in [
            (0, Some(Isometry2::identity())),
            (40, None),
            (80, Some(Isometry2::identity())),
            (80, Some(Isometry2::identity())),
            (60, Some(Isometry2::identity())),
            (10_000, Some(Isometry2::identity())),
        ] {
            tracker.finish_with_field_pose(
                Time::from_nanos(millis * 1_000_000),
                &parameters,
                &dimensions,
                pose,
            );
            assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        }
        tracker.finish_with_field_pose(
            Time::from_nanos(10_040_000_000),
            &parameters,
            &dimensions,
            Some(Isometry2::identity()),
        );
        let decayed = tracker.filter.hypotheses[0].validity;
        assert!(decayed < 25.0 && decayed > 23.0);
        tracker.finish_with_field_pose(
            Time::from_nanos(10_040_000_000),
            &parameters,
            &dimensions,
            Some(Isometry2::identity()),
        );
        assert_eq!(tracker.filter.hypotheses[0].validity, decayed);
    }

    #[test]
    fn whole_ball_prior_is_neutral_at_line_and_decays_monotonically_outside() {
        let dimensions = FieldDimensions::SPL_2025;
        let parameters = parameters();
        let confidence = |x, y| {
            effective_validity(
                &hypothesis(x, y, 2.0),
                Some(Isometry2::identity()),
                &dimensions,
                &parameters,
            )
        };
        let edge = dimensions.length / 2.0;
        assert_eq!(confidence(0.0, 0.0), 2.0);
        assert_eq!(confidence(edge, 0.0), 2.0);
        assert!((confidence(edge + dimensions.ball_radius, 0.0) - 2.0).abs() < 1e-5);
        let near = confidence(
            edge + dimensions.ball_radius + parameters.field_boundary_confidence_decay_distance,
            0.0,
        );
        let far = confidence(edge + dimensions.ball_radius + 0.6, 0.0);
        assert!((near - 2.0 / std::f32::consts::E).abs() < 1e-5);
        assert!(0.0 < far && far < near && near < 2.0);
        assert_eq!(confidence(-edge - 0.5, 0.0), confidence(edge + 0.5, 0.0));
        let corner = confidence(edge + 0.3, dimensions.width / 2.0 + 0.3);
        assert!(corner < confidence(edge + 0.3, 0.0));
    }

    #[test]
    fn field_prior_accounts_for_robot_translation_and_rotation() {
        let dimensions = FieldDimensions::SPL_2025;
        let parameters = parameters();
        let field_position = point![
            dimensions.length / 2.0
                + dimensions.ball_radius
                + parameters.field_boundary_confidence_decay_distance,
            0.4
        ];
        for pose in [
            Isometry2::<Ground, Field>::identity(),
            Isometry2::from_parts(vector![2.0, -1.0], 1.4),
            Isometry2::from_parts(vector![-3.0, 2.0], -2.7),
        ] {
            let ground_position = pose.inverse() * field_position;
            let ball = hypothesis(ground_position.x(), ground_position.y(), 2.0);
            let confidence = effective_validity(&ball, Some(pose), &dimensions, &parameters);
            assert!((confidence - 2.0 / std::f32::consts::E).abs() < 1e-5);
        }
    }

    #[test]
    fn pose_history_uses_bounded_source_time_matching_with_missing_fallback() {
        let mut history = FieldPoseHistory::default();
        let query = Time::from_nanos(100_000_000);
        assert!(history.at(query).is_none());
        history.insert(Time::from_nanos(79_999_999), Isometry2::identity());
        assert!(history.at(query).is_none());
        history.insert(Time::from_nanos(120_000_000), Isometry2::identity());
        assert!(history.at(query).is_some());
        let older = Isometry2::from_parts(vector![1.0, 0.0], 0.0);
        history.insert(Time::from_nanos(80_000_000), older);
        assert_eq!(history.at(query), Some(older)); // earlier wins a symmetric tie
        for tick in 0..600 {
            history.insert(
                Time::from_nanos(1_000_000_000 + tick * 2_000_000),
                Isometry2::identity(),
            );
        }
        assert_eq!(history.poses.len(), POSE_HISTORY_CAPACITY);
        assert!(history.at(query).is_none());
        assert_eq!(
            effective_validity(
                &hypothesis(6.0, 0.0, 2.0),
                history.at(query),
                &FieldDimensions::SPL_2025,
                &parameters()
            ),
            2.0,
        );
    }

    #[test]
    fn localization_jump_can_recover_without_destroying_or_decaying_the_track() {
        let parameters = parameters();
        let dimensions = FieldDimensions::SPL_2025;
        let mut tracker = Tracker::default();
        tracker.filter.hypotheses.push(hypothesis(1.0, 0.0, 2.0));
        let original = tracker.filter.hypotheses[0].position();
        let bad_pose = Some(Isometry2::from_parts(vector![9.0, 0.0], 0.0));
        for tick in 0..100 {
            assert!(
                tracker
                    .finish_with_field_pose(
                        Time::from_nanos(tick * 2_000_000),
                        &parameters,
                        &dimensions,
                        bad_pose,
                    )
                    .is_none()
            );
        }
        assert_eq!(tracker.filter.hypotheses.len(), 1);
        assert_eq!(tracker.filter.hypotheses[0].validity, 2.0);
        let recovered = tracker
            .finish_with_field_pose(
                Time::from_nanos(200_000_000),
                &parameters,
                &dimensions,
                Some(Isometry2::identity()),
            )
            .unwrap();
        assert_eq!(recovered.position, original.position);
        assert_eq!(recovered.velocity, original.velocity);
        assert_eq!(recovered.last_seen, original.last_seen);
        assert!(
            tracker
                .finish_with_field_pose(
                    Time::from_nanos(200_000_000),
                    &parameters,
                    &dimensions,
                    None,
                )
                .is_some()
        );
    }

    #[test]
    fn field_prior_is_unchanged_until_a_confirmed_track_can_replace_a_stale_one() {
        let dimensions = FieldDimensions::SPL_2025;
        let parameters = parameters();
        let outside = hypothesis(
            dimensions.length / 2.0 + dimensions.ball_radius + 0.6,
            0.0,
            1000.0,
        );
        let inside = hypothesis(1.0, 0.0, 4.0);
        let mut filter = BallFilter {
            hypotheses: vec![outside, inside],
        };
        let pose = Some(Isometry2::identity());
        assert!(effective_validity(&filter.hypotheses[0], pose, &dimensions, &parameters) > 4.0);
        // The prior remains soft: strong recent evidence outside the field can
        // still outweigh a weaker inside track, exactly as before recovery.
        assert!(
            filter
                .best_hypothesis_with_field_pose(&parameters, &dimensions, pose)
                .unwrap()
                .position()
                .position
                .x()
                > dimensions.length / 2.0
        );
        filter.hypotheses[1].last_seen = Time::from_nanos(40_000_000);
        assert!(
            filter
                .best_hypothesis_with_field_pose(&parameters, &dimensions, pose)
                .unwrap()
                .position()
                .position
                .x()
                > dimensions.length / 2.0
        );
        // Once stale, accumulated outside confidence cannot neutralize the
        // prior in the recovery comparison against a confirmed fresh track.
        filter.hypotheses[1].last_seen = Time::from_nanos(500_000_000);
        filter.hypotheses[1].validity = 2.0;
        assert!(
            filter
                .best_hypothesis_with_field_pose(&parameters, &dimensions, pose)
                .unwrap()
                .position()
                .position
                .x()
                > dimensions.length / 2.0
        );
        filter.hypotheses[1].validity = 4.0;
        let chosen = filter
            .best_hypothesis_with_field_pose(&parameters, &dimensions, pose)
            .unwrap();
        assert_eq!(chosen.position().position, point![1.0, 0.0]);
        assert_eq!(filter.hypotheses[0].validity, 1000.0);
        assert_eq!(
            filter
                .best_hypothesis(parameters.validity_output_threshold)
                .unwrap()
                .position()
                .position,
            filter
                .best_hypothesis_with_field_pose(&parameters, &dimensions, None)
                .unwrap()
                .position()
                .position
        );
    }

    #[test]
    fn primary_output_and_hypothetical_output_share_the_same_confidence() {
        let dimensions = FieldDimensions::SPL_2025;
        let parameters = parameters();
        let outside = hypothesis(
            dimensions.length / 2.0 + dimensions.ball_radius + 0.9,
            0.0,
            4.0,
        );
        let inside = hypothesis(1.0, 0.0, 1.0);
        let mut tracker = Tracker::default();
        tracker.filter = BallFilter {
            hypotheses: vec![outside, inside],
        };
        let pose = Some(Isometry2::identity());
        let chosen = tracker
            .finish_with_field_pose(Time::zero(), &parameters, &dimensions, pose)
            .unwrap();
        let best = tracker
            .filter
            .best_hypothesis_with_field_pose(&parameters, &dimensions, pose)
            .unwrap();
        assert_eq!(chosen.position, point![1.0, 0.0]);
        assert_eq!(chosen.position, best.position().position);
        let hypothetical =
            crate::hypothetical_ball_positions(&tracker.filter, &parameters, &dimensions, pose);
        assert_eq!(hypothetical.len(), 1);
        assert!(hypothetical[0].position.x() > dimensions.length / 2.0);
        assert!(hypothetical[0].validity < parameters.validity_output_threshold);
        assert_eq!(tracker.filter.hypotheses[0].validity, 4.0);
        let without_pose = tracker
            .finish(Time::zero(), &parameters, &dimensions)
            .unwrap();
        assert_eq!(without_pose.position, hypothetical[0].position);
    }
}

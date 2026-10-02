//! Optional field-boundary confidence prior. Stored track validity is untouched.
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

pub(crate) fn effective_validity(
    hypothesis: &BallHypothesis,
    ground_to_field: Option<Isometry2<Ground, Field>>,
    dimensions: &FieldDimensions,
    parameters: &BallFilterParameters,
) -> f32 {
    let decay_distance = parameters.field_boundary_confidence_decay_distance;
    let Some(ground_to_field) =
        ground_to_field.filter(|_| decay_distance.is_finite() && decay_distance > 0.0)
    else {
        return hypothesis.validity;
    };
    let position = ground_to_field * hypothesis.position().position;
    if !position.x().is_finite() || !position.y().is_finite() {
        return hypothesis.validity;
    }
    // A ball is wholly out only once its nearest edge has crossed the field
    // rectangle. Border strips are outside the playing field, too.
    let dx = (position.x().abs() - dimensions.length / 2.0).max(0.0);
    let dy = (position.y().abs() - dimensions.width / 2.0).max(0.0);
    let whole_ball_distance = (dx.hypot(dy) - dimensions.ball_radius).max(0.0);
    hypothesis.validity * (-whole_ball_distance / decay_distance).exp()
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
        }
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
        let near = confidence(edge + dimensions.ball_radius + 0.3, 0.0);
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
        let field_position = point![dimensions.length / 2.0 + dimensions.ball_radius + 0.3, 0.4];
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

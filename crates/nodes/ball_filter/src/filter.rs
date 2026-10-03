use std::time::Duration;

use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use nalgebra::{Matrix2, Matrix4};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    parameters::BallFilterParameters,
};

use crate::hypothesis::{BallHypothesis, BallMode};

// Allow short perception interruptions before a confirmed competing track can
// replace the most trusted track. At 25 Hz this covers three image intervals;
// this is a switching-delay budget, not the lifetime of a hidden hypothesis.
const STALE_TRACK_RECOVERY_GRACE: Duration = Duration::from_millis(120);

#[derive(Debug, Default, Clone, Serialize, Deserialize, Message)]
pub struct BallFilter {
    pub hypotheses: Vec<BallHypothesis>,
}

impl BallFilter {
    pub fn best_hypothesis(&self, validity_threshold: f32) -> Option<&BallHypothesis> {
        self.select_hypothesis(validity_threshold, |_| 1.0)
    }

    pub fn best_hypothesis_with_field_pose(
        &self,
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
        ground_to_field: Option<Isometry2<Ground, Field>>,
    ) -> Option<&BallHypothesis> {
        self.select_hypothesis(parameters.validity_output_threshold, |hypothesis| {
            crate::field_prior::confidence_weight(
                hypothesis,
                ground_to_field,
                dimensions,
                parameters,
            )
        })
    }

    fn select_hypothesis(
        &self,
        validity_threshold: f32,
        confidence_weight: impl Fn(&BallHypothesis) -> f32,
    ) -> Option<&BallHypothesis> {
        let confirmation_confidence = 3.0_f32.max(validity_threshold);
        let candidates = self.hypotheses.iter().filter_map(|hypothesis| {
            let weight = confidence_weight(hypothesis);
            let effective_validity = hypothesis.validity * weight;
            (effective_validity >= validity_threshold).then_some((
                hypothesis,
                // Cap before applying the field prior only when comparing
                // possible replacements for a stale, established track.
                hypothesis.validity.min(confirmation_confidence) * weight,
                effective_validity,
            ))
        });
        // Preserve accumulated confidence and the existing soft field prior in
        // normal competition. Global capping would let a confirmed false track
        // steal selection after a single missed image.
        let (incumbent, incumbent_recovery_rank, _) = candidates
            .clone()
            .max_by(|(_, _, validity_a), (_, _, validity_b)| validity_a.total_cmp(validity_b))?;
        let recovered = candidates
            .filter(|(candidate, recovery_rank, _)| {
                candidate.validity >= confirmation_confidence
                    && candidate.last_seen > incumbent.last_seen
                    && candidate.last_seen.duration_since(incumbent.last_seen)
                        > STALE_TRACK_RECOVERY_GRACE
                    && *recovery_rank >= incumbent_recovery_rank
            })
            .max_by(|(a, rank_a, validity_a), (b, rank_b, validity_b)| {
                rank_a
                    .total_cmp(rank_b)
                    .then_with(|| a.last_seen.cmp(&b.last_seen))
                    .then_with(|| validity_a.total_cmp(validity_b))
            })
            .map(|(hypothesis, _, _)| hypothesis);
        // Selection does not change stored validity, output eligibility, decay,
        // or timeout. A lone false observation cannot trigger stale recovery.
        Some(recovered.unwrap_or(incumbent))
    }

    pub fn decay_hypotheses(&mut self, decay_factor_criterion: impl Fn(&BallHypothesis) -> f32) {
        for hypothesis in self.hypotheses.iter_mut() {
            let decay_factor = decay_factor_criterion(hypothesis);
            hypothesis.validity *= decay_factor;
        }
    }

    pub fn predict(
        &mut self,
        delta_time: Duration,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        velocity_decay: f32,
        moving_process_noise: Matrix4<f32>,
        resting_process_noise: Matrix2<f32>,
        log_likelihood_of_zero_velocity_threshold: f32,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            hypothesis.predict(
                delta_time,
                last_to_current_odometry,
                velocity_decay,
                moving_process_noise,
                resting_process_noise,
                log_likelihood_of_zero_velocity_threshold,
            )
        }
    }

    pub fn reset(&mut self) {
        self.hypotheses.clear()
    }

    pub fn remove_hypotheses(
        &mut self,
        is_valid: impl Fn(&BallHypothesis) -> bool,
        merge_criterion: impl Fn(&BallHypothesis, &BallHypothesis) -> bool,
    ) -> Vec<BallHypothesis> {
        let (valid, removed): (Vec<_>, Vec<_>) = self.hypotheses.drain(..).partition(is_valid);

        let mut deduplicated: Vec<BallHypothesis> = Vec::new();
        for hypothesis in valid {
            let merged = deduplicated.iter_mut().any(|existing| {
                merge_criterion(existing, &hypothesis) && existing.merge(&hypothesis)
            });
            if !merged {
                deduplicated.push(hypothesis);
            }
        }
        self.hypotheses = deduplicated;

        removed
    }

    pub fn spawn(
        &mut self,
        detection_time: Time,
        measurement: MultivariateNormalDistribution<2>,
        initial_moving_covariance: Matrix4<f32>,
        nearby_spawn_validity_factor: Option<f32>,
    ) {
        // Nearby recent support can survive a tight association gate without
        // manufacturing confidence or copying an old position/unknown velocity.
        // Current-exposure matches (including other newborns) cannot donate.
        const MAXIMUM_PARENT_AGE: Duration = Duration::from_millis(250);
        const MAXIMUM_PARENT_DISTANCE: f32 = 0.2;
        let factor = nearby_spawn_validity_factor
            .filter(|factor| factor.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let parent = (factor > 0.0)
            .then(|| {
                self.hypotheses
                    .iter()
                    .enumerate()
                    .filter_map(|(index, hypothesis)| {
                        if !hypothesis.validity.is_finite()
                            || hypothesis.validity < 3.0
                            || hypothesis.last_seen >= detection_time
                            || detection_time.duration_since(hypothesis.last_seen)
                                > MAXIMUM_PARENT_AGE
                        {
                            return None;
                        }
                        let distance =
                            (hypothesis.position().position.inner.coords - measurement.mean).norm();
                        (distance.is_finite() && distance <= MAXIMUM_PARENT_DISTANCE)
                            .then_some((index, distance))
                    })
                    .min_by(|(_, left), (_, right)| left.total_cmp(right))
                    .map(|(index, _)| index)
            })
            .flatten();
        let bonus = if let Some(parent) = parent {
            let parent = &mut self.hypotheses[parent];
            let bonus = factor * (parent.validity - 1.0).clamp(0.0, 2.0);
            parent.validity -= bonus;
            bonus
        } else {
            0.0
        };
        // An unmatched percept represents a new ball or an abrupt motion change.
        // Starting at the nearest old track biases the new position toward an
        // unrelated ball and cannot infer velocity without a temporal match.
        let new_hypothesis = MultivariateNormalDistribution {
            mean: nalgebra::vector![measurement.mean.x, measurement.mean.y, 0.0, 0.0],
            covariance: initial_moving_covariance,
        };

        let new_hypothesis = BallHypothesis {
            mode: BallMode::Moving(new_hypothesis),
            last_seen: detection_time,
            validity: 1.0 + bonus,
            motion_evidence: None,
            negative_evidence: None,
            validity_decay_evidence: None,
            leadership_evidence: None,
            merge_observation_start: None,
        };

        self.hypotheses.push(new_hypothesis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(x: f32, validity: f32, last_seen: i64) -> BallHypothesis {
        let mut track = BallHypothesis::new(
            MultivariateNormalDistribution {
                mean: nalgebra::vector![x, 0.0, 0.0, 0.0],
                covariance: Matrix4::identity(),
            },
            Time::from_nanos(last_seen),
        );
        track.validity = validity;
        track
    }

    fn spawn_nearby(filter: &mut BallFilter, x: f32, factor: Option<f32>) {
        filter.spawn(
            Time::from_nanos(300_000_000),
            MultivariateNormalDistribution {
                mean: nalgebra::vector![x, 0.0],
                covariance: Matrix2::identity() * 0.01,
            },
            Matrix4::identity() * 0.2,
            factor,
        );
    }

    #[test]
    fn nearby_birth_transfers_bounded_confidence_without_copying_position_or_velocity() {
        for (factor, expected) in [
            (None, 1.0),
            (Some(0.0), 1.0),
            (Some(0.5), 2.0),
            (Some(1.0), 3.0),
            (Some(5.0), 3.0),
            (Some(f32::NAN), 1.0),
        ] {
            let mut parent = track(1.0, 50.0, 100_000_000);
            if let BallMode::Moving(state) = &mut parent.mode {
                state.mean.z = 5.0;
            }
            let mut filter = BallFilter {
                hypotheses: vec![parent],
            };
            spawn_nearby(&mut filter, 1.1, factor);
            let born = &filter.hypotheses[1];
            assert_eq!(born.validity, expected);
            assert_eq!(born.position().position, linear_algebra::point![1.1, 0.0]);
            assert_eq!(born.position().velocity, linear_algebra::Vector2::zeros());
            assert_eq!(born.position_covariance(), Matrix2::identity() * 0.2);
            assert_eq!(born.last_seen, Time::from_nanos(300_000_000));
            assert_eq!(
                filter.hypotheses[0].last_seen,
                Time::from_nanos(100_000_000)
            );
            assert_eq!(filter.hypotheses[0].validity + born.validity, 51.0);
        }
    }

    #[test]
    fn nearby_birth_requires_recent_unmatched_confirmed_parent() {
        for parent in [
            track(0.89, 10.0, 100_000_000), // More than 0.2 m away.
            track(1.0, 10.0, 49_999_999),   // Older than 250 ms.
            track(1.0, 10.0, 300_000_000),  // Already observed in this image.
            track(1.0, 10.0, 320_000_000),  // Future observation is not evidence.
            track(1.0, 2.99, 100_000_000),
            track(1.0, f32::INFINITY, 100_000_000),
        ] {
            let validity = parent.validity;
            let mut filter = BallFilter {
                hypotheses: vec![parent],
            };
            spawn_nearby(&mut filter, 1.1, Some(1.0));
            assert_eq!(filter.hypotheses[1].validity, 1.0);
            assert_eq!(filter.hypotheses[0].validity, validity);
        }
    }

    #[test]
    fn multiple_births_cannot_clone_confidence_or_reuse_newborns_as_parents() {
        let mut filter = BallFilter {
            hypotheses: vec![track(1.0, 3.0, 100_000_000)],
        };
        spawn_nearby(&mut filter, 1.1, Some(1.0));
        spawn_nearby(&mut filter, 1.15, Some(1.0));
        assert_eq!(
            filter
                .hypotheses
                .iter()
                .map(|h| h.validity)
                .collect::<Vec<_>>(),
            vec![1.0, 3.0, 1.0]
        );
        assert_eq!(
            filter.hypotheses.iter().map(|h| h.validity).sum::<f32>(),
            5.0
        );
    }

    #[test]
    fn nearby_birth_uses_nearest_parent_after_odometry_compensation() {
        let mut filter = BallFilter {
            hypotheses: vec![track(1.0, 10.0, 100_000_000), track(1.08, 8.0, 100_000_000)],
        };
        filter.predict(
            Duration::ZERO,
            Isometry2::from_parts(linear_algebra::vector![-0.5, 0.0], 0.0),
            1.0,
            Matrix4::zeros(),
            Matrix2::zeros(),
            f32::INFINITY,
        );
        spawn_nearby(&mut filter, 0.6, Some(0.5));
        assert_eq!(filter.hypotheses[0].validity, 10.0);
        assert_eq!(filter.hypotheses[1].validity, 7.0);
        assert_eq!(filter.hypotheses[2].validity, 2.0);
    }

    #[test]
    fn confirmed_recent_track_can_replace_long_observed_stale_track() {
        let mut filter = BallFilter {
            hypotheses: vec![track(0.0, 25.0, 0), track(3.0, 1.0, 500_000_000)],
        };
        assert_eq!(
            filter.best_hypothesis(0.5).unwrap().position().position.x(),
            0.0
        );
        filter.hypotheses[1].validity = 2.99;
        assert_eq!(
            filter.best_hypothesis(0.5).unwrap().position().position.x(),
            0.0
        );
        filter.hypotheses[1].validity = 3.0;
        assert_eq!(
            filter.best_hypothesis(0.5).unwrap().position().position.x(),
            3.0
        );
        assert_eq!(filter.hypotheses[0].validity, 25.0);
        assert_eq!(filter.hypotheses[0].last_seen, Time::zero());
    }

    #[test]
    fn confirmed_competitor_cannot_take_over_during_short_perception_interruptions() {
        let mut filter = BallFilter {
            hypotheses: vec![track(0.0, 25.0, 0), track(3.0, 7.0, 40_000_000)],
        };
        for observed in [40_000_000, 80_000_000, 120_000_000] {
            filter.hypotheses[1].last_seen = Time::from_nanos(observed);
            assert_eq!(
                filter.best_hypothesis(0.5).unwrap().position().position.x(),
                0.0
            );
        }
        filter.hypotheses[1].last_seen = Time::from_nanos(120_000_001);
        assert_eq!(
            filter.best_hypothesis(0.5).unwrap().position().position.x(),
            3.0
        );

        // A competing track observed during intermittent one-image dropouts
        // must not repeatedly steal selection from the well-established ball.
        for observed in [200_000_000, 280_000_000, 360_000_000] {
            filter.hypotheses[0].last_seen = Time::from_nanos(observed);
            filter.hypotheses[1].last_seen = Time::from_nanos(observed + 40_000_000);
            assert_eq!(
                filter.best_hypothesis(0.5).unwrap().position().position.x(),
                0.0
            );
        }
    }

    #[test]
    fn simultaneously_observed_balls_keep_confidence_order_in_either_storage_order() {
        let mut filter = BallFilter {
            hypotheses: vec![track(0.0, 25.0, 1_000_000), track(3.0, 7.0, 1_000_000)],
        };
        assert_eq!(
            filter.best_hypothesis(0.5).unwrap().position().position.x(),
            0.0
        );
        filter.hypotheses.reverse();
        assert_eq!(
            filter.best_hypothesis(0.5).unwrap().position().position.x(),
            0.0
        );
    }

    #[test]
    fn high_output_threshold_still_requires_enough_evidence() {
        let mut filter = BallFilter {
            hypotheses: vec![track(0.0, 25.0, 0), track(3.0, 4.0, 500_000_000)],
        };
        assert_eq!(
            filter.best_hypothesis(5.0).unwrap().position().position.x(),
            0.0
        );
        filter.hypotheses[1].validity = 5.0;
        assert_eq!(
            filter.best_hypothesis(5.0).unwrap().position().position.x(),
            3.0
        );
        filter.hypotheses.remove(0);
        assert!(filter.best_hypothesis(5.1).is_none());
    }
}

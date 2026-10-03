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

        self.hypotheses = valid
            .into_iter()
            .fold(vec![], |mut deduplicated, hypothesis| {
                let mergeable_hypothesis = deduplicated
                    .iter_mut()
                    .find(|existing_hypothesis| merge_criterion(existing_hypothesis, &hypothesis));

                if let Some(mergeable_hypothesis) = mergeable_hypothesis {
                    mergeable_hypothesis.merge(hypothesis)
                } else {
                    deduplicated.push(hypothesis);
                }

                deduplicated
            });

        removed
    }

    pub fn spawn(
        &mut self,
        detection_time: Time,
        measurement: MultivariateNormalDistribution<2>,
        initial_moving_covariance: Matrix4<f32>,
    ) {
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
            validity: 1.0,
            motion_evidence: None,
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

use std::time::Duration;

use coordinate_systems::Ground;
use linear_algebra::Isometry2;
use nalgebra::{Matrix2, Matrix4};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

use crate::hypothesis::{BallHypothesis, BallMode};

#[derive(Debug, Default, Clone, Serialize, Deserialize, Message)]
pub struct BallFilter {
    pub hypotheses: Vec<BallHypothesis>,
}

impl BallFilter {
    pub fn best_hypothesis(&self, validity_threshold: f32) -> Option<&BallHypothesis> {
        self.hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.validity >= validity_threshold)
            .max_by(|a, b| a.validity.total_cmp(&b.validity))
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
        };

        self.hypotheses.push(new_hypothesis)
    }
}

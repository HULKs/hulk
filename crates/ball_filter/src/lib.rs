use std::time::{Duration, SystemTime};

use coordinate_systems::Ground;
use linear_algebra::{Isometry2, Point2};
use nalgebra::{Matrix2, Matrix4};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

mod hypothesis;

pub use hypothesis::BallHypothesis;
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(
    Debug, Default, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BallFilter {
    hypotheses: Vec<BallHypothesis>,
}

impl BallFilter {
    pub fn best_hypothesis(&self, validity_threshold: f32) -> Option<&BallHypothesis> {
        self.hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.validity >= validity_threshold)
            .max_by(|a, b| a.validity.partial_cmp(&b.validity).unwrap())
    }

    pub fn hypotheses(&self) -> &Vec<BallHypothesis> {
        &self.hypotheses
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
        velocity_threshold: f32,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            hypothesis.predict(
                delta_time,
                last_to_current_odometry,
                velocity_decay,
                moving_process_noise,
                resting_process_noise,
                velocity_threshold,
            )
        }
    }

    pub fn update(
        &mut self,
        detection_time: SystemTime,
        measurement: Point2<Ground>,
        noise: Matrix2<f32>,
        matching_criterion: impl Fn(&BallHypothesis) -> bool,
    ) -> bool {
        let mut number_of_matching_hypotheses = 0;

        for hypothesis in self
            .hypotheses
            .iter_mut()
            .filter(|hypothesis| matching_criterion(hypothesis))
        {
            number_of_matching_hypotheses += 1;
            hypothesis.update(detection_time, measurement, noise)
        }

        number_of_matching_hypotheses > 0
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
        detection_time: SystemTime,
        measurement: Point2<Ground>,
        initial_moving_covariance: Matrix4<f32>,
        initial_resting_covariance: Matrix2<f32>,
    ) {
        let initial_state = nalgebra::vector![measurement.x(), measurement.y(), 0.0, 0.0];

        let moving_hypothesis = MultivariateNormalDistribution {
            mean: initial_state,
            covariance: initial_moving_covariance,
        };
        let resting_hypothesis = MultivariateNormalDistribution {
            mean: initial_state.xy(),
            covariance: initial_resting_covariance,
        };

        self.hypotheses.push(BallHypothesis::new(
            moving_hypothesis,
            resting_hypothesis,
            detection_time,
        ))
    }
}

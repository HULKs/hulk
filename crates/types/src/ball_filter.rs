use std::time::SystemTime;

use nalgebra::{vector, Point2};
use serde::{Deserialize, Serialize};

use crate::{
    configuration::BallFilterConfiguration,
    multivariate_normal_distribution::MultivariateNormalDistribution, BallPosition,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hypothesis {
    pub moving_state: MultivariateNormalDistribution<4>,
    pub resting_state: MultivariateNormalDistribution<4>,

    pub validity: f32,
    pub last_update: SystemTime,
}

impl Hypothesis {
    pub fn is_resting(&self, configuration: &BallFilterConfiguration) -> bool {
        self.moving_state.mean.rows(2, 2).norm() < configuration.resting_ball_velocity_threshold
    }

    pub fn selected_ball_position(&self, configuration: &BallFilterConfiguration) -> BallPosition {
        if self.is_resting(configuration) {
            BallPosition {
                position: Point2::from(self.resting_state.mean.xy()),
                velocity: vector![self.resting_state.mean.z, self.resting_state.mean.w],
                last_seen: self.last_update,
            }
        } else {
            BallPosition {
                position: Point2::from(self.moving_state.mean.xy()),
                velocity: vector![self.moving_state.mean.z, self.moving_state.mean.w],
                last_seen: self.last_update,
            }
        }
    }
}

use std::time::SystemTime;

use coordinate_systems::Ground;
use linear_algebra::{vector, Point};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    ball_position::BallPosition, multivariate_normal_distribution::MultivariateNormalDistribution,
    parameters::BallFilterParameters,
};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct Hypothesis {
    pub moving_state: MultivariateNormalDistribution<4>,
    pub resting_state: MultivariateNormalDistribution<4>,

    pub validity: f32,
    pub last_update: SystemTime,
}

impl Hypothesis {
    pub fn is_resting(&self, configuration: &BallFilterParameters) -> bool {
        self.moving_state.mean.rows(2, 2).norm() < configuration.resting_ball_velocity_threshold
    }

    pub fn selected_state(
        &self,
        configuration: &BallFilterParameters,
    ) -> MultivariateNormalDistribution<4> {
        if self.is_resting(configuration) {
            self.resting_state
        } else {
            self.moving_state
        }
    }

    pub fn selected_ball_position(
        &self,
        configuration: &BallFilterParameters,
    ) -> BallPosition<Ground> {
        let selected_state = self.selected_state(configuration);

        BallPosition {
            position: Point::from(selected_state.mean.xy()),
            velocity: vector![selected_state.mean.z, selected_state.mean.w],
            last_seen: self.last_update,
        }
    }
}

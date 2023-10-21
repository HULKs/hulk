use std::time::SystemTime;

use nalgebra::{vector, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    ball_position::BallPosition, multivariate_normal_distribution::MultivariateNormalDistribution,
    parameters::BallFilterParameters,
};

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
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

    pub fn selected_ball_position(&self, configuration: &BallFilterParameters) -> BallPosition {
        let selected_state = self.selected_state(configuration);

        let rest_position = rest_position(selected_state, configuration);

        BallPosition {
            position: Point2::from(selected_state.mean.xy()),
            rest_position,
            velocity: vector![selected_state.mean.z, selected_state.mean.w],
            last_seen: self.last_update,
            is_resting: self.is_resting(configuration),
        }
    }
}

pub fn rest_position(
    state: MultivariateNormalDistribution<4>,
    configuration: &BallFilterParameters,
) -> Point2<f32> {
    let decay = configuration.linear_velocity_decay;
    let square_decay = configuration.square_velocity_decay;

    let mut position = Point2::from(state.mean.xy());
    let mut velocity = vector![state.mean.z, state.mean.w];

    while velocity.norm_squared() > 0.01 {
        position += velocity * 0.012;
        velocity -= velocity * (1.0 - decay) + velocity * velocity.norm() * square_decay;
    }

    position
}

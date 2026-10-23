use hsl_network_messages::PlayerNumber;
use nalgebra::{Matrix2, Matrix4, Vector4};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

use crate::{
    multivariate_normal_distribution::MultivariateNormalDistribution, obstacles::ObstacleKind,
    tracked_objects::StationaryClass,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct Hypothesis {
    pub id: u64,
    /// Position in the filter's current Ground frame.
    pub state: MultivariateNormalDistribution<2>,
    /// Physical velocity in Ground, excluding the robot's own motion.
    pub velocity: MultivariateNormalDistribution<2>,
    pub position_velocity_covariance: Matrix2<f32>,
    pub measurement_count: usize,
    pub last_update: Time,
    pub last_detection: Option<Time>,
    pub obstacle_kind: ObstacleKind,
    pub stationary_class: Option<StationaryClass>,
    pub teammate: Option<PlayerNumber>,
    pub teammate_evidence: usize,
    pub last_team_report: Option<Time>,
}

impl Hypothesis {
    pub fn state_with_velocity(&self) -> MultivariateNormalDistribution<4> {
        let mut covariance = Matrix4::zeros();
        covariance
            .fixed_view_mut::<2, 2>(0, 0)
            .copy_from(&self.state.covariance);
        covariance
            .fixed_view_mut::<2, 2>(2, 2)
            .copy_from(&self.velocity.covariance);
        covariance
            .fixed_view_mut::<2, 2>(0, 2)
            .copy_from(&self.position_velocity_covariance);
        covariance
            .fixed_view_mut::<2, 2>(2, 0)
            .copy_from(&self.position_velocity_covariance.transpose());
        MultivariateNormalDistribution {
            mean: Vector4::new(
                self.state.mean.x,
                self.state.mean.y,
                self.velocity.mean.x,
                self.velocity.mean.y,
            ),
            covariance,
        }
    }

    pub fn set_state_with_velocity(&mut self, state: MultivariateNormalDistribution<4>) {
        self.state.mean = state.mean.fixed_rows::<2>(0).into_owned();
        self.velocity.mean = state.mean.fixed_rows::<2>(2).into_owned();
        self.state.covariance = state.covariance.fixed_view::<2, 2>(0, 0).into_owned();
        self.velocity.covariance = state.covariance.fixed_view::<2, 2>(2, 2).into_owned();
        self.position_velocity_covariance = state.covariance.fixed_view::<2, 2>(0, 2).into_owned();
    }

    pub fn is_moving(&self) -> bool {
        matches!(
            self.obstacle_kind,
            ObstacleKind::Robot | ObstacleKind::Person
        )
    }
}

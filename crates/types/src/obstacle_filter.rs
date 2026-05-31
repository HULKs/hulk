use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

use crate::{
    multivariate_normal_distribution::MultivariateNormalDistribution, obstacles::ObstacleKind,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct Hypothesis {
    pub state: MultivariateNormalDistribution<2>,
    pub measurement_count: usize,
    pub last_update: Time,
    pub obstacle_kind: ObstacleKind,
}

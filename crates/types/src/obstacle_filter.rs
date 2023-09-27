use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::{
    multivariate_normal_distribution::MultivariateNormalDistribution, obstacles::ObstacleKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub state: MultivariateNormalDistribution<2>,
    pub measurement_count: usize,
    pub last_update: SystemTime,
    pub obstacle_kind: ObstacleKind,
}

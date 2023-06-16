use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hypothesis {
    pub moving_state: MultivariateNormalDistribution<4>,
    pub resting_state: MultivariateNormalDistribution<4>,

    pub validity: f32,
    pub last_update: SystemTime,
}

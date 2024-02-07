use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{multivariate_normal_distribution::MultivariateNormalDistribution};

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Hypothesis {
    // [ground_x, ground_y, velocity_x, velocity_y]
    pub bounding_box: MultivariateNormalDistribution<4>,

    pub validity: f32,
    pub last_update: SystemTime,
}

impl Hypothesis {
    
}

use std::time::SystemTime;

use nalgebra::{Matrix2, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Hypothesis {
    // [ground_x, ground_y, velocity_x, velocity_y]
    pub robot_state: MultivariateNormalDistribution<4>,

    pub validity: f32,
    pub last_update: SystemTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Measurement {
    pub location: Point2<f32>,
    pub score: f32,
    #[serialize_hierarchy(leaf)]
    pub projected_error: Matrix2<f32>,
}

use nalgebra::{vector, Isometry2, Matrix3, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct Update {
    pub robot_to_field: Isometry2<f32>,
    pub line_center_point: Point2<f32>,
    pub fit_error: f32,
    pub number_of_measurements_weight: f32,
    pub line_distance_to_robot: f32,
    pub line_length_weight: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct ScoredPose {
    pub state: MultivariateNormalDistribution<3>,
    pub score: f32,
}

impl ScoredPose {
    pub fn from_isometry(pose: Isometry2<f32>, covariance: Matrix3<f32>, score: f32) -> Self {
        Self {
            state: MultivariateNormalDistribution {
                mean: vector![
                    pose.translation.x,
                    pose.translation.y,
                    pose.rotation.angle()
                ],
                covariance,
            },
            score,
        }
    }
}

use coordinate_systems::{Isometry2, Point2, Pose};
use nalgebra::{vector, Matrix3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    coordinate_systems::{Field, Ground},
    multivariate_normal_distribution::MultivariateNormalDistribution,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct Update {
    pub ground_to_field: Isometry2<Ground, Field>,
    pub line_center_point: Point2<Field>,
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
    pub fn from_isometry(pose: Pose<Field>, covariance: Matrix3<f32>, score: f32) -> Self {
        Self {
            state: MultivariateNormalDistribution {
                mean: vector![
                    pose.position().x(),
                    pose.position().y(),
                    pose.orientation().angle(),
                ],
                covariance,
            },
            score,
        }
    }
}

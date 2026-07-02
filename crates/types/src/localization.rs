use nalgebra::{Matrix3, vector};
use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground, Robot};
use linear_algebra::{Isometry2, Isometry3, Point2, Pose2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    ros_z::Message,
)]
pub struct Update {
    pub ground_to_field: Isometry2<Ground, Field>,
    pub line_center_point: Point2<Field>,
    pub fit_error: f32,
    pub number_of_measurements_weight: f32,
    pub line_distance_to_robot: f32,
    pub line_length_weight: f32,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Message,
)]
pub struct ScoredPose {
    pub state: MultivariateNormalDistribution<3>,
    pub score: f32,
}

impl ScoredPose {
    pub fn from_isometry(pose: Pose2<Field>, covariance: Matrix3<f32>, score: f32) -> Self {
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

pub fn ground_to_field_from_field_to_robot(
    field_to_robot: Isometry3<Field, Robot>,
    robot_to_ground: Isometry3<Robot, Ground>,
) -> Isometry2<Ground, Field> {
    let robot_to_field = field_to_robot.inverse();
    let ground_to_field = robot_to_field * robot_to_ground.inverse();
    let (_, _, field_to_robot_yaw) = field_to_robot.inner.rotation.euler_angles();
    let yaw = -field_to_robot_yaw;
    let translation = ground_to_field.inner.translation.vector;

    Isometry2::wrap(nalgebra::Isometry2::new(
        nalgebra::vector![translation.x, translation.y],
        yaw,
    ))
}

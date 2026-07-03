use nalgebra::{Matrix3, vector};
use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground, Robot};
use linear_algebra::{Isometry2, Isometry3, Point2, Pose2};

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ros_z::Message)]
pub struct Update {
    pub ground_to_field: Isometry2<Ground, Field>,
    pub line_center_point: Point2<Field>,
    pub fit_error: f32,
    pub number_of_measurements_weight: f32,
    pub line_distance_to_robot: f32,
    pub line_length_weight: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
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

#[cfg(test)]
mod tests {
    use linear_algebra::IntoTransform;

    use super::*;

    #[test]
    fn ground_to_field_from_field_to_robot_flattens_robot_pose() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.7),
        );
        let field_to_robot: Isometry3<Field, Robot> = robot_to_field.inverse().framed_transform();
        let robot_to_ground = Isometry3::identity();

        let ground_to_field = ground_to_field_from_field_to_robot(field_to_robot, robot_to_ground);

        assert!((ground_to_field.translation().x() - 1.5).abs() < 1.0e-6);
        assert!((ground_to_field.translation().y() + 2.0).abs() < 1.0e-6);
        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn ground_to_field_from_field_to_robot_ignores_ground_roll_pitch() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.7),
        );
        let field_to_robot: Isometry3<Field, Robot> = robot_to_field.inverse().framed_transform();
        let robot_to_ground: Isometry3<Robot, Ground> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.0, 0.0, 0.523),
            nalgebra::UnitQuaternion::from_euler_angles(0.045, 0.047, 0.0),
        )
        .framed_transform();

        let ground_to_field = ground_to_field_from_field_to_robot(field_to_robot, robot_to_ground);

        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
    }
}

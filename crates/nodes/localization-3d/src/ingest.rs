use color_eyre::Result;
use coordinate_systems::Robot;
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Point3;
use localization_factrs::{VinsFrontend, VinsFrontendError};
use projection::camera_matrix::CameraMatrix;
use types::{
    time_wrapper::TimeWrapper, visual_odometry::VisualOdometryDelta as VisualOdometryDeltaMessage,
};

use crate::camera::robot_to_camera;

/// Ingests one visual-odometry delta into the VINS frontend.
///
/// `previous_camera_matrix` and `current_camera_matrix` provide the robot-to-camera extrinsics for
/// the two endpoints of the visual-odometry measurement.
pub fn ingest_visual_odometry(
    frontend: &mut VinsFrontend,
    delta: VisualOdometryDeltaMessage,
    previous_camera_matrix: &CameraMatrix,
    current_camera_matrix: &CameraMatrix,
) -> Result<(), VinsFrontendError> {
    frontend.ingest_visual_odometry_delta(
        delta.previous_time.to_wallclock(),
        delta.current_time.to_wallclock(),
        robot_to_camera(previous_camera_matrix),
        robot_to_camera(current_camera_matrix),
        delta.current_left_camera_to_previous_left_camera,
    )
}

/// Ingests left and right foot-height observations into the VINS frontend.
///
/// The sole positions are read from `robot_kinematics` in the robot frame and timestamped with the
/// kinematics message time.
pub fn ingest_foot_heights(
    frontend: &mut VinsFrontend,
    robot_kinematics: TimeWrapper<RobotKinematics>,
) -> Result<(), VinsFrontendError> {
    let (left_sole_in_robot, right_sole_in_robot) = foot_height_points(&robot_kinematics.inner);

    frontend.ingest_foot_heights(
        robot_kinematics.time.to_wallclock(),
        left_sole_in_robot,
        right_sole_in_robot,
    )
}

fn foot_height_points(robot_kinematics: &RobotKinematics) -> (Point3<Robot>, Point3<Robot>) {
    (
        robot_kinematics.left_leg.sole_to_robot.translation(),
        robot_kinematics.right_leg.sole_to_robot.translation(),
    )
}

#[cfg(test)]
mod tests {
    use coordinate_systems::{LeftSole, RightSole};
    use linear_algebra::{IntoTransform, Isometry3, point};

    use super::*;

    #[test]
    fn foot_height_points_use_sole_positions_in_robot_frame() {
        let left_sole_to_robot: Isometry3<LeftSole, Robot> =
            nalgebra::Isometry3::translation(0.1, 0.2, -0.3).framed_transform();
        let right_sole_to_robot: Isometry3<RightSole, Robot> =
            nalgebra::Isometry3::translation(0.4, -0.5, -0.6).framed_transform();
        let robot_kinematics = RobotKinematics {
            left_leg: kinematics::robot_kinematics::RobotLeftLegKinematics {
                sole_to_robot: left_sole_to_robot,
                ..Default::default()
            },
            right_leg: kinematics::robot_kinematics::RobotRightLegKinematics {
                sole_to_robot: right_sole_to_robot,
                ..Default::default()
            },
            ..Default::default()
        };

        let (left, right) = foot_height_points(&robot_kinematics);

        assert!((left - point![<Robot>, 0.1, 0.2, -0.3]).norm() < 1.0e-6);
        assert!((right - point![<Robot>, 0.4, -0.5, -0.6]).norm() < 1.0e-6);
    }
}

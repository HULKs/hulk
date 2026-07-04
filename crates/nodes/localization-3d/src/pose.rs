use coordinate_systems::{Field, Ground, Robot};
use kinematics::{forward::left_sole_to_robot, joints::leg::LegJoints};
use linear_algebra::{IntoTransform, Isometry3};
use localization_factrs::{InitialState, OptimizationResult};
use projection::camera_matrix::CameraMatrix;
use types::time_wrapper::TimeWrapper;

use crate::camera::camera_intrinsics_from_matrix;

/// Constructs the backend initial state from the first live camera matrix.
///
/// Startup localization is intentionally position-agnostic. Global field-mark association selects
/// the startup symmetry branch before the backend is reset to a visual pose.
pub fn initial_state_from_camera_matrix(camera_matrix: &CameraMatrix) -> InitialState {
    InitialState::from_initial_height_and_intrinsics(
        robot_height() as f64,
        camera_intrinsics_from_matrix(camera_matrix),
    )
}

pub(crate) fn backend_localization_for_result(
    result: &OptimizationResult,
    camera_matrix: Option<&TimeWrapper<CameraMatrix>>,
) -> Isometry3<Field, Robot> {
    let backend_localization = localization_transform_from_backend_pose(&result.transform);
    camera_matrix
        .map(|camera_matrix| {
            constrain_localization_to_ground(
                backend_localization,
                &camera_matrix.inner.ground_to_robot,
            )
        })
        .unwrap_or(backend_localization)
}

fn localization_transform_from_backend_pose(
    robot_to_field: &nalgebra::Isometry3<f64>,
) -> Isometry3<Field, Robot> {
    robot_to_field.cast::<f32>().inverse().framed_transform()
}

pub(crate) fn localization_transform_constrained_to_ground(
    robot_to_field: &nalgebra::Isometry3<f64>,
    ground_to_robot: &Isometry3<Ground, Robot>,
) -> Isometry3<Field, Robot> {
    let localization = localization_transform_from_backend_pose(robot_to_field);

    constrain_localization_to_ground(localization, ground_to_robot)
}

fn constrain_localization_to_ground(
    localization: Isometry3<Field, Robot>,
    ground_to_robot: &Isometry3<Ground, Robot>,
) -> Isometry3<Field, Robot> {
    let (_, _, yaw) = localization.inner.rotation.euler_angles();
    let (roll, pitch, _) = ground_to_robot.inner.rotation.euler_angles();

    let mut translation = localization.inner.translation;
    translation.vector.z = ground_to_robot.inner.translation.vector.z;

    nalgebra::Isometry3::from_parts(
        translation,
        nalgebra::UnitQuaternion::from_euler_angles(roll, pitch, yaw),
    )
    .framed_transform()
}

fn robot_height() -> f32 {
    -left_sole_to_robot(&LegJoints::default()).translation().z()
}

#[cfg(test)]
mod tests {
    use linear_algebra::point;
    use projection::intrinsic::Intrinsic;

    use super::*;

    #[test]
    fn initial_state_from_camera_matrix_uses_height_only_and_live_intrinsics() {
        let robot_to_ground_rotation = nalgebra::UnitQuaternion::from_euler_angles(0.1, -0.2, 0.3);
        let robot_to_ground = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.0, 2.0, 0.42),
            robot_to_ground_rotation,
        );
        let camera_matrix = CameraMatrix {
            ground_to_robot: robot_to_ground.inverse().framed_transform(),
            intrinsics: Intrinsic::new(nalgebra::vector![216.0, 217.0], point![251.0, 235.0]),
            ..Default::default()
        };

        let initial_state = initial_state_from_camera_matrix(&camera_matrix);

        assert!(initial_state.pose.xyz().x.abs() < 1.0e-9);
        assert!(initial_state.pose.xyz().y.abs() < 1.0e-9);
        assert!((initial_state.pose.xyz().z - robot_height() as f64).abs() < 1.0e-9);
        assert!(initial_state.pose.uvw().norm() < 1.0e-9);
        assert_eq!(
            initial_state.camera_intrinsics.focals(),
            nalgebra::vector![216.0, 217.0]
        );
        assert_eq!(
            initial_state.camera_intrinsics.optical_center(),
            nalgebra::vector![251.0, 235.0]
        );
        let rotation = initial_state.pose.rot();
        assert!((rotation.w() - 1.0).abs() < 1.0e-9);
        assert!(rotation.x().abs() < 1.0e-9);
        assert!(rotation.y().abs() < 1.0e-9);
        assert!(rotation.z().abs() < 1.0e-9);
    }

    #[test]
    fn localization_publisher_outputs_field_to_robot() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(-3.0, 0.25, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.3),
        );

        let field_to_robot = localization_transform_from_backend_pose(&robot_to_field);
        let roundtrip_robot_to_field = field_to_robot.inverse().inner.cast::<f64>();

        assert!(
            (roundtrip_robot_to_field.translation.vector - robot_to_field.translation.vector)
                .norm()
                < 1.0e-6
        );
        assert!(
            roundtrip_robot_to_field
                .rotation
                .angle_to(&robot_to_field.rotation)
                < 1.0e-6
        );
    }

    #[test]
    fn constrained_localization_preserves_yaw_and_xy_but_uses_ground_tilt_and_height() {
        let localization: Isometry3<Field, Robot> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, -1.7),
            nalgebra::UnitQuaternion::from_euler_angles(3.0, 0.2, 0.7),
        )
        .framed_transform();
        let ground_to_robot: Isometry3<Ground, Robot> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.01, -0.02, -0.523),
            nalgebra::UnitQuaternion::from_euler_angles(-0.045, -0.047, 0.1),
        )
        .framed_transform();

        let constrained = constrain_localization_to_ground(localization, &ground_to_robot);
        let (roll, pitch, yaw) = constrained.inner.rotation.euler_angles();

        assert!((constrained.inner.translation.vector.x - 1.5).abs() < 1.0e-6);
        assert!((constrained.inner.translation.vector.y + 2.0).abs() < 1.0e-6);
        assert!((constrained.inner.translation.vector.z + 0.523).abs() < 1.0e-6);
        assert!((roll + 0.045).abs() < 1.0e-6);
        assert!((pitch + 0.047).abs() < 1.0e-6);
        assert!((yaw - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn robot_height_is_sensible() {
        assert!(robot_height() > 0.3);
        assert!(robot_height() < 1.0);
    }
}

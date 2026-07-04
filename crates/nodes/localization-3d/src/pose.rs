use coordinate_systems::{Field, Ground, Robot};
use kinematics::{forward::left_sole_to_robot, joints::leg::LegJoints};
use linear_algebra::{IntoTransform, Isometry3, Orientation3};
use localization_factrs::{InitialState, OptimizationResult};
use projection::camera_matrix::CameraMatrix;
use types::{field_dimensions::FieldDimensions, time_wrapper::TimeWrapper};

use crate::camera::camera_intrinsics_from_matrix;

/// Constructs the backend initial state from the first live camera matrix.
///
/// The initial pose uses a fixed own-half right-touchline prior and the camera intrinsics contained
/// in `camera_matrix`.
pub fn initial_state_from_camera_matrix(
    camera_matrix: &CameraMatrix,
    field_dimensions: &FieldDimensions,
) -> InitialState {
    let initial_pose = initial_robot_to_field_from_field_dimensions(field_dimensions);

    InitialState::from_robot_to_field_and_intrinsics(
        initial_pose,
        camera_intrinsics_from_matrix(camera_matrix),
    )
}

/// Estimates the initial robot pose in the field frame from known startup placement.
///
/// Robots start on the right touchline in their own half, looking into the field.
pub fn initial_robot_to_field_from_field_dimensions(
    field_dimensions: &FieldDimensions,
) -> Isometry3<Robot, Field> {
    let translation: linear_algebra::Vector3<Field> =
        linear_algebra::Vector3::wrap(nalgebra::vector![
            -field_dimensions.length / 2.0,
            -field_dimensions.width / 2.0,
            robot_height(),
        ]);
    let orientation: Orientation3<Field> =
        Orientation3::from_euler_angles(0.0, 0.0, std::f32::consts::FRAC_PI_2);

    Isometry3::from_parts(translation, orientation)
}

pub(crate) fn initial_localization_for_branch_hint(
    field_dimensions: &FieldDimensions,
) -> Option<Isometry3<Field, Robot>> {
    Some(initial_robot_to_field_from_field_dimensions(field_dimensions).inverse())
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

fn constrain_localization_to_ground(
    localization: Isometry3<Field, Robot>,
    ground_to_robot: &Isometry3<Ground, Robot>,
) -> Isometry3<Field, Robot> {
    let robot_to_field = localization.inverse();
    let ground_to_field = robot_to_field * *ground_to_robot;
    let (_, _, yaw) = ground_to_field.inner.rotation.euler_angles();
    let translation = ground_to_field.inner.translation.vector;

    let flattened_ground_to_field = Isometry3::from_parts(
        linear_algebra::vector![<Field>, translation.x, translation.y, 0.0],
        Orientation3::from_euler_angles(0.0, 0.0, yaw),
    );
    let constrained_robot_to_field = flattened_ground_to_field * ground_to_robot.inverse();

    constrained_robot_to_field.inverse()
}

fn robot_height() -> f32 {
    -left_sole_to_robot(&LegJoints::default()).translation().z()
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use linear_algebra::point;
    use projection::intrinsic::Intrinsic;

    use super::*;

    #[test]
    fn initial_state_from_camera_matrix_uses_startup_prior_and_live_intrinsics() {
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

        let initial_state =
            initial_state_from_camera_matrix(&camera_matrix, &FieldDimensions::SPL_2025);

        assert!(
            (initial_state.pose.xyz().x + FieldDimensions::SPL_2025.length as f64 / 2.0).abs()
                < 1.0e-9
        );
        assert!(
            (initial_state.pose.xyz().y + FieldDimensions::SPL_2025.width as f64 / 2.0).abs()
                < 1.0e-9
        );
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
        let yaw = (2.0 * (rotation.w() * rotation.z() + rotation.x() * rotation.y()))
            .atan2(1.0 - 2.0 * (rotation.y().powi(2) + rotation.z().powi(2)));
        assert!((yaw - FRAC_PI_2).abs() < 1.0e-6);
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
    fn initial_localization_branch_hint_uses_startup_prior() {
        let hint = initial_localization_for_branch_hint(&FieldDimensions::SPL_2025)
            .expect("initial branch hint should be available");
        let robot_to_field = hint.inverse();
        let expected = initial_robot_to_field_from_field_dimensions(&FieldDimensions::SPL_2025);

        assert!(
            (robot_to_field.inner.translation.vector - expected.inner.translation.vector).norm()
                < 1.0e-6
        );
        assert!(
            robot_to_field
                .inner
                .rotation
                .angle_to(&expected.inner.rotation)
                < 1.0e-6
        );
    }

    #[test]
    fn constrained_localization_preserves_ground_pose_without_height_coupling() {
        let robot_to_field: Isometry3<Robot, Field> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(4.0, -3.0, 0.6),
            nalgebra::UnitQuaternion::from_euler_angles(0.2, -0.3, 0.7),
        )
        .framed_transform();
        let localization = robot_to_field.inverse();
        let ground_to_robot: Isometry3<Ground, Robot> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.01, -0.02, -0.523),
            nalgebra::UnitQuaternion::from_euler_angles(-0.045, -0.047, 0.0),
        )
        .framed_transform();

        let constrained = constrain_localization_to_ground(localization, &ground_to_robot);
        let constrained_robot_to_field = constrained.inverse();
        let constrained_ground_to_field = constrained_robot_to_field * ground_to_robot;
        let original_ground_to_field = robot_to_field * ground_to_robot;
        let (roll, pitch, yaw) = constrained_ground_to_field.inner.rotation.euler_angles();
        let (_, _, original_yaw) = original_ground_to_field.inner.rotation.euler_angles();
        let robot_to_ground = ground_to_robot.inverse();

        assert!(
            (constrained_ground_to_field.inner.translation.vector.x
                - original_ground_to_field.inner.translation.vector.x)
                .abs()
                < 1.0e-6
        );
        assert!(
            (constrained_ground_to_field.inner.translation.vector.y
                - original_ground_to_field.inner.translation.vector.y)
                .abs()
                < 1.0e-6
        );
        assert!(constrained_ground_to_field.inner.translation.vector.z.abs() < 1.0e-6);
        assert!(roll.abs() < 1.0e-6);
        assert!(pitch.abs() < 1.0e-6);
        assert!((yaw - original_yaw).abs() < 1.0e-6);
        assert!(
            (constrained_robot_to_field.inner.translation.vector.z
                - robot_to_ground.inner.translation.vector.z)
                .abs()
                < 1.0e-6
        );
    }

    #[test]
    fn robot_height_is_sensible() {
        assert!(robot_height() > 0.3);
        assert!(robot_height() < 1.0);
    }
}

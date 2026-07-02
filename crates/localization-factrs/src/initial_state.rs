use coordinate_systems::{Field, Robot};
use factrs::{core::SO3, traits::Variable, variables::SE23};
use linear_algebra::Isometry3;
use nalgebra::{Vector2, Vector3, vector};

use crate::camera_intrinsics::CameraIntrinsics;
use crate::conversions::robot_to_field_to_se23;

#[derive(Clone, Debug)]
pub struct InitialState {
    pub pose: SE23<f64>,
    pub camera_intrinsics: CameraIntrinsics<f64>,
}

impl InitialState {
    pub fn new(pose: SE23<f64>, camera_intrinsics: CameraIntrinsics<f64>) -> Self {
        Self {
            pose,
            camera_intrinsics,
        }
    }

    pub fn from_pose_and_intrinsics_components(
        pose: SE23<f64>,
        focal_lengths: Vector2<f64>,
        optical_center: Vector2<f64>,
    ) -> Self {
        Self::new(pose, CameraIntrinsics::new(focal_lengths, optical_center))
    }

    pub fn from_initial_height_and_intrinsics(
        initial_height: f64,
        camera_intrinsics: CameraIntrinsics<f64>,
    ) -> Self {
        let pose = SE23::from_rot_vel_trans(
            SO3::identity(),
            Vector3::zeros(),
            Vector3::new(0.0, 0.0, initial_height),
        );

        Self::new(pose, camera_intrinsics)
    }

    pub fn from_robot_to_field_and_intrinsics(
        robot_to_field: Isometry3<Robot, Field>,
        camera_intrinsics: CameraIntrinsics<f64>,
    ) -> Self {
        Self::new(robot_to_field_to_se23(robot_to_field), camera_intrinsics)
    }
}

impl Default for InitialState {
    fn default() -> Self {
        Self {
            pose: SE23::identity(),
            // Current default callers do not use visual factors yet. Use a valid
            // normalized pinhole calibration instead of a zeroed variable state.
            camera_intrinsics: CameraIntrinsics::new(vector![1.0, 1.0], vector![0.0, 0.0]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_non_degenerate_camera_intrinsics() {
        let initial_state = InitialState::default();

        assert_eq!(initial_state.camera_intrinsics.focals(), vector![1.0, 1.0]);
        assert_eq!(
            initial_state.camera_intrinsics.optical_center(),
            vector![0.0, 0.0]
        );
    }

    #[test]
    fn initial_height_constructor_sets_only_translation_z() {
        let initial_state = InitialState::from_initial_height_and_intrinsics(
            0.52,
            CameraIntrinsics::new(vector![1.0, 1.0], vector![0.0, 0.0]),
        );

        assert_eq!(initial_state.pose.xyz(), vector![0.0, 0.0, 0.52]);
        assert!(initial_state.pose.uvw().norm() < 1.0e-9);
    }

    #[test]
    fn robot_to_field_constructor_preserves_pose_and_intrinsics() {
        let rotation = linear_algebra::Orientation3::from_euler_angles(0.1, -0.2, 0.3);
        let initial_pose =
            Isometry3::from_parts(linear_algebra::vector![<Field>, 1.0, 2.0, 0.4], rotation);
        let camera_intrinsics = CameraIntrinsics::new(vector![200.0, 210.0], vector![250.0, 240.0]);

        let initial_state =
            InitialState::from_robot_to_field_and_intrinsics(initial_pose, camera_intrinsics);

        assert!((initial_state.pose.xyz() - vector![1.0, 2.0, 0.4]).norm() < 1.0e-6);
        assert!((initial_state.pose.uvw() - vector![0.0, 0.0, 0.0]).norm() < 1.0e-9);
        assert_eq!(
            initial_state.camera_intrinsics.focals(),
            vector![200.0, 210.0]
        );
        assert_eq!(
            initial_state.camera_intrinsics.optical_center(),
            vector![250.0, 240.0]
        );
        assert!((initial_state.pose.rot().w() - rotation.inner.w as f64).abs() < 1.0e-9);
        assert!((initial_state.pose.rot().x() - rotation.inner.i as f64).abs() < 1.0e-9);
        assert!((initial_state.pose.rot().y() - rotation.inner.j as f64).abs() < 1.0e-9);
        assert!((initial_state.pose.rot().z() - rotation.inner.k as f64).abs() < 1.0e-9);
    }
}

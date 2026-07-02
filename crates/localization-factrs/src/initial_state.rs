use factrs::{core::SO3, traits::Variable, variables::SE23};
use nalgebra::{Vector2, Vector3, vector};

use crate::camera_intrinsics::CameraIntrinsics;

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

    pub fn from_isometry_and_intrinsics(
        initial_pose: nalgebra::Isometry3<f64>,
        initial_velocity: Vector3<f64>,
        camera_intrinsics: CameraIntrinsics<f64>,
    ) -> Self {
        let rotation = initial_pose.rotation.quaternion();
        let pose = SE23::from_rot_vel_trans(
            SO3::from_xyzw(rotation.i, rotation.j, rotation.k, rotation.w),
            initial_velocity,
            initial_pose.translation.vector,
        );

        Self::new(pose, camera_intrinsics)
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
    fn isometry_constructor_preserves_pose_and_intrinsics() {
        let rotation = nalgebra::UnitQuaternion::from_euler_angles(0.1, -0.2, 0.3);
        let initial_pose =
            nalgebra::Isometry3::from_parts(nalgebra::Translation3::new(1.0, 2.0, 0.4), rotation);
        let camera_intrinsics = CameraIntrinsics::new(vector![200.0, 210.0], vector![250.0, 240.0]);

        let initial_state = InitialState::from_isometry_and_intrinsics(
            initial_pose,
            vector![0.5, 0.0, 0.0],
            camera_intrinsics,
        );

        assert_eq!(initial_state.pose.xyz(), vector![1.0, 2.0, 0.4]);
        assert_eq!(initial_state.pose.uvw(), vector![0.5, 0.0, 0.0]);
        assert_eq!(
            initial_state.camera_intrinsics.focals(),
            vector![200.0, 210.0]
        );
        assert_eq!(
            initial_state.camera_intrinsics.optical_center(),
            vector![250.0, 240.0]
        );
        assert!((initial_state.pose.rot().w() - rotation.w).abs() < 1.0e-9);
        assert!((initial_state.pose.rot().x() - rotation.i).abs() < 1.0e-9);
        assert!((initial_state.pose.rot().y() - rotation.j).abs() < 1.0e-9);
        assert!((initial_state.pose.rot().z() - rotation.k).abs() < 1.0e-9);
    }
}

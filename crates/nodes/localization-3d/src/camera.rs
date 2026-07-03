use std::{sync::Arc, time::Duration};

use coordinate_systems::{Camera, Robot};
use linear_algebra::{Isometry3, point};
use localization_factrs::CameraIntrinsics;
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{cache::Cache, time::Time};
use types::time_wrapper::TimeWrapper;

const MAX_CAMERA_MATRIX_TIME_DISTANCE: Duration = Duration::from_millis(100);

pub(crate) fn fresh_camera_matrix(
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    time: Time,
) -> Option<Arc<TimeWrapper<CameraMatrix>>> {
    let camera_matrix = camera_matrix_cache.get_nearest(time)?;
    camera_matrix_is_fresh(&camera_matrix, time).then_some(camera_matrix)
}

fn camera_matrix_is_fresh(camera_matrix: &TimeWrapper<CameraMatrix>, time: Time) -> bool {
    time_distance(camera_matrix.time, time) <= MAX_CAMERA_MATRIX_TIME_DISTANCE
}

fn time_distance(a: Time, b: Time) -> Duration {
    Duration::from_nanos(a.as_nanos().abs_diff(b.as_nanos()))
}

/// Converts ROS-Z camera-matrix intrinsics into the optimizer camera-intrinsics type.
pub fn camera_intrinsics_from_matrix(camera_matrix: &CameraMatrix) -> CameraIntrinsics {
    CameraIntrinsics::new(
        nalgebra::vector![
            camera_matrix.intrinsics.focals.x as f64,
            camera_matrix.intrinsics.focals.y as f64,
        ],
        nalgebra::vector![
            camera_matrix.intrinsics.optical_center.x() as f64,
            camera_matrix.intrinsics.optical_center.y() as f64,
        ],
    )
}

/// Converts optimizer camera intrinsics back into the projection crate's intrinsic model.
pub fn intrinsic_from_camera_intrinsics(camera_intrinsics: &CameraIntrinsics) -> Intrinsic {
    let focals = camera_intrinsics.focals();
    let optical_center = camera_intrinsics.optical_center();
    Intrinsic::new(
        nalgebra::vector![focals.x as f32, focals.y as f32],
        point![optical_center.x as f32, optical_center.y as f32],
    )
}

pub(crate) fn robot_to_camera(camera_matrix: &CameraMatrix) -> Isometry3<Robot, Camera> {
    camera_matrix.head_to_camera * camera_matrix.robot_to_head
}

#[cfg(test)]
mod tests {
    use coordinate_systems::Head;
    use linear_algebra::IntoTransform;

    use super::*;

    #[test]
    fn intrinsic_from_camera_intrinsics_casts_solver_intrinsics() {
        let camera_intrinsics = CameraIntrinsics::new(
            nalgebra::vector![216.5, 217.5],
            nalgebra::vector![251.25, 235.75],
        );

        let intrinsic = intrinsic_from_camera_intrinsics(&camera_intrinsics);

        assert_eq!(intrinsic.focals, nalgebra::vector![216.5, 217.5]);
        assert_eq!(intrinsic.optical_center, point![251.25, 235.75]);
    }

    #[test]
    fn visual_odometry_extrinsic_uses_head_and_camera_transforms() {
        let robot_to_head: Isometry3<Robot, Head> =
            nalgebra::Isometry3::translation(1.0, 2.0, 3.0).framed_transform();
        let head_to_camera: Isometry3<Head, Camera> =
            nalgebra::Isometry3::translation(0.5, 0.0, -0.25).framed_transform();
        let camera_matrix = CameraMatrix {
            robot_to_head,
            head_to_camera,
            ..Default::default()
        };

        let robot_to_camera = robot_to_camera(&camera_matrix).inner;
        let expected = (head_to_camera * robot_to_head).inner;

        assert!((robot_to_camera.translation.vector - expected.translation.vector).norm() < 1.0e-6);
        assert!(robot_to_camera.rotation.angle_to(&expected.rotation) < 1.0e-6);
    }
}

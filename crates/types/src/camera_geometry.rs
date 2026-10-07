use std::time::Duration;

use coordinate_systems::{Camera, Robot};
use linear_algebra::Isometry3;
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{cache::Cache, time::Time};
use serde::{Deserialize, Serialize};

use crate::time_wrapper::TimeWrapper;

/// Calibrated camera geometry relative to the body, independent of ground contact.
/// Published with the kinematics timestamp, not the publication time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct CameraGeometry {
    pub robot_to_camera: Isometry3<Robot, Camera>,
    pub intrinsics: Intrinsic,
}

impl From<&CameraMatrix> for CameraGeometry {
    fn from(camera: &CameraMatrix) -> Self {
        Self {
            robot_to_camera: camera.head_to_camera * camera.robot_to_head,
            intrinsics: camera.intrinsics,
        }
    }
}

pub fn camera_geometry_at(
    cameras: &Cache<TimeWrapper<CameraGeometry>>,
    time: Time,
    max_gap: Duration,
) -> Option<CameraGeometry> {
    let before = cameras.get_before(time)?;
    let after = cameras.get_after(time)?;
    interpolate_camera_geometry(&before, &after, time, max_gap)
}

/// Interpolate a validated source-time bracket without extrapolating head motion.
pub fn interpolate_camera_geometry(
    before: &TimeWrapper<CameraGeometry>,
    after: &TimeWrapper<CameraGeometry>,
    time: Time,
    max_gap: Duration,
) -> Option<CameraGeometry> {
    if time < before.time || time > after.time {
        return None;
    }
    let gap = after.time.duration_since(before.time);
    if gap > max_gap || before.inner.intrinsics != after.inner.intrinsics {
        return None;
    }
    for camera in [&before.inner, &after.inner] {
        if !camera.intrinsics.is_valid()
            || !camera
                .robot_to_camera
                .inner
                .to_homogeneous()
                .iter()
                .all(|v| v.is_finite())
        {
            return None;
        }
    }
    if gap.is_zero() {
        return Some(before.inner);
    }
    let fraction = time.duration_since(before.time).as_secs_f32() / gap.as_secs_f32();
    // Interpolate camera-to-body poses so rotation follows the physical camera origin.
    let a = before.inner.robot_to_camera.inner.inverse();
    let b = after.inner.robot_to_camera.inner.inverse();
    let mut rotation = a.rotation.slerp(&b.rotation, fraction);
    rotation.renormalize();
    let pose = nalgebra::Isometry3::from_parts(
        a.translation
            .vector
            .lerp(&b.translation.vector, fraction)
            .into(),
        rotation,
    );
    Some(CameraGeometry {
        robot_to_camera: Isometry3::wrap(pose.inverse()),
        intrinsics: before.inner.intrinsics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_sweep_interpolates_and_cancels_stationary_body_motion() {
        let sample = |nanos, angle| TimeWrapper {
            time: Time::from_nanos(nanos),
            inner: CameraGeometry {
                robot_to_camera: Isometry3::wrap(
                    nalgebra::Isometry3::from_parts(
                        nalgebra::Translation3::new(0.05, 0.0, 0.25),
                        nalgebra::UnitQuaternion::from_euler_angles(0.0, angle, 0.0),
                    )
                    .inverse(),
                ),
                ..Default::default()
            },
        };
        let before = sample(0, 0.0);
        let after = sample(10_000_000, 0.2);
        let max_gap = Duration::from_millis(20);
        let middle =
            interpolate_camera_geometry(&before, &after, Time::from_nanos(5_000_000), max_gap)
                .unwrap();
        let expected = sample(5_000_000, 0.1);
        assert!(
            (middle.robot_to_camera.inner.to_homogeneous()
                - expected.inner.robot_to_camera.inner.to_homogeneous())
            .norm()
                < 1e-6,
            "actual: {:?}, expected: {:?}",
            middle,
            expected.inner
        );
        let camera_delta =
            before.inner.robot_to_camera.inner * middle.robot_to_camera.inner.inverse();
        let body_delta = before.inner.robot_to_camera.inner.inverse()
            * camera_delta
            * middle.robot_to_camera.inner;
        assert!((body_delta.to_homogeneous() - nalgebra::Matrix4::identity()).norm() < 1e-6);
        assert!(
            interpolate_camera_geometry(&before, &after, Time::from_nanos(11_000_000), max_gap)
                .is_none()
        );
        assert!(
            interpolate_camera_geometry(
                &before,
                &sample(30_000_000, 0.2),
                Time::from_nanos(5_000_000),
                max_gap,
            )
            .is_none()
        );
        assert_eq!(
            interpolate_camera_geometry(&before, &before, before.time, max_gap),
            Some(before.inner)
        );
        let mut changed = after.clone();
        changed.inner.intrinsics.focals.x += 1.0;
        assert!(
            interpolate_camera_geometry(&before, &changed, Time::from_nanos(5_000_000), max_gap)
                .is_none()
        );
        assert!(
            interpolate_camera_geometry(
                &before,
                &after,
                Time::from_nanos(5_000_000),
                Duration::from_millis(9),
            )
            .is_none()
        );
        assert!(
            interpolate_camera_geometry(
                &before,
                &sample(30_000_000, 0.2),
                Time::from_nanos(5_000_000),
                Duration::from_millis(30),
            )
            .is_some()
        );
    }
}

use std::collections::VecDeque;

use booster::ImuState;
use coordinate_systems::{Field, Odometry, Robot};
use linear_algebra::{IntoTransform, Isometry3, Pose2};
use localization_factrs::OptimizationResult;
use ros_z::{cache::Cache, time::Time};
use types::odometry;

const ODOMETRY_HISTORY_CAPACITY: usize = 128;

pub(crate) type ImuStateCache = Cache<ImuState>;

#[derive(Clone, Copy, Debug)]
pub(crate) struct TimedOdometry {
    pub(crate) time: Time,
    pub(crate) pose: Pose2<Odometry>,
}

#[derive(Clone, Debug)]
pub(crate) struct TimedOdometryHistory {
    samples: VecDeque<TimedOdometry>,
}

#[derive(Default)]
pub(crate) struct LiveOdometryLocalization {
    anchor: Option<LiveOdometryAnchor>,
    pending_result: Option<OptimizationResult>,
}

struct LiveOdometryAnchor {
    time: Time,
    robot_to_field: nalgebra::Isometry3<f64>,
    odometry: Pose2<Odometry>,
    imu_orientation: nalgebra::UnitQuaternion<f64>,
}

impl Default for TimedOdometryHistory {
    fn default() -> Self {
        Self {
            samples: VecDeque::with_capacity(ODOMETRY_HISTORY_CAPACITY),
        }
    }
}

impl TimedOdometryHistory {
    pub(crate) fn clear(&mut self) {
        self.samples.clear();
    }

    pub(crate) fn push(&mut self, odometry: TimedOdometry) {
        if self
            .samples
            .back()
            .is_some_and(|latest| odometry.time <= latest.time)
        {
            return;
        }

        self.samples.push_back(odometry);
        while self.samples.len() > ODOMETRY_HISTORY_CAPACITY {
            self.samples.pop_front();
        }
    }

    pub(crate) fn latest(&self) -> Option<TimedOdometry> {
        self.samples.back().copied()
    }

    pub(crate) fn at(&self, time: Time) -> Option<TimedOdometry> {
        let before = self
            .samples
            .iter()
            .rev()
            .find(|odometry| odometry.time <= time)?;
        if before.time == time {
            return Some(*before);
        }

        let after = self.samples.iter().find(|odometry| odometry.time >= time)?;
        interpolate_odometry_samples(before, after, time)
    }
}

impl LiveOdometryLocalization {
    pub(crate) fn clear(&mut self) {
        self.anchor = None;
        self.pending_result = None;
    }

    pub(crate) fn reset(
        &mut self,
        result: &OptimizationResult,
        odometry_history: &TimedOdometryHistory,
        imu_cache: &ImuStateCache,
    ) {
        self.pending_result = Some(result.clone());
        self.anchor = None;
        self.try_reset_pending(odometry_history, imu_cache);
    }

    pub(crate) fn try_reset_pending(
        &mut self,
        odometry_history: &TimedOdometryHistory,
        imu_cache: &ImuStateCache,
    ) {
        let Some(result) = self.pending_result.as_ref() else {
            return;
        };
        let time = Time::from_wallclock(result.time);
        if let Some(anchor) = live_odometry_anchor(result, odometry_history, imu_cache, time) {
            self.anchor = Some(anchor);
            self.pending_result = None;
        }
    }

    pub(crate) fn field_to_robot_latest(
        &mut self,
        odometry_history: &TimedOdometryHistory,
        imu_cache: &ImuStateCache,
    ) -> Option<Isometry3<Field, Robot>> {
        let latest = odometry_history.latest()?;
        self.update_from_odometry(latest, imu_cache)
    }

    pub(crate) fn update_from_odometry(
        &mut self,
        current_odometry: TimedOdometry,
        imu_cache: &ImuStateCache,
    ) -> Option<Isometry3<Field, Robot>> {
        let anchor = self.anchor.as_ref()?;
        if current_odometry.time <= anchor.time {
            return Some(field_to_robot_from_robot_to_field(&anchor.robot_to_field));
        }

        let current_imu_orientation = imu_orientation_at(imu_cache, current_odometry.time)?;
        let current_robot_to_field =
            live_robot_to_field(anchor, current_odometry.pose, current_imu_orientation);

        Some(field_to_robot_from_robot_to_field(&current_robot_to_field))
    }
}

fn live_odometry_anchor(
    result: &OptimizationResult,
    odometry_history: &TimedOdometryHistory,
    imu_cache: &ImuStateCache,
    time: Time,
) -> Option<LiveOdometryAnchor> {
    Some(LiveOdometryAnchor {
        time,
        robot_to_field: result.transform,
        odometry: odometry_history.at(time)?.pose,
        imu_orientation: imu_orientation_at(imu_cache, time)?,
    })
}

fn live_robot_to_field(
    anchor: &LiveOdometryAnchor,
    current_odometry: Pose2<Odometry>,
    current_imu_orientation: nalgebra::UnitQuaternion<f64>,
) -> nalgebra::Isometry3<f64> {
    let current_to_anchor_odometry =
        odometry::current_to_previous(anchor.odometry, current_odometry);
    let odometry_delta = current_to_anchor_odometry
        .inner
        .translation
        .vector
        .cast::<f64>();
    let (_, _, anchor_yaw) = anchor.robot_to_field.rotation.euler_angles();
    let field_delta = nalgebra::Rotation2::new(anchor_yaw) * odometry_delta;

    let mut translation = anchor.robot_to_field.translation.vector;
    translation.x += field_delta.x;
    translation.y += field_delta.y;

    let rotation =
        anchor.robot_to_field.rotation * anchor.imu_orientation.inverse() * current_imu_orientation;

    nalgebra::Isometry3::from_parts(nalgebra::Translation3::from(translation), rotation)
}

fn field_to_robot_from_robot_to_field(
    robot_to_field: &nalgebra::Isometry3<f64>,
) -> Isometry3<Field, Robot> {
    robot_to_field.cast::<f32>().inverse().framed_transform()
}

fn imu_orientation_at(
    imu_cache: &ImuStateCache,
    time: Time,
) -> Option<nalgebra::UnitQuaternion<f64>> {
    imu_cache
        .get_nearest(time)
        .map(|imu| orientation_from_imu(imu.as_ref()))
}

fn orientation_from_imu(imu: &ImuState) -> nalgebra::UnitQuaternion<f64> {
    let roll_pitch_yaw = imu.roll_pitch_yaw.inner.cast::<f64>();
    nalgebra::UnitQuaternion::from_euler_angles(
        roll_pitch_yaw.x,
        roll_pitch_yaw.y,
        roll_pitch_yaw.z,
    )
}

fn interpolate_odometry_samples(
    before: &TimedOdometry,
    after: &TimedOdometry,
    time: Time,
) -> Option<TimedOdometry> {
    if after.time <= before.time {
        return Some(*before);
    }

    let total = after.time.duration_since(before.time).as_secs_f64();
    let elapsed = time.duration_since(before.time).as_secs_f64();
    let interpolation = (elapsed / total).clamp(0.0, 1.0) as f32;

    Some(TimedOdometry {
        time,
        pose: interpolate_odometry_pose(before.pose, after.pose, interpolation),
    })
}

fn interpolate_odometry_pose(
    start: Pose2<Odometry>,
    end: Pose2<Odometry>,
    interpolation: f32,
) -> Pose2<Odometry> {
    let translation = start.inner.translation.vector * (1.0 - interpolation)
        + end.inner.translation.vector * interpolation;
    let yaw = start.angle() + normalize_angle(end.angle() - start.angle()) * interpolation;

    Pose2::wrap(nalgebra::Isometry2::new(translation, yaw))
}

fn normalize_angle(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

#[cfg(test)]
mod tests {
    use linear_algebra::point;

    use super::*;

    fn timed_odometry(time_ns: i64, x: f32, y: f32, yaw: f32) -> TimedOdometry {
        TimedOdometry {
            time: Time::from_nanos(time_ns),
            pose: Pose2::<Odometry>::new(point![<Odometry>, x, y], yaw),
        }
    }

    #[test]
    fn odometry_history_interpolates_samples() {
        let mut history = TimedOdometryHistory::default();
        history.push(timed_odometry(1_000_000_000, 0.0, 0.0, 0.0));
        history.push(timed_odometry(2_000_000_000, 2.0, 4.0, 1.0));

        let interpolated = history
            .at(Time::from_nanos(1_500_000_000))
            .expect("sample should interpolate between neighbors");

        assert!((interpolated.pose.position().x() - 1.0).abs() < 1.0e-6);
        assert!((interpolated.pose.position().y() - 2.0).abs() < 1.0e-6);
        assert!((interpolated.pose.angle() - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn live_pose_uses_odometry_xy_anchored_z_and_live_imu_orientation() {
        let anchor_imu_orientation = nalgebra::UnitQuaternion::from_euler_angles(0.01, 0.02, 0.3);
        let anchor_robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(10.0, 20.0, 0.5),
            nalgebra::UnitQuaternion::from_euler_angles(0.1, -0.2, 0.5),
        );
        let anchor = LiveOdometryAnchor {
            time: Time::from_nanos(1_000_000_000),
            robot_to_field: anchor_robot_to_field,
            odometry: timed_odometry(1_000_000_000, 0.0, 0.0, 0.0).pose,
            imu_orientation: anchor_imu_orientation,
        };
        let current_odometry = timed_odometry(2_000_000_000, 1.0, 2.0, 1.2).pose;
        let live_imu_delta = nalgebra::UnitQuaternion::from_euler_angles(0.05, -0.03, 0.4);
        let current_imu_orientation = anchor_imu_orientation * live_imu_delta;

        let live_robot_to_field =
            live_robot_to_field(&anchor, current_odometry, current_imu_orientation);
        let expected_field_delta = nalgebra::Rotation2::new(0.5) * nalgebra::vector![1.0, 2.0];
        let expected_rotation = anchor_robot_to_field.rotation * live_imu_delta;

        assert!(
            (live_robot_to_field.translation.vector.x - (10.0 + expected_field_delta.x)).abs()
                < 1.0e-6
        );
        assert!(
            (live_robot_to_field.translation.vector.y - (20.0 + expected_field_delta.y)).abs()
                < 1.0e-6
        );
        assert!((live_robot_to_field.translation.vector.z - 0.5).abs() < 1.0e-6);
        assert!(live_robot_to_field.rotation.angle_to(&expected_rotation) < 1.0e-6);
    }
}

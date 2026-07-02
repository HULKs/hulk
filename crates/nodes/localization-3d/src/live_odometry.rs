use coordinate_systems::{Field, Robot};
use linear_algebra::Isometry3;
use localization_factrs::OptimizationResult;
use projection::camera_matrix::CameraMatrix;
use ros_z::{cache::Cache, time::Time};
use types::{time_wrapper::TimeWrapper, visual_odometry::VisualOdometer};

use crate::{
    camera::{fresh_camera_matrix, robot_to_camera},
    pose::localization_transform_constrained_to_ground,
};

pub(crate) type VisualOdometerCache = Cache<VisualOdometer>;

#[derive(Default)]
pub(crate) struct LiveVisualOdometryLocalization {
    anchor: Option<LiveVisualOdometryAnchor>,
    pending_result: Option<OptimizationResult>,
}

struct LiveVisualOdometryAnchor {
    time: Time,
    odometer_epoch: u64,
    robot_to_field: nalgebra::Isometry3<f64>,
    left_camera_to_visual_odometer: nalgebra::Isometry3<f32>,
    robot_to_camera: nalgebra::Isometry3<f32>,
}

impl LiveVisualOdometryLocalization {
    pub(crate) fn clear(&mut self) {
        self.anchor = None;
        self.pending_result = None;
    }

    pub(crate) fn reset(
        &mut self,
        result: &OptimizationResult,
        visual_odometer_cache: &VisualOdometerCache,
        camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    ) {
        self.pending_result = Some(result.clone());
        self.anchor = None;
        self.try_reset_pending(visual_odometer_cache, camera_matrix_cache);
    }

    pub(crate) fn try_reset_pending(
        &mut self,
        visual_odometer_cache: &VisualOdometerCache,
        camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    ) {
        let Some(result) = self.pending_result.as_ref() else {
            return;
        };
        let time = Time::from_wallclock(result.time);
        if let Some(anchor) =
            live_visual_odometry_anchor(result, visual_odometer_cache, camera_matrix_cache, time)
        {
            self.anchor = Some(anchor);
            self.pending_result = None;
        }
    }

    pub(crate) fn field_to_robot_latest(
        &mut self,
        visual_odometer_cache: &VisualOdometerCache,
        camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    ) -> Option<Isometry3<Field, Robot>> {
        let latest = visual_odometer_cache.get_latest()?;
        self.update_from_odometer(&latest, camera_matrix_cache)
    }

    pub(crate) fn update_from_odometer(
        &mut self,
        current_odometer: &VisualOdometer,
        camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    ) -> Option<Isometry3<Field, Robot>> {
        if current_odometer.epoch != self.anchor.as_ref()?.odometer_epoch {
            self.anchor = None;
            return None;
        }
        let anchor = self.anchor.as_ref()?;
        if current_odometer.time <= anchor.time {
            let current_camera_matrix =
                fresh_camera_matrix(camera_matrix_cache, current_odometer.time)?;
            return Some(localization_transform_constrained_to_ground(
                &anchor.robot_to_field,
                &current_camera_matrix.inner.ground_to_robot,
            ));
        }

        let current_camera_matrix =
            fresh_camera_matrix(camera_matrix_cache, current_odometer.time)?;
        let current_robot_to_camera = robot_to_camera(&current_camera_matrix.inner).inner;
        let current_camera_to_anchor_camera = anchor.left_camera_to_visual_odometer.inverse()
            * current_odometer.current_left_camera_to_visual_odometer;
        let current_robot_to_anchor_robot = anchor.robot_to_camera.inverse()
            * current_camera_to_anchor_camera
            * current_robot_to_camera;
        let current_robot_to_field = anchor.robot_to_field * current_robot_to_anchor_robot.cast();

        Some(localization_transform_constrained_to_ground(
            &current_robot_to_field,
            &current_camera_matrix.inner.ground_to_robot,
        ))
    }
}

fn live_visual_odometry_anchor(
    result: &OptimizationResult,
    visual_odometer_cache: &VisualOdometerCache,
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    time: Time,
) -> Option<LiveVisualOdometryAnchor> {
    let left_camera_to_visual_odometer = odometer_at(visual_odometer_cache, time)?;
    let camera_matrix = fresh_camera_matrix(camera_matrix_cache, time)?;
    Some(LiveVisualOdometryAnchor {
        time,
        odometer_epoch: left_camera_to_visual_odometer.epoch,
        robot_to_field: result.transform,
        left_camera_to_visual_odometer: left_camera_to_visual_odometer
            .current_left_camera_to_visual_odometer,
        robot_to_camera: robot_to_camera(&camera_matrix.inner).inner,
    })
}

fn odometer_at(visual_odometer_cache: &VisualOdometerCache, time: Time) -> Option<VisualOdometer> {
    if let Some((stamp, exact)) = visual_odometer_cache.get_nearest_with_stamp(time)
        && stamp == time
    {
        return Some(exact.as_ref().clone());
    }

    let before = visual_odometer_cache.get_before(time)?;
    let after = visual_odometer_cache.get_after(time)?;
    interpolate_odometer_samples(&before, &after, time)
}

fn interpolate_odometer_samples(
    before: &VisualOdometer,
    after: &VisualOdometer,
    time: Time,
) -> Option<VisualOdometer> {
    if before.epoch != after.epoch {
        return None;
    }
    if after.time <= before.time {
        return Some(before.clone());
    }

    let total = after.time.duration_since(before.time).as_secs_f64();
    let elapsed = time.duration_since(before.time).as_secs_f64();
    let interpolation = (elapsed / total).clamp(0.0, 1.0) as f32;

    Some(VisualOdometer {
        time,
        epoch: before.epoch,
        current_left_camera_to_visual_odometer: interpolate_isometry(
            before.current_left_camera_to_visual_odometer,
            after.current_left_camera_to_visual_odometer,
            interpolation,
        ),
    })
}

fn interpolate_isometry(
    start: nalgebra::Isometry3<f32>,
    end: nalgebra::Isometry3<f32>,
    interpolation: f32,
) -> nalgebra::Isometry3<f32> {
    let translation =
        start.translation.vector * (1.0 - interpolation) + end.translation.vector * interpolation;
    let rotation = start.rotation.slerp(&end.rotation, interpolation);

    nalgebra::Isometry3::from_parts(nalgebra::Translation3::from(translation), rotation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odometer_interpolation_refuses_epoch_crossing() {
        let time = Time::from_nanos(1_500_000_000);
        let before = VisualOdometer {
            time: Time::from_nanos(1_000_000_000),
            epoch: 1,
            current_left_camera_to_visual_odometer: nalgebra::Isometry3::identity(),
        };
        let after = VisualOdometer {
            time: Time::from_nanos(2_000_000_000),
            epoch: 2,
            current_left_camera_to_visual_odometer: nalgebra::Isometry3::translation(2.0, 0.0, 0.0),
        };

        assert!(interpolate_odometer_samples(&before, &after, time).is_none());
    }

    #[test]
    fn odometer_interpolation_preserves_epoch() {
        let time = Time::from_nanos(1_500_000_000);
        let before = VisualOdometer {
            time: Time::from_nanos(1_000_000_000),
            epoch: 1,
            current_left_camera_to_visual_odometer: nalgebra::Isometry3::identity(),
        };
        let after = VisualOdometer {
            time: Time::from_nanos(2_000_000_000),
            epoch: 1,
            current_left_camera_to_visual_odometer: nalgebra::Isometry3::translation(2.0, 0.0, 0.0),
        };

        let interpolated = interpolate_odometer_samples(&before, &after, time)
            .expect("same-epoch samples can be interpolated");

        assert_eq!(interpolated.epoch, 1);
        assert!(
            (interpolated
                .current_left_camera_to_visual_odometer
                .translation
                .vector
                .x
                - 1.0)
                .abs()
                < 1.0e-6
        );
    }
}

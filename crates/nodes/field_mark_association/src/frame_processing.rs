use std::{
    future::ready,
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::{Result, eyre::Context as _};
use coordinate_systems::{Camera, ImuReference, Robot};
use linear_algebra::{Isometry3, Orientation3};
use projection::intrinsic::Intrinsic;
use ros_z::{
    cache::{Cache, CacheInner},
    parameter::NodeParameters,
    pubsub::Publisher,
    time::{Clock, Time},
};
use types::{
    camera_geometry::{CameraGeometry, camera_geometry_at},
    field_dimensions::FieldDimensions,
    localization::{LocalizationEstimate, LocalizationState, LocalizationStatus},
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
    visual_localization::{
        AssociationGeometry, GlobalLocalizationDebug, VisualAssociationSource,
        VisualLocalizationFrame,
    },
};

use crate::{
    api::{AssociationInput, associate_visual_features},
    parameters::FieldMarkAssociationParameters,
};

type DetectedObjects = TimeWrapper<Vec<Object<RobocupObjectLabel>>>;

pub(crate) struct DetectionProcessingContext<'a> {
    pub(crate) parameters: &'a NodeParameters<FieldMarkAssociationParameters>,
    pub(crate) camera_geometry_cache: &'a Cache<TimeWrapper<CameraGeometry>>,
    pub(crate) field_dimensions_cache: &'a Cache<FieldDimensions>,
    pub(crate) estimates: &'a Cache<LocalizationEstimate>,
    pub(crate) status: &'a Cache<LocalizationStatus>,
    pub(crate) attitudes: std::sync::Mutex<CacheInner<TimeWrapper<Orientation3<ImuReference>>>>,
    pub(crate) associations_publisher: Arc<Publisher<TimeWrapper<VisualLocalizationFrame>>>,
    pub(crate) global_localization_publisher: Arc<Publisher<Option<GlobalLocalizationDebug>>>,
    pub(crate) clock: &'a Clock,
}

impl DetectionProcessingContext<'_> {
    pub(crate) fn record_attitude(&self, time: Time, imu: &booster::ImuState) {
        let rpy = imu.roll_pitch_yaw.inner;
        if rpy.iter().all(|v| v.is_finite()) {
            self.attitudes.lock().unwrap().insert(
                time,
                TimeWrapper {
                    time,
                    inner: Orientation3::from_euler_angles(rpy.x, rpy.y, rpy.z),
                },
            );
        }
    }

    fn attitude_at(&self, time: Time, max_gap: Duration) -> Option<Orientation3<ImuReference>> {
        let attitudes = self.attitudes.lock().unwrap();
        let before = attitudes.get_before(time)?;
        let after = attitudes.get_after(time)?;
        interpolate_attitude(&before, &after, time, max_gap)
    }
}

struct PreparedDetectionFrame {
    image_time: Time,
    objects: Vec<Object<RobocupObjectLabel>>,
    robot_to_camera: Isometry3<Robot, Camera>,
    camera_intrinsic: Intrinsic,
    status: LocalizationStatus,
    tracking: Option<AssociationGeometry>,
    attitude: Option<Orientation3<ImuReference>>,
    field_dimensions: Arc<FieldDimensions>,
    parameters: Arc<FieldMarkAssociationParameters>,
}

pub(crate) fn keep_latest_detection(
    pending: &mut Option<DetectedObjects>,
    objects: DetectedObjects,
) {
    if pending
        .as_ref()
        .is_none_or(|previous| objects.time > previous.time)
    {
        *pending = Some(objects);
    }
}

// The node polls this future alongside recv; neither solving nor publishing suspends ingestion.
pub(crate) async fn process_detected_objects(
    objects: DetectedObjects,
    ctx: &DetectionProcessingContext<'_>,
) -> Result<()> {
    let Some(frame) = prepare_detection_frame(objects, ctx) else {
        return Ok(());
    };
    let started = Instant::now();
    let (frame, localization) = tokio::task::spawn_blocking(move || {
        let visual_features = crate::find_detected_visual_features(&frame.objects);
        let localization = associate_visual_features(
            AssociationInput {
                status: &frame.status,
                attitude: frame.attitude,
                tracking: frame.tracking.as_ref(),
                visual_features: &visual_features,
                robot_to_camera: frame.robot_to_camera,
                camera_intrinsic: frame.camera_intrinsic,
                field_dimensions: &frame.field_dimensions,
                time: frame.image_time,
            },
            &frame.parameters,
        );
        (frame, localization)
    })
    .await
    .wrap_err("field-mark association worker failed")?;

    // A blocking job cannot be cancelled: retain its slot until it returns, then discard late work.
    // Reuse the calibrated age limit for the wall-time budget, including blocking-pool queue time.
    let max_age = frame.parameters.max_pose_hint_age;
    if started.elapsed() > max_age || !frame_is_current(&frame, ctx, max_age, localization.source) {
        return Ok(());
    }
    if !localization.associations.is_empty() {
        let publisher = Arc::clone(&ctx.associations_publisher);
        let message = TimeWrapper {
            time: frame.image_time,
            inner: VisualLocalizationFrame {
                epoch: frame.status.epoch,
                generation: frame.status.generation,
                source: localization.source,
                robot_to_camera: frame.robot_to_camera,
                camera_intrinsic: frame.camera_intrinsic,
                associations: localization.associations,
            },
        };
        let runtime = tokio::runtime::Handle::current();
        // Await each blocking send before starting another: publication keeps the single frame
        // slot, but never occupies an ingestion worker. Cancellation cannot abort an active send.
        tokio::task::spawn_blocking(move || runtime.block_on(publisher.publish(&message)))
            .await
            .wrap_err("field-mark association publisher failed")??;
    }
    if started.elapsed() <= max_age && frame_is_current(&frame, ctx, max_age, localization.source) {
        let publisher = Arc::clone(&ctx.global_localization_publisher);
        let runtime = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            runtime.block_on(publisher.publish_if_subscribed(|| ready(localization.debug)))
        })
        .await
        .wrap_err("field-mark association debug publisher failed")??;
    }
    Ok(())
}

fn prepare_detection_frame(
    objects: DetectedObjects,
    ctx: &DetectionProcessingContext<'_>,
) -> Option<PreparedDetectionFrame> {
    let image_time = objects.time;
    let parameters = ctx.parameters.snapshot().typed.clone();
    let camera = camera_geometry_at(
        ctx.camera_geometry_cache,
        image_time,
        parameters.max_camera_gap,
    )?;
    let field_dimensions = ctx.field_dimensions_cache.get_latest()?;
    let status = ctx.status.get_latest()?;
    let (attitude, tracking) = if status.state == LocalizationState::Tracking {
        (
            None,
            tracking_geometry(image_time, ctx, &status, parameters.max_pose_hint_age),
        )
    } else {
        (ctx.attitude_at(image_time, parameters.max_imu_gap), None)
    };
    if attitude.is_none() && tracking.is_none() {
        return None;
    }
    let frame = PreparedDetectionFrame {
        image_time,
        objects: objects.inner,
        robot_to_camera: camera.robot_to_camera,
        camera_intrinsic: camera.intrinsics,
        status: *status,
        tracking,
        attitude,
        field_dimensions,
        parameters,
    };
    lifecycle_is_current(&frame, ctx, frame.parameters.max_pose_hint_age).then_some(frame)
}

fn tracking_geometry(
    image_time: Time,
    ctx: &DetectionProcessingContext<'_>,
    status: &LocalizationStatus,
    max_age: Duration,
) -> Option<AssociationGeometry> {
    let latest_estimate = ctx.estimates.get_latest()?;
    let latest = TimeWrapper {
        time: latest_estimate.time,
        inner: AssociationGeometry::from_estimate(&latest_estimate, status)?,
    };
    let history = ctx.estimates.get_interval(image_time - max_age, image_time);
    let snapshots = history.iter().rev().filter_map(|estimate| {
        Some(TimeWrapper {
            time: estimate.time,
            inner: AssociationGeometry::from_estimate(estimate, status)?,
        })
    });
    geometry_for_image(image_time, snapshots, &latest, max_age)
}

fn interpolate_attitude(
    before: &TimeWrapper<Orientation3<ImuReference>>,
    after: &TimeWrapper<Orientation3<ImuReference>>,
    time: Time,
    max_gap: Duration,
) -> Option<Orientation3<ImuReference>> {
    if time < before.time || time > after.time {
        return None;
    }
    let gap = after.time.duration_since(before.time);
    if gap > max_gap {
        return None;
    }
    let mut orientation = if gap.is_zero() {
        before.inner
    } else {
        before.inner.slerp(
            after.inner,
            time.duration_since(before.time).as_secs_f32() / gap.as_secs_f32(),
        )
    };
    orientation.inner.renormalize();
    Some(orientation)
}

fn geometry_for_image(
    image_time: Time,
    history: impl IntoIterator<Item = TimeWrapper<AssociationGeometry>>,
    latest: &TimeWrapper<AssociationGeometry>,
    max_age: Duration,
) -> Option<AssociationGeometry> {
    // Select complete snapshots; never attach a new state/covariance to an old branch's pose.
    // Include latest because the independently populated history may not contain it yet.
    std::iter::once(latest.clone())
        .chain(history)
        .filter(|geometry| {
            geometry.time <= image_time
                && geometry.time.abs_diff(image_time) <= max_age
                && same_lifecycle(&geometry.inner, &latest.inner)
        })
        .min_by_key(|geometry| (geometry.time.abs_diff(image_time), geometry.time))
        .map(|geometry| geometry.inner)
}

fn lifecycle_is_current(
    frame: &PreparedDetectionFrame,
    ctx: &DetectionProcessingContext<'_>,
    max_age: Duration,
) -> bool {
    // Both sources replace their immutable Arc on update, including runtime parameter reloads.
    if !Arc::ptr_eq(&frame.parameters, &ctx.parameters.snapshot().typed)
        || ctx
            .field_dimensions_cache
            .get_latest()
            .is_none_or(|dimensions| !Arc::ptr_eq(&frame.field_dimensions, &dimensions))
    {
        return false;
    }
    frame.image_time.abs_diff(ctx.clock.now()) <= max_age
        && ctx.status.get_latest().is_some_and(|status| {
            status.epoch == frame.status.epoch
                && status.generation == frame.status.generation
                && status.state == frame.status.state
                && status.time == frame.status.time
        })
}

fn frame_is_current(
    frame: &PreparedDetectionFrame,
    ctx: &DetectionProcessingContext<'_>,
    max_age: Duration,
    source: VisualAssociationSource,
) -> bool {
    if !lifecycle_is_current(frame, ctx, max_age) {
        return false;
    }
    // Global recovery needs a trusted heading and fresh exposure IMU, not a fresh
    // optimized pose: rejecting field estimates must not starve its own recovery.
    if source == VisualAssociationSource::Global {
        return ctx
            .status
            .get_latest()
            .is_some_and(|status| status.heading == frame.status.heading);
    }
    let Some(geometry) = frame.tracking.as_ref() else {
        return false;
    };
    let latest = ctx
        .estimates
        .get_latest()
        .zip(ctx.status.get_latest())
        .and_then(|(estimate, status)| {
            Some(TimeWrapper {
                time: estimate.time,
                inner: AssociationGeometry::from_estimate(&estimate, &status)?,
            })
        });
    result_is_current(
        frame.image_time,
        geometry,
        latest.as_ref(),
        ctx.clock.now(),
        max_age,
    )
}

fn result_is_current(
    image_time: Time,
    geometry: &AssociationGeometry,
    latest: Option<&TimeWrapper<AssociationGeometry>>,
    now: Time,
    max_age: Duration,
) -> bool {
    image_time.abs_diff(now) <= max_age
        && latest.is_some_and(|latest| {
            latest.time.abs_diff(now) <= max_age && same_lifecycle(geometry, &latest.inner)
        })
}

fn same_lifecycle(geometry: &AssociationGeometry, latest: &AssociationGeometry) -> bool {
    geometry.epoch == latest.epoch && geometry.generation == latest.generation
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::localization::PoseEstimate;

    #[test]
    fn snapshot_selection_uses_preceding_pose_from_the_same_generation() {
        let pose = PoseEstimate {
            pose: Isometry3::from_translation(1.0, 0.0, 0.0),
            covariance: nalgebra::SMatrix::identity(),
        };
        let previous = TimeWrapper {
            time: Time::from_nanos(90_000_000),
            inner: AssociationGeometry {
                epoch: 7,
                generation: 2,
                estimate: pose,
                last_successful_solve: Time::from_nanos(90_000_000),
            },
        };
        let mut future = previous.clone();
        future.time = Time::from_nanos(105_000_000);
        future.inner.estimate.pose.inner.translation.x = 2.0;
        future.inner.last_successful_solve = future.time;
        let image = Time::from_nanos(100_000_000);
        let age = Duration::from_millis(20);
        let selected = geometry_for_image(image, [previous.clone()], &future, age).unwrap();
        assert_eq!(selected.estimate.pose.translation().x(), 1.0);
        assert_eq!(selected.estimate, previous.inner.estimate);
        assert!(geometry_for_image(image, [], &future, age).is_none());
        future.inner.generation += 1;
        assert!(geometry_for_image(image, [previous], &future, age).is_none());
    }

    #[test]
    fn attitude_interpolation_requires_bracketing_samples() {
        let before = TimeWrapper {
            time: Time::from_nanos(0),
            inner: Orientation3::from_euler_angles(0.0, 0.0, 0.0),
        };
        let after = TimeWrapper {
            time: Time::from_nanos(10_000_000),
            inner: Orientation3::from_euler_angles(0.2, 0.0, 0.0),
        };
        let time = Time::from_nanos(5_000_000);
        let gap = Duration::from_millis(10);
        let attitude = interpolate_attitude(&before, &after, time, gap).unwrap();
        assert!((attitude.euler_angles().0 - 0.1).abs() < 1e-6);
        assert!(interpolate_attitude(&before, &after, Time::from_nanos(11_000_000), gap).is_none());
        assert!(
            interpolate_attitude(&before, &after, time, gap - Duration::from_nanos(1)).is_none()
        );
        let late = TimeWrapper {
            time: Time::from_nanos(30_000_000),
            ..after
        };
        assert!(interpolate_attitude(&before, &late, time, gap).is_none());
        assert!(interpolate_attitude(&before, &late, time, Duration::from_millis(30)).is_some());
    }

    #[test]
    fn pending_detection_keeps_only_the_newest_timestamp() {
        let mut pending = None;
        for nanos in [1, 3, 2, 4, 4] {
            keep_latest_detection(
                &mut pending,
                TimeWrapper {
                    time: Time::from_nanos(nanos),
                    inner: Vec::new(),
                },
            );
        }
        assert_eq!(pending.take().unwrap().time, Time::from_nanos(4));
    }

    #[test]
    fn newer_generation_with_older_pose_invalidates_tracking_and_supplies_coherent_geometry() {
        use ros_z::cache::CacheInner;

        let time = Time::from_nanos(1_000_000_000);
        let age = Duration::from_millis(250);
        let estimate = PoseEstimate {
            pose: Isometry3::identity(),
            covariance: nalgebra::SMatrix::identity(),
        };
        let tracking = TimeWrapper {
            time,
            inner: AssociationGeometry {
                generation: 0,
                epoch: 7,
                estimate,
                last_successful_solve: time,
            },
        };
        let mut replacement = tracking.clone();
        replacement.time = time - Duration::from_millis(100);
        replacement.inner.generation += 1;
        replacement.inner.estimate.covariance *= 2.0;
        replacement.inner.last_successful_solve = replacement.time;
        replacement.inner.estimate.pose = Isometry3::from_translation(1.0, 2.0, 0.0);

        let mut history = CacheInner::new(128);
        history.insert(tracking.time, tracking.clone());
        history.insert(replacement.time, replacement.clone());
        let mut publications = CacheInner::new(1);
        publications.insert(time, tracking.clone());
        publications.insert(time + Duration::from_millis(1), replacement.clone());
        let latest = publications.get_latest().unwrap();
        assert!(result_is_current(
            time,
            &tracking.inner,
            history.get_latest().as_deref(),
            time,
            age
        ));
        assert!(!result_is_current(
            time,
            &tracking.inner,
            Some(&latest),
            time,
            age
        ));

        let selected = geometry_for_image(
            time,
            history
                .get_interval(time - age, time + age)
                .iter()
                .rev()
                .map(|g| (**g).clone()),
            &latest,
            age,
        )
        .unwrap();
        assert_eq!(selected.estimate, replacement.inner.estimate);
        assert_eq!(selected.last_successful_solve, replacement.time);
        // Latest remains usable before the history subscriber receives the transition.
        let old_branch = [Arc::new(tracking)];
        assert_eq!(
            geometry_for_image(time, old_branch.iter().map(|g| (**g).clone()), &latest, age)
                .unwrap()
                .estimate,
            replacement.inner.estimate
        );
        replacement.time = time - age - Duration::from_nanos(1);
        assert!(
            geometry_for_image(
                time,
                old_branch.iter().map(|g| (**g).clone()),
                &replacement,
                age
            )
            .is_none()
        );
    }

    #[test]
    fn late_results_require_fresh_matching_epoch_and_explicit_state() {
        let time = Time::from_nanos(1_000_000_000);
        let age = Duration::from_millis(250);
        let estimate = LocalizationEstimate {
            time,
            epoch: 7,
            generation: 0,
            robot_to_local: PoseEstimate {
                pose: Isometry3::identity(),
                covariance: nalgebra::SMatrix::identity(),
            },
            robot_to_field: Some(PoseEstimate {
                pose: Isometry3::identity(),
                covariance: nalgebra::SMatrix::identity(),
            }),
        };
        let mut status = LocalizationStatus {
            time,
            epoch: 7,
            generation: 0,
            state: LocalizationState::Tracking,
            heading: None,
        };
        let geometry = AssociationGeometry::from_estimate(&estimate, &status).unwrap();
        let mut latest = TimeWrapper {
            time,
            inner: geometry.clone(),
        };
        let valid = |latest: Option<&TimeWrapper<AssociationGeometry>>, now| {
            result_is_current(time, &geometry, latest, now, age)
        };
        for state in [LocalizationState::Startup, LocalizationState::LostTrack] {
            status.state = state;
            let unavailable = AssociationGeometry::from_estimate(&estimate, &status)
                .map(|inner| TimeWrapper { time, inner });
            assert!(!valid(unavailable.as_ref(), time));
        }
        let expired = age + Duration::from_nanos(1);
        assert!(valid(Some(&latest), time + age));
        assert!(!valid(None, time));
        assert!(!valid(Some(&latest), time + expired));
        assert!(!valid(Some(&latest), time - expired));
        latest.time = time - expired;
        assert!(!valid(Some(&latest), time));
        latest.time = time;
        latest.inner.epoch += 1;
        assert!(!valid(Some(&latest), time));
        latest.inner.epoch = geometry.epoch;
        latest.inner.generation += 1;
        assert!(!valid(Some(&latest), time));
    }
}

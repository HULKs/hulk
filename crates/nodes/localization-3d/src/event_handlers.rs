use std::future::ready;

use color_eyre::{Result, eyre::Context as _};
use localization_factrs::VinsFrontend;
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{cache::Cache, pubsub::Publisher, time::Time};
use types::{
    time_wrapper::TimeWrapper,
    visual_localization::AssociationPoseHintSource,
    visual_odometry::{VisualOdometer, VisualOdometryDelta as VisualOdometryDeltaMessage},
};

use crate::{
    camera::{fresh_camera_matrix, intrinsic_from_camera_intrinsics},
    ingest::ingest_visual_odometry,
    live_odometry::LiveVisualOdometryLocalization,
    pose::backend_localization_for_result,
    publish::LocalizationPublishers,
    visual_localization::GlobalVisualLock,
};

pub(crate) fn handle_visual_odometry(
    frontend: &mut VinsFrontend,
    visual_odometry: VisualOdometryDeltaMessage,
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
) -> Result<()> {
    let Some(previous_camera_matrix) =
        fresh_camera_matrix(camera_matrix_cache, visual_odometry.previous_time)
    else {
        return Ok(());
    };
    let Some(current_camera_matrix) =
        fresh_camera_matrix(camera_matrix_cache, visual_odometry.current_time)
    else {
        return Ok(());
    };

    ingest_visual_odometry(
        frontend,
        visual_odometry,
        &previous_camera_matrix.inner,
        &current_camera_matrix.inner,
    )
    .wrap_err("failed to ingest visual odometry measurement into frontend")
}

pub(crate) async fn handle_visual_odometer(
    live_localization: &mut LiveVisualOdometryLocalization,
    global_visual_lock: GlobalVisualLock,
    visual_odometer: VisualOdometer,
    visual_odometer_cache: &Cache<VisualOdometer>,
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    publishers: LocalizationPublishers<'_>,
) -> Result<()> {
    if !global_visual_lock.has_backend_result() {
        return Ok(());
    }

    live_localization.try_reset_pending(visual_odometer_cache, camera_matrix_cache);
    if let Some(transform) =
        live_localization.update_from_odometer(&visual_odometer, camera_matrix_cache)
    {
        publishers
            .publish_outputs(
                visual_odometer.time,
                Some(transform),
                Some(transform),
                AssociationPoseHintSource::LiveVisualOdometry,
            )
            .await?;
    }
    Ok(())
}

pub(crate) async fn handle_optimization_result(
    frontend: &mut VinsFrontend,
    live_localization: &mut LiveVisualOdometryLocalization,
    global_visual_lock: &mut GlobalVisualLock,
    visual_odometer_cache: &Cache<VisualOdometer>,
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    publishers: LocalizationPublishers<'_>,
    calibrated_intrinsics_publisher: &Publisher<Intrinsic>,
) -> Result<()> {
    let Some(result) = frontend.last_optimization_result() else {
        return Ok(());
    };
    let result_time = Time::from_wallclock(result.time);
    let camera_matrix = fresh_camera_matrix(camera_matrix_cache, result_time);
    let backend_transform = backend_localization_for_result(&result, camera_matrix.as_deref());
    let (transform, pose_3d) = if global_visual_lock.mark_backend_result() {
        live_localization.reset(&result, visual_odometer_cache, camera_matrix_cache);
        (
            Some(
                live_localization
                    .field_to_robot_latest(visual_odometer_cache, camera_matrix_cache)
                    .unwrap_or(backend_transform),
            ),
            Some(backend_transform),
        )
    } else {
        (None, None)
    };

    publishers
        .publish_outputs(
            result_time,
            transform,
            pose_3d,
            AssociationPoseHintSource::BackendBranch,
        )
        .await?;
    calibrated_intrinsics_publisher
        .publish_if_subscribed(|| {
            ready(intrinsic_from_camera_intrinsics(&result.camera_intrinsics))
        })
        .await?;
    Ok(())
}

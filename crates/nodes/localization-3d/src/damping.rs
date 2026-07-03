use color_eyre::Result;
use localization_factrs::{InitialState, VinsFrontend, VinsFrontendError};
use projection::camera_matrix::CameraMatrix;
use ros_z::{cache::Cache, time::Time};
use types::{
    field_dimensions::FieldDimensions, primary_state::PrimaryState, time_wrapper::TimeWrapper,
};

use crate::{
    live_odometry::LiveVisualOdometryLocalization, pose::initial_state_from_camera_matrix,
    publish::LocalizationPublishers, visual_localization::GlobalVisualLock,
};

pub(crate) fn localization_is_damping(primary_state_cache: &Cache<PrimaryState>) -> bool {
    primary_state_cache
        .get_latest()
        .is_none_or(|state| *state == PrimaryState::Damping)
}

pub(crate) fn initial_state_for_reset(
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    field_dimensions: &FieldDimensions,
    fallback: &InitialState,
) -> InitialState {
    camera_matrix_cache
        .get_latest()
        .map(|camera_matrix| {
            initial_state_from_camera_matrix(&camera_matrix.inner, field_dimensions)
        })
        .unwrap_or_else(|| fallback.clone())
}

pub(crate) fn reset_localization_for_damping(
    frontend: &mut VinsFrontend,
    live_localization: &mut LiveVisualOdometryLocalization,
    global_visual_lock: &mut GlobalVisualLock,
    initial_state: InitialState,
    time: Time,
) -> Result<(), VinsFrontendError> {
    frontend.reset(time.to_wallclock(), initial_state)?;
    live_localization.clear();
    global_visual_lock.reset_for_damping();
    Ok(())
}

pub(crate) async fn reset_and_publish_startup_prior(
    frontend: &mut VinsFrontend,
    live_localization: &mut LiveVisualOdometryLocalization,
    global_visual_lock: &mut GlobalVisualLock,
    initial_state: InitialState,
    time: Time,
    field_dimensions: &FieldDimensions,
    publishers: LocalizationPublishers<'_>,
) -> Result<()> {
    reset_localization_for_damping(
        frontend,
        live_localization,
        global_visual_lock,
        initial_state,
        time,
    )?;
    publishers
        .publish_startup_prior(time, field_dimensions)
        .await
}

pub(crate) async fn publish_damping_optimization_result(
    frontend: &mut VinsFrontend,
    live_localization: &mut LiveVisualOdometryLocalization,
    global_visual_lock: &mut GlobalVisualLock,
    time: Time,
    field_dimensions: &FieldDimensions,
    publishers: LocalizationPublishers<'_>,
) -> Result<()> {
    let _ = frontend.last_optimization_result();
    live_localization.clear();
    global_visual_lock.reset_for_damping();
    publishers
        .publish_startup_prior(time, field_dimensions)
        .await
}

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use booster::ImuState;
use color_eyre::{
    Result,
    eyre::{Context as _, bail},
};
use coordinate_systems::{Field, Robot};
use linear_algebra::Isometry3;
use localization_factrs::{InitialState, initialize};
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{
    cache::Cache,
    context::Context,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosProfile},
};
use tokio::select;
use types::{
    field_dimensions::FieldDimensions,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    visual_localization::{
        ASSOCIATION_POSE_HINT_TOPIC, AssociationPoseHint, LOCALIZATION_POSE_3D_TOPIC,
        VISUAL_LOCALIZATION_TOPIC, VisualLocalizationFrame,
    },
    visual_odometry::{VisualOdometer, VisualOdometryDelta as VisualOdometryDeltaMessage},
};

use crate::{
    backend_task::spawn_backend_task,
    damping::{
        initial_state_for_reset, localization_is_damping, publish_damping_optimization_result,
        reset_and_publish_unlocalized,
    },
    diagnostics::SolveDiagnostics,
    event_handlers::{handle_optimization_result, handle_visual_odometer, handle_visual_odometry},
    ingest::ingest_foot_heights,
    live_odometry::LiveVisualOdometryLocalization,
    parameters::{
        Localization3dParameters, backend_configuration_from_parameters_and_field_dimensions,
    },
    pose::initial_state_from_camera_matrix,
    publish::LocalizationPublishers,
    visual_localization::{GlobalVisualLock, handle_visual_localization_frame},
};

const VISUAL_ODOMETER_TOPIC: &str = "visual_odometry/current_left_camera_to_visual_odometer";

/// Starts the localization node and erases the concrete future type for node runners.
///
/// `ctx` is the ROS-Z context used to create publishers, subscribers, caches, and parameters.
pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

/// Runs the asynchronous 3D localization node until its input streams terminate or fail.
///
/// The node consumes IMU, camera matrix, field-mark associations, visual odometry, and kinematics
/// topics, then publishes the optimized robot pose and debug streams.
pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("localization3d").build().await?;
    let parameters = node.bind_parameter_as::<Localization3dParameters>("localization3d")?;
    parameters.add_validation_hook(Localization3dParameters::validate)?;

    let imu_subscriber = node
        .subscriber::<ImuState>("inputs/imu_state")
        .build()
        .await?;

    let camera_matrix_cache = node
        .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .cache(128)
        .with_stamp(|message| message.time)
        .build()
        .await?;

    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let visual_localization_subscriber = node
        .subscriber::<TimeWrapper<VisualLocalizationFrame>>(VISUAL_LOCALIZATION_TOPIC)
        .build()
        .await?;

    let visual_odometry_subscriber = node
        .subscriber::<VisualOdometryDeltaMessage>(
            "visual_odometry/current_left_camera_to_previous_left_camera",
        )
        .build()
        .await?;
    let visual_odometer_cache = node
        .subscriber::<VisualOdometer>(VISUAL_ODOMETER_TOPIC)
        .cache(128)
        .with_stamp(|message| message.time)
        .build()
        .await?;
    let visual_odometer_subscriber = node
        .subscriber::<VisualOdometer>(VISUAL_ODOMETER_TOPIC)
        .build()
        .await?;

    let robot_kinematics_subscriber = node
        .subscriber::<TimeWrapper<kinematics::robot_kinematics::RobotKinematics>>(
            "robot_kinematics",
        )
        .build()
        .await?;

    let primary_state_cache = node
        .subscriber::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let localization_publisher = node
        .publisher::<Option<Isometry3<Field, Robot>>>("localization")
        .build()
        .await?;
    let pose_3d_publisher = node
        .publisher::<TimeWrapper<Option<Isometry3<Field, Robot>>>>(LOCALIZATION_POSE_3D_TOPIC)
        .build()
        .await?;
    let association_pose_hint_publisher = node
        .publisher::<TimeWrapper<Option<AssociationPoseHint>>>(ASSOCIATION_POSE_HINT_TOPIC)
        .build()
        .await?;
    let calibrated_intrinsics_publisher = node
        .publisher::<Intrinsic>("debug/calibrated_intrinsics")
        .build()
        .await?;
    let solve_diagnostics_publisher = node
        .publisher::<TimeWrapper<SolveDiagnostics>>("debug/solve_diagnostics")
        .build()
        .await?;

    let field_dimensions = wait_for_field_dimensions(&field_dimensions_cache).await;
    let initial_state = wait_for_initial_state(&camera_matrix_cache).await;
    let localization_parameters = parameters.snapshot().typed().clone();
    let (mut frontend, backend) = initialize(
        backend_configuration_from_parameters_and_field_dimensions(
            &localization_parameters,
            &field_dimensions,
        ),
        initial_state.clone(),
    );
    let mut backend_handle =
        std::pin::pin!(spawn_backend_task(backend, solve_diagnostics_publisher));
    let mut live_localization = LiveVisualOdometryLocalization::default();
    let mut global_visual_lock = GlobalVisualLock::Unlocked;
    let mut damping_reset_interval = tokio::time::interval(Duration::from_millis(100));
    let publishers = LocalizationPublishers::new(
        &localization_publisher,
        &pose_3d_publisher,
        &association_pose_hint_publisher,
    );

    loop {
        select! {
            _ = damping_reset_interval.tick() => {
                if !localization_is_damping(&primary_state_cache) {
                    continue;
                }

                let now = node.clock().now();
                reset_and_publish_unlocalized(
                    &mut frontend,
                    &mut live_localization,
                    &mut global_visual_lock,
                    initial_state_for_reset(&camera_matrix_cache, &initial_state),
                    now,
                    publishers,
                )
                .await
                .wrap_err("failed to reset localization while damping")?;
            }
            visual_localization = visual_localization_subscriber.recv() => {
                let visual_localization = visual_localization?;
                if should_drop_input_while_damping(&primary_state_cache) {
                    continue;
                }

                handle_visual_localization_frame(
                    &mut frontend,
                    &mut live_localization,
                    &mut global_visual_lock,
                    visual_localization,
                )
                .wrap_err("failed to ingest visual localization frame")?;
            }
            // IMU payloads have no sensor timestamp; ros-z source time is the aligned clock.
            imu = imu_subscriber.recv_with_metadata() => {
                let imu = imu?;
                if should_drop_input_while_damping(&primary_state_cache) {
                    continue;
                }
                frontend.ingest_imu(imu.source_time.to_wallclock(), imu.message)
                    .wrap_err("failed to ingest imu measurement into frontend")?;
            }
            visual_odometry = visual_odometry_subscriber.recv() => {
                let visual_odometry = visual_odometry?;
                if should_drop_input_while_damping(&primary_state_cache) {
                    continue;
                }
                handle_visual_odometry(&mut frontend, visual_odometry, &camera_matrix_cache)?;
            }
            visual_odometer = visual_odometer_subscriber.recv() => {
                let visual_odometer = visual_odometer?;
                if should_drop_input_while_damping(&primary_state_cache) {
                    continue;
                }
                handle_visual_odometer(
                    &mut live_localization,
                    global_visual_lock,
                    visual_odometer,
                    &visual_odometer_cache,
                    &camera_matrix_cache,
                    publishers,
                ).await?;
            }
            robot_kinematics = robot_kinematics_subscriber.recv() => {
                let robot_kinematics = robot_kinematics?;
                if should_drop_input_while_damping(&primary_state_cache) {
                    continue;
                }
                ingest_foot_heights(&mut frontend, robot_kinematics)
                    .wrap_err("failed to ingest foot height measurement into frontend")?;
            }
            result = &mut backend_handle => {
                result.wrap_err("failed to join")?.wrap_err("solver failed")?;
                bail!("solver stopped unexpectedly")
            }
            result = frontend.wait_for_optimization_result() => {
                result?;
                if localization_is_damping(&primary_state_cache) {
                    publish_damping_optimization_result(
                        &mut frontend,
                        &mut live_localization,
                        &mut global_visual_lock,
                        node.clock().now(),
                        publishers,
                    ).await?;
                    continue;
                }
                handle_optimization_result(
                    &mut frontend,
                    &mut live_localization,
                    &mut global_visual_lock,
                    &visual_odometer_cache,
                    &camera_matrix_cache,
                    publishers,
                    &calibrated_intrinsics_publisher,
                ).await?;
            }
        }
    }
}

fn should_drop_input_while_damping(primary_state_cache: &Cache<PrimaryState>) -> bool {
    localization_is_damping(primary_state_cache)
}

async fn wait_for_initial_state(
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
) -> InitialState {
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    loop {
        if let Some(camera_matrix) = camera_matrix_cache.get_latest() {
            return initial_state_from_camera_matrix(&camera_matrix.inner);
        }
        interval.tick().await;
    }
}

async fn wait_for_field_dimensions(
    field_dimensions_cache: &Cache<FieldDimensions>,
) -> FieldDimensions {
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    loop {
        if let Some(field_dimensions) = field_dimensions_cache.get_latest() {
            return *field_dimensions.as_ref();
        }
        interval.tick().await;
    }
}

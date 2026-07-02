use std::{
    f64::consts::FRAC_PI_2,
    future::{Future, ready},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use booster::ImuState;
use color_eyre::{
    Result,
    eyre::{Context as _, bail},
};
use coordinate_systems::{Camera, Field, Ground, Robot};
use field_mark_association::{FieldMarkAssociationKind, FieldMarkAssociations};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::{IntoTransform, Isometry2, Isometry3, point};
use localization_factrs::{
    BackendConfiguration, CameraIntrinsics, FieldContainmentConfiguration, InitialState,
    OptimizationResult, VinsFrontend, VinsFrontendError, VisualReprojectionAssociation,
    VisualReprojectionAssociationKind, initialize,
};
use nalgebra::{Matrix2, Matrix3, Point3, SMatrix, Vector3, vector};
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{
    Message,
    cache::Cache,
    context::Context,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosProfile},
    time::Time,
};
use serde::{Deserialize, Serialize};
use tokio::select;
use types::{
    field_dimensions::FieldDimensions,
    time_wrapper::TimeWrapper,
    visual_odometry::{VisualOdometer, VisualOdometryDelta as VisualOdometryDeltaMessage},
};

mod diagnostics;

pub use diagnostics::{SolveDiagnostics, SolveOptimizerStatus, SolveResidualDiagnostics};

/// Runtime parameters for the 3D localization node.
#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Localization3dParameters {
    /// Pixel residual variance for accepted visual feature associations.
    pub visual_feature_noise_variance: f64,
    /// Pixel residual variance for lower-trust pose-hint visual feature associations.
    pub pose_hint_visual_feature_noise_variance: f64,
    /// Huber threshold for pose-hint visual residuals in whitened residual units.
    pub pose_hint_visual_huber_threshold: f64,
    /// Soft field containment sigma in meters outside field plus border strip.
    pub field_containment_sigma: f64,
}

impl Default for Localization3dParameters {
    fn default() -> Self {
        Self {
            visual_feature_noise_variance: 10_000.0,
            pose_hint_visual_feature_noise_variance:
                DEFAULT_POSE_HINT_VISUAL_FEATURE_NOISE_VARIANCE,
            pose_hint_visual_huber_threshold: DEFAULT_POSE_HINT_VISUAL_HUBER_THRESHOLD,
            field_containment_sigma: DEFAULT_FIELD_CONTAINMENT_SIGMA,
        }
    }
}

impl Localization3dParameters {
    fn validate(&self) -> std::result::Result<(), String> {
        if !self.visual_feature_noise_variance.is_finite()
            || self.visual_feature_noise_variance <= 0.0
        {
            return Err("visual_feature_noise_variance must be finite and > 0".to_string());
        }
        if !self.pose_hint_visual_feature_noise_variance.is_finite()
            || self.pose_hint_visual_feature_noise_variance <= 0.0
        {
            return Err(
                "pose_hint_visual_feature_noise_variance must be finite and > 0".to_string(),
            );
        }
        if !self.pose_hint_visual_huber_threshold.is_finite()
            || self.pose_hint_visual_huber_threshold <= 0.0
        {
            return Err("pose_hint_visual_huber_threshold must be finite and > 0".to_string());
        }
        if !self.field_containment_sigma.is_finite() || self.field_containment_sigma <= 0.0 {
            return Err("field_containment_sigma must be finite and > 0".to_string());
        }
        Ok(())
    }
}

const MAX_CAMERA_MATRIX_TIME_DISTANCE: Duration = Duration::from_millis(100);
const VISUAL_ODOMETER_TOPIC: &str = "visual_odometry/current_left_camera_to_visual_odometer";
const DEFAULT_POSE_HINT_VISUAL_FEATURE_NOISE_VARIANCE: f64 = 100_000.0;
const DEFAULT_POSE_HINT_VISUAL_HUBER_THRESHOLD: f64 = 2.0;
const DEFAULT_FIELD_CONTAINMENT_SIGMA: f64 = 1.0;
const DEFAULT_INITIAL_ROBOT_HEIGHT: f64 = 0.52;

type VisualOdometerCache = Cache<VisualOdometer>;

/// Builds the VINS backend configuration used by the localization node.
///
/// `visual_feature_noise_variance` is the pixel-space variance assigned to globally certified
/// field-feature reprojection factors. Pose-hint fallback factors use conservative defaults here.
pub fn backend_configuration(visual_feature_noise_variance: f64) -> BackendConfiguration {
    backend_configuration_with_pose_hint(
        visual_feature_noise_variance,
        DEFAULT_POSE_HINT_VISUAL_FEATURE_NOISE_VARIANCE,
        DEFAULT_POSE_HINT_VISUAL_HUBER_THRESHOLD,
        DEFAULT_FIELD_CONTAINMENT_SIGMA,
        &FieldDimensions::SPL_2025,
    )
}

pub fn backend_configuration_from_parameters(
    parameters: &Localization3dParameters,
) -> BackendConfiguration {
    backend_configuration_from_parameters_and_field_dimensions(
        parameters,
        &FieldDimensions::SPL_2025,
    )
}

pub fn backend_configuration_from_parameters_and_field_dimensions(
    parameters: &Localization3dParameters,
    field_dimensions: &FieldDimensions,
) -> BackendConfiguration {
    backend_configuration_with_pose_hint(
        parameters.visual_feature_noise_variance,
        parameters.pose_hint_visual_feature_noise_variance,
        parameters.pose_hint_visual_huber_threshold,
        parameters.field_containment_sigma,
        field_dimensions,
    )
}

fn backend_configuration_with_pose_hint(
    visual_feature_noise_variance: f64,
    pose_hint_visual_feature_noise_variance: f64,
    pose_hint_visual_huber_threshold: f64,
    field_containment_sigma: f64,
    field_dimensions: &FieldDimensions,
) -> BackendConfiguration {
    let process_noise = Matrix3::identity() * 0.01;
    BackendConfiguration {
        knot_spacing: Duration::from_millis(200),
        max_optimization_window: Duration::from_secs(2),
        optimizer_max_iterations: 2,
        gyroscope_noise: Matrix3::identity() * 0.1_f64.powi(2),
        // TODO: tune accelerometer noise
        accelerometer_noise: Matrix3::from_diagonal(&vector![
            5.0_f64.powi(2),   // x sigma = 5 m/s^2
            5.0_f64.powi(2),   // y sigma = 5 m/s^2
            100.0_f64.powi(2), // z disabled
        ]),
        use_accelerometer_measurements: false,
        gyroscope_process_noise: process_noise,
        roll_pitch_yaw_noise: Matrix3::from_diagonal(&Vector3::new(0.01, 0.01, 0.00001)),
        accelerometer_process_noise: process_noise,
        visual_feature_noise: Matrix2::identity() * visual_feature_noise_variance,
        pose_hint_visual_feature_noise: Matrix2::identity()
            * pose_hint_visual_feature_noise_variance,
        pose_hint_visual_huber_threshold,
        // factrs::SE3 tangent order is [rot_x, rot_y, rot_z, trans_x, trans_y, trans_z].
        visual_odometry_noise: SMatrix::<f64, 6, 6>::identity() * 1.0e-2,
        foot_ground_sigma: 1e-2,
        field_containment: FieldContainmentConfiguration::from_field_dimensions(
            field_dimensions,
            field_containment_sigma,
        ),
        gravity: Vector3::new(0.0, 0.0, 9.81),
    }
}

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

    let field_mark_associations_subscriber = node
        .subscriber::<TimeWrapper<FieldMarkAssociations>>("field_mark_association/associations")
        .build()
        .await?;
    let global_pose_subscriber = node
        .subscriber::<TimeWrapper<Option<Isometry3<Robot, Field>>>>(
            "field_mark_association/global_pose",
        )
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
        .subscriber::<TimeWrapper<RobotKinematics>>("robot_kinematics")
        .build()
        .await?;

    let localization_publisher = node
        .publisher::<Option<Isometry3<Field, Robot>>>("localization")
        .build()
        .await?;
    let ground_to_field_publisher = node
        .publisher::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await?;
    let timestamped_localization_publisher = node
        .publisher::<TimeWrapper<Option<Isometry3<Field, Robot>>>>("localization/timestamped")
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
    let initial_state = wait_for_initial_state(&camera_matrix_cache, &field_dimensions).await;
    let localization_parameters = parameters.snapshot().typed().clone();
    let (mut frontend, backend) = initialize(
        backend_configuration_from_parameters_and_field_dimensions(
            &localization_parameters,
            &field_dimensions,
        ),
        initial_state,
    );
    let runtime = tokio::runtime::Handle::current();
    let mut backend_handle = std::pin::pin!(tokio::task::spawn_blocking(move || -> Result<()> {
        let mut backend = backend;
        loop {
            let Some(result) = backend.solve_next_blocking()? else {
                continue;
            };

            runtime
                .block_on(solve_diagnostics_publisher.publish_if_subscribed(|| {
                    let diagnostics = backend
                        .compute_last_solve_diagnostics()
                        .expect("diagnostics are available after a successful solve");

                    ready(TimeWrapper {
                        time: Time::from_wallclock(result.time),
                        inner: diagnostics.into(),
                    })
                }))
                .wrap_err("failed to publish solve diagnostics")?;
        }
    }));
    let mut live_localization = LiveVisualOdometryLocalization::default();
    let mut global_visual_lock = GlobalVisualLock::Unlocked;

    loop {
        select! {
            field_mark_associations = field_mark_associations_subscriber.recv() => {
                let field_mark_associations = field_mark_associations?;
                let has_global_associations = field_mark_associations
                    .inner
                    .associations
                    .iter()
                    .any(|association| {
                        matches!(association.kind, FieldMarkAssociationKind::GlobalUnique)
                    });
                if has_global_associations && !global_visual_lock.has_backend_result() {
                    continue;
                }

                ingest_field_mark_associations(&mut frontend, field_mark_associations)
                    .wrap_err("failed to ingest globally associated field marks")?;
            }
            global_pose = global_pose_subscriber.recv() => {
                let global_pose = global_pose?;
                if global_pose.inner.is_none() {
                    continue;
                }
                if matches!(global_visual_lock, GlobalVisualLock::WaitingForBackend) {
                    continue;
                }

                ingest_global_pose(&mut frontend, global_pose)
                    .wrap_err("failed to ingest global pose reset into frontend")?;
                global_visual_lock = GlobalVisualLock::WaitingForBackend;
                live_localization.clear();
            }
            // IMU payloads have no sensor timestamp; ros-z source time is the aligned clock.
            imu = imu_subscriber.recv_with_metadata() => {
                let imu = imu?;
                frontend.ingest_imu(imu.source_time.to_wallclock(), imu.message)
                    .wrap_err("failed to ingest imu measurement into frontend")?;
            }
            visual_odometry = visual_odometry_subscriber.recv() => {
                let visual_odometry = visual_odometry?;
                let Some(previous_camera_matrix) = fresh_camera_matrix(&camera_matrix_cache, visual_odometry.previous_time) else {
                    continue;
                };
                let Some(current_camera_matrix) = fresh_camera_matrix(&camera_matrix_cache, visual_odometry.current_time) else {
                    continue;
                };

                ingest_visual_odometry(&mut frontend, visual_odometry, &previous_camera_matrix.inner, &current_camera_matrix.inner)
                    .wrap_err("failed to ingest visual odometry measurement into frontend")?;
            }
            visual_odometer = visual_odometer_subscriber.recv() => {
                let visual_odometer = visual_odometer?;
                if !global_visual_lock.has_backend_result() {
                    continue;
                }

                live_localization.try_reset_pending(
                    &visual_odometer_cache,
                    &camera_matrix_cache,
                );

                if let Some(transform) = live_localization.field_to_robot_from_odometer(
                    &visual_odometer,
                    &camera_matrix_cache,
                ) {
                    if let Some(camera_matrix) = fresh_camera_matrix(&camera_matrix_cache, visual_odometer.time) {
                        ground_to_field_publisher
                            .publish(&ground_to_field_from_field_to_robot(
                                transform,
                                &camera_matrix.inner.ground_to_robot,
                            ))
                            .await?;
                    }
                    let localization = Some(transform);
                    localization_publisher.publish(&localization).await?;
                    timestamped_localization_publisher
                        .publish(&TimeWrapper {
                            time: visual_odometer.time,
                            inner: localization,
                        })
                        .await?;
                }
            }
            robot_kinematics = robot_kinematics_subscriber.recv() => {
                let robot_kinematics = robot_kinematics?;
                ingest_foot_heights(&mut frontend, robot_kinematics)
                    .wrap_err("failed to ingest foot height measurement into frontend")?;
            }
            result = &mut backend_handle => {
                result.wrap_err("failed to join")?.wrap_err("solver failed")?;
                bail!("solver stopped unexpectedly")
            }
            result = frontend.wait_for_optimization_result() => {
                result?;
                let Some(result) = frontend.last_optimization_result() else { continue };
                let result_time = Time::from_wallclock(result.time);
                let camera_matrix = fresh_camera_matrix(&camera_matrix_cache, result_time);
                let backend_transform = backend_localization_for_result(&result, camera_matrix.as_deref());
                let transform = if global_visual_lock.is_active() {
                    global_visual_lock = GlobalVisualLock::Locked;
                    live_localization.reset(&result, &visual_odometer_cache, &camera_matrix_cache);
                    Some(
                        live_localization
                            .field_to_robot_latest(&visual_odometer_cache, &camera_matrix_cache)
                            .unwrap_or(backend_transform),
                    )
                } else {
                    None
                };

                if let (Some(transform), Some(camera_matrix)) = (transform, camera_matrix.as_deref()) {
                    ground_to_field_publisher
                        .publish(&ground_to_field_from_field_to_robot(
                            transform,
                            &camera_matrix.inner.ground_to_robot,
                        ))
                        .await?;
                }
                localization_publisher.publish(&transform).await?;
                timestamped_localization_publisher
                    .publish(&TimeWrapper {
                        time: result_time,
                        inner: global_visual_lock.is_active().then_some(backend_transform),
                    })
                    .await?;

                calibrated_intrinsics_publisher
                    .publish_if_subscribed(|| {
                        ready(intrinsic_from_camera_intrinsics(&result.camera_intrinsics))
                    })
                    .await?;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GlobalVisualLock {
    Unlocked,
    WaitingForBackend,
    Locked,
}

impl GlobalVisualLock {
    fn is_active(self) -> bool {
        !matches!(self, Self::Unlocked)
    }

    fn has_backend_result(self) -> bool {
        matches!(self, Self::Locked)
    }
}

#[derive(Default)]
struct LiveVisualOdometryLocalization {
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
    fn clear(&mut self) {
        self.anchor = None;
        self.pending_result = None;
    }

    fn reset(
        &mut self,
        result: &OptimizationResult,
        visual_odometer_cache: &VisualOdometerCache,
        camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    ) {
        self.pending_result = Some(result.clone());
        self.anchor = None;
        self.try_reset_pending(visual_odometer_cache, camera_matrix_cache);
    }

    fn try_reset_pending(
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

    fn field_to_robot_latest(
        &mut self,
        visual_odometer_cache: &VisualOdometerCache,
        camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    ) -> Option<Isometry3<Field, Robot>> {
        let latest = visual_odometer_cache.get_latest()?;
        self.field_to_robot_from_odometer(&latest, camera_matrix_cache)
    }

    fn field_to_robot_from_odometer(
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

fn localization_transform_from_backend_pose(
    robot_to_field: &nalgebra::Isometry3<f64>,
) -> Isometry3<Field, Robot> {
    robot_to_field.inverse().cast::<f32>().framed_transform()
}

fn backend_localization_for_result(
    result: &OptimizationResult,
    camera_matrix: Option<&TimeWrapper<CameraMatrix>>,
) -> Isometry3<Field, Robot> {
    let backend_localization = localization_transform_from_backend_pose(&result.transform);
    camera_matrix
        .map(|camera_matrix| {
            constrain_localization_to_ground(
                backend_localization,
                &camera_matrix.inner.ground_to_robot,
            )
        })
        .unwrap_or(backend_localization)
}

fn localization_transform_constrained_to_ground(
    robot_to_field: &nalgebra::Isometry3<f64>,
    ground_to_robot: &Isometry3<Ground, Robot>,
) -> Isometry3<Field, Robot> {
    let localization = localization_transform_from_backend_pose(robot_to_field);

    constrain_localization_to_ground(localization, ground_to_robot)
}

fn ground_to_field_from_field_to_robot(
    field_to_robot: Isometry3<Field, Robot>,
    ground_to_robot: &Isometry3<Ground, Robot>,
) -> Isometry2<Ground, Field> {
    let robot_to_field = field_to_robot.inverse();
    let ground_to_field = robot_to_field * *ground_to_robot;
    let (_, _, field_to_robot_yaw) = field_to_robot.inner.rotation.euler_angles();
    let yaw = -field_to_robot_yaw;
    let translation = ground_to_field.inner.translation.vector;

    Isometry2::wrap(nalgebra::Isometry2::new(
        nalgebra::vector![translation.x, translation.y],
        yaw,
    ))
}

fn constrain_localization_to_ground(
    localization: Isometry3<Field, Robot>,
    ground_to_robot: &Isometry3<Ground, Robot>,
) -> Isometry3<Field, Robot> {
    let (_, _, yaw) = localization.inner.rotation.euler_angles();
    let (roll, pitch, _) = ground_to_robot.inner.rotation.euler_angles();

    let mut translation = localization.inner.translation;
    translation.vector.z = ground_to_robot.inner.translation.vector.z;

    nalgebra::Isometry3::from_parts(
        translation,
        nalgebra::UnitQuaternion::from_euler_angles(roll, pitch, yaw),
    )
    .framed_transform()
}

fn camera_matrix_is_fresh(camera_matrix: &TimeWrapper<CameraMatrix>, time: Time) -> bool {
    time_distance(camera_matrix.time, time) <= MAX_CAMERA_MATRIX_TIME_DISTANCE
}

fn fresh_camera_matrix(
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    time: Time,
) -> Option<Arc<TimeWrapper<CameraMatrix>>> {
    let camera_matrix = camera_matrix_cache.get_nearest(time)?;
    camera_matrix_is_fresh(&camera_matrix, time).then_some(camera_matrix)
}

fn time_distance(a: Time, b: Time) -> Duration {
    Duration::from_nanos(a.as_nanos().abs_diff(b.as_nanos()))
}

fn ingest_field_mark_associations(
    frontend: &mut VinsFrontend,
    field_mark_associations: TimeWrapper<FieldMarkAssociations>,
) -> Result<(), VinsFrontendError> {
    let associations = field_mark_associations
        .inner
        .associations
        .into_iter()
        .map(|association| VisualReprojectionAssociation {
            detection: association.detection,
            field_point: association.field_point,
            kind: match association.kind {
                FieldMarkAssociationKind::GlobalUnique => {
                    VisualReprojectionAssociationKind::GlobalUnique
                }
                FieldMarkAssociationKind::PoseHint => VisualReprojectionAssociationKind::PoseHint,
            },
        });
    frontend.ingest_visual_reprojection_associations(
        field_mark_associations.time.to_wallclock(),
        associations,
        field_mark_associations.inner.robot_to_camera.inner,
    )
}

fn ingest_global_pose(
    frontend: &mut VinsFrontend,
    global_pose: TimeWrapper<Option<Isometry3<Robot, Field>>>,
) -> Result<(), VinsFrontendError> {
    let Some(robot_to_field) = global_pose.inner else {
        return Ok(());
    };

    frontend.ingest_global_pose(
        global_pose.time.to_wallclock(),
        robot_to_field.inner.cast::<f64>(),
    )
}

async fn wait_for_initial_state(
    camera_matrix_cache: &Cache<TimeWrapper<CameraMatrix>>,
    field_dimensions: &FieldDimensions,
) -> InitialState {
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    loop {
        if let Some(camera_matrix) = camera_matrix_cache.get_latest() {
            return initial_state_from_camera_matrix(&camera_matrix.inner, field_dimensions);
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

/// Constructs the backend initial state from the first live camera matrix.
///
/// The initial pose uses a fixed own-half right-touchline prior and the camera intrinsics contained
/// in `camera_matrix`.
pub fn initial_state_from_camera_matrix(
    camera_matrix: &CameraMatrix,
    field_dimensions: &FieldDimensions,
) -> InitialState {
    let initial_pose = initial_robot_to_field_from_field_dimensions(field_dimensions).inner;

    InitialState::from_isometry_and_intrinsics(
        initial_pose,
        Vector3::zeros(),
        camera_intrinsics_from_matrix(camera_matrix),
    )
}

/// Estimates the initial robot pose in the field frame from known startup placement.
///
/// Robots start on the right touchline in their own half, looking into the field.
pub fn initial_robot_to_field_from_field_dimensions(
    field_dimensions: &FieldDimensions,
) -> Isometry3<Robot, Field, f64> {
    nalgebra::Isometry3::from_parts(
        nalgebra::Translation3::new(
            (field_dimensions.length as f64) / 2.0,
            -(field_dimensions.width as f64) / 2.0,
            DEFAULT_INITIAL_ROBOT_HEIGHT,
        ),
        nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, FRAC_PI_2),
    )
    .framed_transform()
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

/// Ingests one visual-odometry delta into the VINS frontend.
///
/// `previous_camera_matrix` and `current_camera_matrix` provide the robot-to-camera extrinsics for
/// the two endpoints of the visual-odometry measurement.
pub fn ingest_visual_odometry(
    frontend: &mut VinsFrontend,
    delta: VisualOdometryDeltaMessage,
    previous_camera_matrix: &CameraMatrix,
    current_camera_matrix: &CameraMatrix,
) -> Result<(), VinsFrontendError> {
    frontend.ingest_visual_odometry_delta(
        delta.previous_time.to_wallclock(),
        delta.current_time.to_wallclock(),
        robot_to_camera(previous_camera_matrix).inner,
        robot_to_camera(current_camera_matrix).inner,
        delta.current_left_camera_to_previous_left_camera,
    )
}

/// Ingests left and right foot-height observations into the VINS frontend.
///
/// The sole positions are read from `robot_kinematics` in the robot frame and timestamped with the
/// kinematics message time.
pub fn ingest_foot_heights(
    frontend: &mut VinsFrontend,
    robot_kinematics: TimeWrapper<RobotKinematics>,
) -> Result<(), VinsFrontendError> {
    let (left_sole_in_robot, right_sole_in_robot) = foot_height_points(&robot_kinematics.inner);

    frontend.ingest_foot_heights(
        robot_kinematics.time.to_wallclock(),
        left_sole_in_robot,
        right_sole_in_robot,
    )
}

fn foot_height_points(robot_kinematics: &RobotKinematics) -> (Point3<f64>, Point3<f64>) {
    (
        robot_kinematics
            .left_leg
            .sole_to_robot
            .translation()
            .inner
            .cast(),
        robot_kinematics
            .right_leg
            .sole_to_robot
            .translation()
            .inner
            .cast(),
    )
}

fn robot_to_camera(camera_matrix: &CameraMatrix) -> Isometry3<Robot, Camera> {
    camera_matrix.head_to_camera * camera_matrix.robot_to_head
}

#[cfg(test)]
mod tests {
    use coordinate_systems::{Camera, Head, LeftSole, RightSole};

    use super::*;

    #[test]
    fn initial_state_from_camera_matrix_uses_startup_prior_and_live_intrinsics() {
        let robot_to_ground_rotation = nalgebra::UnitQuaternion::from_euler_angles(0.1, -0.2, 0.3);
        let robot_to_ground = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.0, 2.0, 0.42),
            robot_to_ground_rotation,
        );
        let camera_matrix = CameraMatrix {
            ground_to_robot: robot_to_ground.inverse().framed_transform(),
            intrinsics: Intrinsic::new(nalgebra::vector![216.0, 217.0], point![251.0, 235.0]),
            ..Default::default()
        };

        let initial_state =
            initial_state_from_camera_matrix(&camera_matrix, &FieldDimensions::SPL_2025);

        assert!(
            (initial_state.pose.xyz().x + FieldDimensions::SPL_2025.length as f64 / 2.0).abs()
                < 1.0e-9
        );
        assert!(
            (initial_state.pose.xyz().y + FieldDimensions::SPL_2025.width as f64 / 2.0).abs()
                < 1.0e-9
        );
        assert!((initial_state.pose.xyz().z - DEFAULT_INITIAL_ROBOT_HEIGHT).abs() < 1.0e-9);
        assert!(initial_state.pose.uvw().norm() < 1.0e-9);
        assert_eq!(
            initial_state.camera_intrinsics.focals(),
            nalgebra::vector![216.0, 217.0]
        );
        assert_eq!(
            initial_state.camera_intrinsics.optical_center(),
            nalgebra::vector![251.0, 235.0]
        );
        let rotation = initial_state.pose.rot();
        let yaw = (2.0 * (rotation.w() * rotation.z() + rotation.x() * rotation.y()))
            .atan2(1.0 - 2.0 * (rotation.y().powi(2) + rotation.z().powi(2)));
        assert!((yaw - FRAC_PI_2).abs() < 1.0e-9);
    }

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
    fn localization_publisher_outputs_field_to_robot() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(-3.0, 0.25, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.3),
        );

        let field_to_robot = localization_transform_from_backend_pose(&robot_to_field);
        let roundtrip_robot_to_field = field_to_robot.inverse().inner.cast::<f64>();

        assert!(
            (roundtrip_robot_to_field.translation.vector - robot_to_field.translation.vector)
                .norm()
                < 1.0e-6
        );
        assert!(
            roundtrip_robot_to_field
                .rotation
                .angle_to(&robot_to_field.rotation)
                < 1.0e-6
        );
    }

    #[test]
    fn constrained_localization_preserves_yaw_and_xy_but_uses_ground_tilt_and_height() {
        let localization: Isometry3<Field, Robot> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, -1.7),
            nalgebra::UnitQuaternion::from_euler_angles(3.0, 0.2, 0.7),
        )
        .framed_transform();
        let ground_to_robot: Isometry3<Ground, Robot> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.01, -0.02, -0.523),
            nalgebra::UnitQuaternion::from_euler_angles(-0.045, -0.047, 0.1),
        )
        .framed_transform();

        let constrained = constrain_localization_to_ground(localization, &ground_to_robot);
        let (roll, pitch, yaw) = constrained.inner.rotation.euler_angles();

        assert!((constrained.inner.translation.vector.x - 1.5).abs() < 1.0e-6);
        assert!((constrained.inner.translation.vector.y + 2.0).abs() < 1.0e-6);
        assert!((constrained.inner.translation.vector.z + 0.523).abs() < 1.0e-6);
        assert!((roll + 0.045).abs() < 1.0e-6);
        assert!((pitch + 0.047).abs() < 1.0e-6);
        assert!((yaw - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn ground_to_field_from_field_to_robot_flattens_robot_pose() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.7),
        );
        let field_to_robot: Isometry3<Field, Robot> = robot_to_field.inverse().framed_transform();
        let ground_to_robot = Isometry3::identity();

        let ground_to_field = ground_to_field_from_field_to_robot(field_to_robot, &ground_to_robot);

        assert!((ground_to_field.translation().x() - 1.5).abs() < 1.0e-6);
        assert!((ground_to_field.translation().y() + 2.0).abs() < 1.0e-6);
        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn ground_to_field_from_field_to_robot_ignores_ground_roll_pitch() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.7),
        );
        let unconstrained_localization: Isometry3<Field, Robot> =
            robot_to_field.inverse().framed_transform();
        let ground_to_robot: Isometry3<Ground, Robot> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.0, 0.0, -0.523),
            nalgebra::UnitQuaternion::from_euler_angles(-0.045, -0.047, 0.0),
        )
        .framed_transform();
        let constrained_localization =
            constrain_localization_to_ground(unconstrained_localization, &ground_to_robot);

        let ground_to_field =
            ground_to_field_from_field_to_robot(constrained_localization, &ground_to_robot);

        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
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

    #[test]
    fn foot_height_points_use_sole_positions_in_robot_frame() {
        let left_sole_to_robot: Isometry3<LeftSole, Robot> =
            nalgebra::Isometry3::translation(0.1, 0.2, -0.3).framed_transform();
        let right_sole_to_robot: Isometry3<RightSole, Robot> =
            nalgebra::Isometry3::translation(0.4, -0.5, -0.6).framed_transform();
        let robot_kinematics = RobotKinematics {
            left_leg: kinematics::robot_kinematics::RobotLeftLegKinematics {
                sole_to_robot: left_sole_to_robot,
                ..Default::default()
            },
            right_leg: kinematics::robot_kinematics::RobotRightLegKinematics {
                sole_to_robot: right_sole_to_robot,
                ..Default::default()
            },
            ..Default::default()
        };

        let (left, right) = foot_height_points(&robot_kinematics);

        assert!((left - nalgebra::Point3::new(0.1, 0.2, -0.3)).norm() < 1.0e-6);
        assert!((right - nalgebra::Point3::new(0.4, -0.5, -0.6)).norm() < 1.0e-6);
    }

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

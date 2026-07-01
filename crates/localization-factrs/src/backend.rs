use std::{
    cell::OnceCell,
    time::{Duration, SystemTime},
};

use factrs::{
    containers::FactorBuilder,
    core::{GaussNewton, Graph, Huber, PriorResidual, SE3, SO3, Values, Vector3},
    linalg::{Matrix3, VectorX},
    noise::GaussianNoise,
    optimizers::{BaseOptParams, OptError, OptStatus},
    residuals::ErasedResidual,
    traits::{Optimizer, Variable},
    variables::SE23,
};
use itertools::Itertools;
use nalgebra::{Matrix2, SMatrix};
use thiserror::Error;

use crate::{
    factors::{
        field_containment::FieldContainmentFactor,
        foot_above_ground::{FootHeightMeasurement, IntervalFootAboveGroundFactor},
        gaussian_process_prior::GaussianProcessPriorFactor,
        imu::{
            CurrentSplineOrientationFactor, IntervalGaussianProcessImuFactor, RelativeYawFactor,
            RollPitchPriorFactor, interpolate_measurement_orientation,
        },
        visual_odometry::{
            AdjacentVisualOdometryFactor, VisualOdometryDelta, VisualOdometryFactor,
            VisualOdometryMeasurement,
        },
        visual_reprojection::VisualReprojectionFactor,
    },
    initial_state::InitialState,
    interval_measurement::IntervalMeasurements,
    measurements::{
        GlobalPoseMeasurement, ImuMeasurement, SensorMeasurement, VisualReprojectionMeasurement,
    },
    schur_marginalization::marginalize,
    splines::SE23Spline,
    symbols::{CameraIntrinsics, State},
    tau,
};

use types::field_dimensions::FieldDimensions;

use tokio::sync::{
    mpsc::{UnboundedReceiver, error::TryRecvError},
    watch,
};

const INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA: f64 = 1e-3;
const INITIAL_POSE_PRIOR_SIGMA: f64 = 10.0;
const GLOBAL_VISUAL_HUBER_THRESHOLD: f64 = 2.0;
const VISUAL_ODOMETRY_HUBER_THRESHOLD: f64 = 2.0;
// Empty intervals are inserted only to keep graph components connected across
// dropped recording data. They use zero-start-velocity GP priors so stale
// pre-gap velocity is not treated as measured ballistic motion.
const EMPTY_INTERVAL_PROCESS_COVARIANCE_SCALE: f64 = 10.0;
const LONG_GAP_MIN_EMPTY_INTERVALS: u32 = 5;
const IMU_KINEMATICS_MIN_SPACING: Duration = Duration::from_millis(20); // 50Hz

pub struct BackendConfiguration {
    /// The spacing between control knots on the Gaussian Process
    /// Each control knot represents 9 DoFs for the optimizer.
    pub knot_spacing: Duration,
    /// The maximum optimization window size.
    /// Factors before the optimization window are marginalized.
    pub max_optimization_window: Duration,
    /// Maximum optimizer iterations per solve call.
    /// Slow solve cadences can spend more iterations on each larger batch.
    pub optimizer_max_iterations: usize,

    pub gyroscope_noise: Matrix3<f64>,
    pub accelerometer_noise: Matrix3<f64>,
    pub use_accelerometer_measurements: bool,
    pub gyroscope_process_noise: Matrix3<f64>,
    pub accelerometer_process_noise: Matrix3<f64>,
    pub gravity: Vector3<f64>,

    pub roll_pitch_yaw_noise: Matrix3<f64>,
    pub visual_feature_noise: Matrix2<f64>,
    pub pose_hint_visual_feature_noise: Matrix2<f64>,
    pub pose_hint_visual_huber_threshold: f64,
    pub visual_odometry_noise: SMatrix<f64, 6, 6>,
    pub foot_ground_sigma: f64,
    pub field_containment: FieldContainmentConfiguration,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldContainmentConfiguration {
    pub x_limit: f64,
    pub y_limit: f64,
    pub sigma: f64,
}

impl FieldContainmentConfiguration {
    pub fn from_field_dimensions(field_dimensions: &FieldDimensions, sigma: f64) -> Self {
        Self {
            x_limit: field_dimensions.length as f64 * 0.5
                + field_dimensions.border_strip_width as f64,
            y_limit: field_dimensions.width as f64 * 0.5
                + field_dimensions.border_strip_width as f64,
            sigma,
        }
    }
}

impl Default for FieldContainmentConfiguration {
    fn default() -> Self {
        Self::from_field_dimensions(&FieldDimensions::SPL_2025, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendOptimizerStatus {
    Converged,
    MaxIterations,
    FailedToStep,
    InvalidSystem,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResidualDiagnostics {
    pub factor_count: usize,
    pub residual_dim: usize,
    pub mean_rms: f64,
    pub max_rms: f64,
}

#[derive(Debug, Clone)]
pub struct BackendSolveDiagnostics {
    pub optimizer_status: BackendOptimizerStatus,
    pub value_count: usize,
    pub factor_count: usize,
    pub total_error: f64,
    pub visual_odometry: ResidualDiagnostics,
    pub visual_reprojection: ResidualDiagnostics,
    pub gaussian_process_prior: ResidualDiagnostics,
}

#[derive(Debug, Error)]
pub enum VinsBackendError {
    #[error("frontend disconnected")]
    FrontendDisconnected,
    #[error("failed to ingest IMU measurements")]
    FailedToIngestImu,
    #[error("failed to ingest visual measurements")]
    FailedToIngestVisual,
    #[error("failed to ingest visual odometry measurements")]
    FailedToIngestVisualOdometry,
    #[error("failed to ingest foot height measurements")]
    FailedToIngestFootHeights,
}

#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// The timestamp of the most recent variable in the optimized graph.
    pub time: SystemTime,
    /// The latest pose estimate from the optimized graph.
    pub latest_pose: SE23<f64>,
    /// The current optimized camera intrinsics estimate.
    pub camera_intrinsics: crate::camera_intrinsics::CameraIntrinsics<f64>,
}

impl OptimizationResult {
    pub fn position(&self) -> Vector3<f64> {
        self.latest_pose.xyz().into_owned()
    }
}

pub struct VinsBackend {
    /// Channel to retrieve new measurements from the frontend.
    measurement_receiver: UnboundedReceiver<SensorMeasurement>,
    /// Channel to send solver results to the frontend,
    result_sender: watch::Sender<Option<OptimizationResult>>,
    /// Configuration parameters for the solver backend
    config: BackendConfiguration,
    /// Initial state of the graph
    initial_state: InitialState,
    /// Stores the optimizer and the optimization graph
    optimizer: GaussNewton,
    /// Stores the optimized graph values.
    values: Values,
    /// Stores the timestamp of the last knot added to the graph.
    last_knot_time: Option<SystemTime>,
    /// Highest interval whose start/end states and GP prior have been initialized.
    highest_initialized_interval: Option<u32>,
    /// Stores the timestamp of the first measurement received by the backend.
    interval_assigner: IntervalAssigner,
    /// Last IMU attitude sample seen by the sorted stream.
    last_imu_attitude_measurement: Option<ImuMeasurement>,
    /// Latest IMU attitude sample used by the live in-interval orientation factor.
    latest_imu_attitude_measurement: Option<ImuMeasurement>,
    /// Next knot whose interpolated IMU attitude measurement has not been finalized yet.
    next_imu_attitude_knot_index: u32,
    /// Latest finalized knot orientation, used to form relative yaw and live yaw residuals.
    last_imu_knot_orientation: Option<ImuKnotOrientation>,
    /// Optimizer status from the most recent solve.
    last_optimizer_status: Option<BackendOptimizerStatus>,
    last_imu_kinematics_measurement_time: Option<SystemTime>,
}

pub fn initialize_graph(
    initial_state: &InitialState,
    config: &BackendConfiguration,
) -> (Graph, Values) {
    let mut graph = Graph::default();
    let mut values = Values::default();

    // Initialize camera intrinsics
    let factor = FactorBuilder::new(
        PriorResidual::new(initial_state.camera_intrinsics.clone()),
        CameraIntrinsics(0),
    )
    .noise(GaussianNoise::<4>::from_diag_sigmas(
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
        INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA,
    ))
    .build();

    graph.add_factor(factor);
    values.insert(CameraIntrinsics(0), initial_state.camera_intrinsics.clone());

    // Initial orientation
    let initial_pose = initial_state.pose.clone();
    let factor = FactorBuilder::new(PriorResidual::new(initial_pose.clone()), State(0))
        .noise(GaussianNoise::<9>::from_scalar_sigma(
            INITIAL_POSE_PRIOR_SIGMA,
        ))
        .build();
    graph.add_factor(factor);
    values.insert(State(0), initial_pose);
    add_field_containment_factor(&mut graph, State(0), config);

    (graph, values)
}

fn optimizer_from_graph(config: &BackendConfiguration, graph: Graph) -> GaussNewton {
    let mut optimizer = GaussNewton::new(
        BaseOptParams {
            max_iterations: config.optimizer_max_iterations,
            ..Default::default()
        },
        graph,
    );
    optimizer.set_dense_normal_equations(true);
    optimizer
}

impl VinsBackend {
    pub(crate) fn new(
        config: BackendConfiguration,
        initial_state: InitialState,
        measurement_receiver: UnboundedReceiver<SensorMeasurement>,
        result_sender: watch::Sender<Option<OptimizationResult>>,
    ) -> Self {
        assert!(
            config.optimizer_max_iterations > 0,
            "optimizer_max_iterations must be positive"
        );

        let (graph, values) = initialize_graph(&initial_state, &config);

        let optimizer = optimizer_from_graph(&config, graph);

        Self {
            interval_assigner: IntervalAssigner::new(config.knot_spacing),
            measurement_receiver,
            result_sender,
            config,
            initial_state,
            optimizer,
            values,
            last_knot_time: None,
            highest_initialized_interval: None,
            last_imu_attitude_measurement: None,
            latest_imu_attitude_measurement: None,
            next_imu_attitude_knot_index: 0,
            last_imu_knot_orientation: None,
            last_optimizer_status: None,
            last_imu_kinematics_measurement_time: None,
        }
    }

    pub fn values(&self) -> &Values {
        &self.values
    }

    pub fn compute_last_solve_diagnostics(&self) -> Option<BackendSolveDiagnostics> {
        self.last_optimizer_status
            .map(|optimizer_status| self.solve_diagnostics(optimizer_status))
    }

    /// Blocks until new measurements are available, ingests them, and optimizes the graph once.
    pub fn solve_next_blocking(&mut self) -> Result<Option<OptimizationResult>, VinsBackendError> {
        let mut measurements = Vec::new();
        if self
            .measurement_receiver
            .blocking_recv_many(&mut measurements, usize::MAX)
            == 0
        {
            return Err(VinsBackendError::FrontendDisconnected);
        }

        self.ingest_sensor_measurements(measurements.drain(..))?;
        self.optimize_and_publish()
    }

    pub fn solve_once(&mut self) -> Result<Option<OptimizationResult>, VinsBackendError> {
        self.ingest_until_empty()?;
        self.optimize_and_publish()
    }

    fn optimize_and_publish(&mut self) -> Result<Option<OptimizationResult>, VinsBackendError> {
        let result = self.optimize();
        if self.result_sender.send(result.clone()).is_err() {
            return Err(VinsBackendError::FrontendDisconnected);
        }

        Ok(result)
    }

    /// Ingests a batch of IMU measurements into the graph.
    /// Assumes the measurements are already sorted by time.
    fn ingest_imu(&mut self, measurements: Vec<ImuMeasurement>) {
        let (Some(first), Some(last)) = (measurements.first(), measurements.last()) else {
            return;
        };
        let first_time = first.time;
        let last_time = last.time;
        let _ = self.interval_assigner.assign_interval(first_time);
        self.update_last_knot_time(last_time);

        let Some(last_interval_index) = self.interval_assigner.assign_interval(last_time) else {
            return;
        };
        self.init_intervals_through(last_interval_index);

        let kinematics_measurements =
            self.keep_imu_kinematics_measurements(measurements.iter().cloned());
        self.add_imu_kinematics_factors(kinematics_measurements);

        self.process_imu_attitude_measurements(measurements);
    }

    fn process_imu_attitude_measurements(&mut self, measurements: Vec<ImuMeasurement>) {
        let mut previous = self.last_imu_attitude_measurement.clone();

        for measurement in measurements {
            if let Some(previous) = previous.as_ref() {
                debug_assert!(
                    previous.time <= measurement.time,
                    "IMU measurements must be globally sorted by time"
                );
                self.add_imu_attitude_factors_between(previous, &measurement);
            } else {
                self.add_exact_imu_attitude_knot(&measurement);
            }

            previous = Some(measurement);
        }

        self.latest_imu_attitude_measurement = previous.clone();
        self.last_imu_attitude_measurement = previous;
    }

    fn add_exact_imu_attitude_knot(&mut self, measurement: &ImuMeasurement) {
        while let Some(knot_time) = self
            .interval_assigner
            .interval_start_time(self.next_imu_attitude_knot_index)
        {
            if knot_time > measurement.time {
                return;
            }

            if knot_time == measurement.time {
                self.add_imu_knot_orientation(
                    self.next_imu_attitude_knot_index,
                    interpolate_measurement_orientation(measurement, measurement, knot_time),
                );
            }
            self.next_imu_attitude_knot_index += 1;
        }
    }

    fn add_imu_attitude_factors_between(
        &mut self,
        previous: &ImuMeasurement,
        current: &ImuMeasurement,
    ) {
        while let Some(knot_time) = self
            .interval_assigner
            .interval_start_time(self.next_imu_attitude_knot_index)
        {
            if knot_time < previous.time {
                self.next_imu_attitude_knot_index += 1;
                continue;
            }
            if knot_time > current.time {
                return;
            }

            let measured_orientation =
                interpolate_measurement_orientation(previous, current, knot_time);
            self.add_imu_knot_orientation(self.next_imu_attitude_knot_index, measured_orientation);
            self.next_imu_attitude_knot_index += 1;
        }
    }

    fn add_imu_knot_orientation(&mut self, knot_index: u32, measured_orientation: SO3) {
        self.add_roll_pitch_prior_if_available(State(knot_index), measured_orientation.clone());

        if let Some(previous) = self.last_imu_knot_orientation.as_ref()
            && previous.index + 1 == knot_index
            && self.interval_states_available(previous.index)
        {
            self.add_relative_yaw_factor_if_available(
                previous.index,
                previous.orientation.clone(),
                measured_orientation.clone(),
            );
        }

        self.last_imu_knot_orientation = Some(ImuKnotOrientation {
            index: knot_index,
            orientation: measured_orientation,
        });
    }

    fn add_roll_pitch_prior_if_available(&mut self, state: State, measured_orientation: SO3) {
        if self.values.get(state).is_none() {
            return;
        }
        let graph = self.optimizer.graph_mut();
        if graph
            .factors_for_residual_mut::<RollPitchPriorFactor, _>(state)
            .next()
            .is_some()
        {
            return;
        }

        let residual =
            RollPitchPriorFactor::new(measured_orientation, self.config.roll_pitch_yaw_noise);
        let factor = FactorBuilder::new(residual, state).build();
        graph.add_factor(factor);
    }

    fn add_relative_yaw_factor_if_available(
        &mut self,
        interval_index: u32,
        measured_start_orientation: SO3,
        measured_end_orientation: SO3,
    ) {
        let keys = (State(interval_index), State(interval_index + 1));
        let graph = self.optimizer.graph_mut();
        if graph
            .factors_for_residual_mut::<RelativeYawFactor, _>(keys)
            .next()
            .is_some()
        {
            return;
        }

        let residual = RelativeYawFactor::new(
            measured_start_orientation,
            measured_end_orientation,
            self.config.roll_pitch_yaw_noise,
        );
        let factor = FactorBuilder::new(residual, keys).build();
        graph.add_factor(factor);
    }

    fn add_current_spline_orientation_factor(&mut self) {
        let Some(current_measurement) = self.latest_imu_attitude_measurement.clone() else {
            return;
        };
        let Some(start_time) = self
            .interval_assigner
            .current_interval_start_time(current_measurement.time)
        else {
            return;
        };
        let Some(interval_index) = self.interval_assigner.assign_interval(start_time) else {
            return;
        };
        if !self.interval_states_available(interval_index) {
            return;
        }

        let Some(start_orientation) = self.last_imu_knot_orientation.as_ref() else {
            return;
        };
        if start_orientation.index != interval_index {
            return;
        }

        let end_time = start_time + self.config.knot_spacing;
        let residual = CurrentSplineOrientationFactor::new(
            start_orientation.orientation.clone(),
            &current_measurement,
            self.config.roll_pitch_yaw_noise,
            start_time,
            end_time,
        );
        let factor =
            FactorBuilder::new(residual, (State(interval_index), State(interval_index + 1)))
                .build();
        self.optimizer.graph_mut().add_factor(factor);
    }

    fn remove_current_spline_orientation_factor(&mut self) {
        self.optimizer.graph_mut().remove_factors(|factor| {
            factor
                .residual_as::<CurrentSplineOrientationFactor>()
                .is_some()
        });
    }

    /// filter out measurements in order to keep graph from exploding with too many IMU kinematics factors when the frontend provides high-frequency IMU data
    fn keep_imu_kinematics_measurements(
        &mut self,
        measurements: impl IntoIterator<Item = ImuMeasurement>,
    ) -> Vec<ImuMeasurement> {
        let mut kept = Vec::new();

        for measurement in measurements {
            let keep = match self.last_imu_kinematics_measurement_time {
                None => true,
                Some(last) => measurement
                    .time
                    .duration_since(last)
                    .is_ok_and(|dt| dt >= IMU_KINEMATICS_MIN_SPACING),
            };

            if keep {
                self.last_imu_kinematics_measurement_time = Some(measurement.time);
                kept.push(measurement);
            }
        }

        kept
    }

    fn add_imu_kinematics_factors(&mut self, measurements: Vec<ImuMeasurement>) {
        if measurements.is_empty() {
            return;
        }

        let interval_groups = self.interval_groups(measurements, |measurement| measurement.time);

        for group in interval_groups {
            if !self.prepare_interval_for_measurements(group.start_index, "imu kinematics") {
                continue;
            }

            let keys = (State(group.start_index), State(group.start_index + 1));
            let graph = self.optimizer.graph_mut();

            if let Some(factor) = graph
                .factors_for_residual_mut::<IntervalGaussianProcessImuFactor, _>(keys)
                .next()
            {
                factor
                    .residual_as_mut::<IntervalGaussianProcessImuFactor>()
                    .expect("factor query must return matching residual")
                    .extend_measurements(group.measurements);
            } else {
                let residual = IntervalGaussianProcessImuFactor::new(
                    group.measurements,
                    self.config.gyroscope_noise,
                    self.config.accelerometer_noise,
                    self.config.use_accelerometer_measurements,
                    self.config.gravity,
                    group.start_time,
                    group.end_time,
                );
                graph.add_factor(FactorBuilder::new(residual, keys).build());
            }
        }
    }

    fn ingest_visual(&mut self, visuals: Vec<Vec<VisualReprojectionMeasurement>>) {
        self.ingest_visual_frames(
            visuals,
            "visual",
            self.config.visual_feature_noise,
            GLOBAL_VISUAL_HUBER_THRESHOLD,
        );
    }

    fn ingest_pose_hint_visual(&mut self, visuals: Vec<Vec<VisualReprojectionMeasurement>>) {
        self.ingest_visual_frames(
            visuals,
            "pose-hint visual",
            self.config.pose_hint_visual_feature_noise,
            self.config.pose_hint_visual_huber_threshold,
        );
    }

    fn ingest_visual_frames(
        &mut self,
        mut visuals: Vec<Vec<VisualReprojectionMeasurement>>,
        sensor_name: &str,
        noise: Matrix2<f64>,
        huber_threshold: f64,
    ) {
        visuals.retain(|visual| !visual.is_empty());
        let Some(last) = visuals.last() else {
            return;
        };

        let last_time = visual_frame_time(last);
        self.update_last_knot_time(last_time);

        let interval_groups = self.interval_groups(visuals, |visual| visual_frame_time(visual));

        for group in interval_groups {
            if !self.prepare_interval_for_measurements(group.start_index, sensor_name) {
                continue;
            }

            let keys = (
                State(group.start_index),
                State(group.start_index + 1),
                CameraIntrinsics(0),
            );
            let graph = self.optimizer.graph_mut();
            for measurement in group.measurements.into_iter().flatten() {
                let residual = VisualReprojectionFactor::new(
                    group.start_time,
                    group.end_time,
                    [measurement],
                    noise,
                );
                let factor = FactorBuilder::new(residual, keys)
                    .robust(Huber::new(huber_threshold))
                    .build();

                graph.add_factor(factor);
            }
        }
    }

    fn ingest_visual_odometry(&mut self, visual_odometry: Vec<VisualOdometryMeasurement>) {
        let Some(last) = visual_odometry.last() else {
            return;
        };
        let last_timestamp = last.current_time;

        let mut same_interval_deltas = Vec::new();
        let mut adjacent_interval_deltas = Vec::new();
        for measurement in visual_odometry {
            for measurement in self.split_visual_odometry_measurement(measurement) {
                match self.visual_odometry_delta(measurement) {
                    Some(TimedVisualOdometryDelta::SameInterval(delta)) => {
                        same_interval_deltas.push(delta);
                    }
                    Some(TimedVisualOdometryDelta::AdjacentInterval(delta)) => {
                        adjacent_interval_deltas.push(delta);
                    }
                    None => {}
                }
            }
        }

        if same_interval_deltas.is_empty() && adjacent_interval_deltas.is_empty() {
            return;
        }
        self.update_last_knot_time(last_timestamp);
        self.add_visual_odometry_factors(same_interval_deltas);
        self.add_adjacent_visual_odometry_factors(adjacent_interval_deltas);
    }

    fn add_visual_odometry_factors(&mut self, deltas: Vec<IntervalVisualOdometryDelta>) {
        let interval_groups =
            self.interval_groups(deltas, |measurement| measurement.interval_start_time);

        for group in interval_groups {
            if !self.prepare_visual_odometry_states(group.start_index, group.start_index) {
                continue;
            }

            let keys = (State(group.start_index), State(group.start_index + 1));
            let graph = self.optimizer.graph_mut();
            // factrs robust kernels are factor-wide, so keep each 6D delta in
            // its own factor to avoid downweighting unrelated residuals.
            for measurement in group.measurements {
                let residual = VisualOdometryFactor::new(
                    vec![measurement.delta],
                    self.config.visual_odometry_noise,
                    self.config.knot_spacing.as_secs_f64(),
                );
                let factor = FactorBuilder::new(residual, keys)
                    .robust(Huber::new(VISUAL_ODOMETRY_HUBER_THRESHOLD))
                    .build();

                graph.add_factor(factor);
            }
        }
    }

    fn add_adjacent_visual_odometry_factors(&mut self, deltas: Vec<IntervalVisualOdometryDelta>) {
        let interval_groups =
            self.interval_groups(deltas, |measurement| measurement.interval_start_time);

        for group in interval_groups {
            let end_index = group.start_index + 1;
            if !self.prepare_visual_odometry_states(group.start_index, end_index) {
                continue;
            }

            let keys = (
                State(group.start_index),
                State(group.start_index + 1),
                State(group.start_index + 2),
            );
            let graph = self.optimizer.graph_mut();
            // factrs robust kernels are factor-wide, so keep each 6D delta in
            // its own factor to avoid downweighting unrelated residuals.
            for measurement in group.measurements {
                let residual = AdjacentVisualOdometryFactor::new(
                    vec![measurement.delta],
                    self.config.visual_odometry_noise,
                    self.config.knot_spacing.as_secs_f64(),
                );
                let factor = FactorBuilder::new(residual, keys)
                    .robust(Huber::new(VISUAL_ODOMETRY_HUBER_THRESHOLD))
                    .build();

                graph.add_factor(factor);
            }
        }
    }

    fn prepare_visual_odometry_states(&mut self, start_index: u32, end_index: u32) -> bool {
        self.init_intervals_through(end_index);
        if (start_index..=(end_index + 1)).all(|index| self.values.get(State(index)).is_some()) {
            return true;
        }

        log::debug!(
            "skipping visual odometry measurements for marginalized states {start_index}..{}",
            end_index + 1
        );
        false
    }

    fn visual_odometry_delta(
        &self,
        measurement: VisualOdometryMeasurement,
    ) -> Option<TimedVisualOdometryDelta> {
        if measurement.current_time <= measurement.previous_time {
            log::debug!("dropping non-forward visual odometry measurement");
            return None;
        }

        let previous_interval_start_time = self
            .interval_assigner
            .current_interval_start_time(measurement.previous_time)?;
        let previous_interval_index = self
            .interval_assigner
            .assign_interval(previous_interval_start_time)?;
        let current_interval_start_time = self
            .interval_assigner
            .current_interval_start_time(measurement.current_time)?;
        let current_interval_index = self
            .interval_assigner
            .assign_interval(current_interval_start_time)?;

        match current_interval_index.checked_sub(previous_interval_index)? {
            0 => Some(TimedVisualOdometryDelta::SameInterval(
                IntervalVisualOdometryDelta {
                    interval_start_time: previous_interval_start_time,
                    delta: VisualOdometryDelta::from_measurement(
                        measurement,
                        previous_interval_start_time,
                        previous_interval_start_time + self.config.knot_spacing,
                    ),
                },
            )),
            1 => Some(TimedVisualOdometryDelta::AdjacentInterval(
                IntervalVisualOdometryDelta {
                    interval_start_time: previous_interval_start_time,
                    delta: VisualOdometryDelta::new(
                        tau(
                            previous_interval_start_time,
                            previous_interval_start_time + self.config.knot_spacing,
                            measurement.previous_time,
                        ),
                        tau(
                            current_interval_start_time,
                            current_interval_start_time + self.config.knot_spacing,
                            measurement.current_time,
                        ),
                        measurement.robot_delta,
                    ),
                },
            )),
            skipped_intervals => {
                log::debug!(
                    "dropping visual odometry measurement spanning {skipped_intervals} intervals"
                );
                None
            }
        }
    }

    fn split_visual_odometry_measurement(
        &self,
        measurement: VisualOdometryMeasurement,
    ) -> Vec<VisualOdometryMeasurement> {
        if measurement.current_time <= measurement.previous_time {
            return vec![measurement];
        }

        let Some(previous_interval_start_time) = self
            .interval_assigner
            .current_interval_start_time(measurement.previous_time)
        else {
            return vec![measurement];
        };
        let Some(previous_interval_index) = self
            .interval_assigner
            .assign_interval(previous_interval_start_time)
        else {
            return vec![measurement];
        };
        let Some(current_interval_start_time) = self
            .interval_assigner
            .current_interval_start_time(measurement.current_time)
        else {
            return vec![measurement];
        };
        let Some(current_interval_index) = self
            .interval_assigner
            .assign_interval(current_interval_start_time)
        else {
            return vec![measurement];
        };
        let Some(spanned_intervals) = current_interval_index.checked_sub(previous_interval_index)
        else {
            return vec![measurement];
        };
        if spanned_intervals == 0 {
            return vec![measurement];
        }

        let Ok(total_duration) = measurement
            .current_time
            .duration_since(measurement.previous_time)
        else {
            return vec![measurement];
        };
        if total_duration.is_zero() {
            return vec![measurement];
        }

        let tangent = measurement.robot_delta.log();
        let mut segments = Vec::new();
        let mut segment_start_time = measurement.previous_time;
        for boundary_index in (previous_interval_index + 1)..=current_interval_index {
            let Some(boundary_time) = self.interval_assigner.interval_start_time(boundary_index)
            else {
                continue;
            };
            if boundary_time <= segment_start_time || boundary_time >= measurement.current_time {
                continue;
            }
            let guarded_boundary_time = boundary_time
                .checked_sub(Duration::from_nanos(1))
                .unwrap_or(boundary_time);
            if guarded_boundary_time > segment_start_time {
                segments.push(split_visual_odometry_segment(
                    segment_start_time,
                    guarded_boundary_time,
                    total_duration,
                    &tangent,
                ));
            }
            segment_start_time = boundary_time;
        }
        if measurement.current_time > segment_start_time {
            segments.push(split_visual_odometry_segment(
                segment_start_time,
                measurement.current_time,
                total_duration,
                &tangent,
            ));
        }

        segments
    }

    fn ingest_foot_heights(&mut self, foot_heights: Vec<FootHeightMeasurement>) {
        let Some(last) = foot_heights.last() else {
            return;
        };
        self.update_last_knot_time(last.time);

        let interval_groups = self.interval_groups(foot_heights, |measurement| measurement.time);

        for group in interval_groups {
            if !self.prepare_interval_for_measurements(group.start_index, "foot height") {
                continue;
            }

            let keys = (State(group.start_index), State(group.start_index + 1));
            let graph = self.optimizer.graph_mut();
            if let Some(factor) = graph
                .factors_for_residual_mut::<IntervalFootAboveGroundFactor, _>(keys)
                .next()
            {
                factor
                    .residual_as_mut::<IntervalFootAboveGroundFactor>()
                    .expect("factor query must return matching residual")
                    .extend_measurements(group.measurements);
            } else {
                let residual = IntervalFootAboveGroundFactor::new(
                    group.measurements,
                    group.start_time,
                    group.end_time,
                    self.config.foot_ground_sigma,
                );
                let factor = FactorBuilder::new(residual, keys).build();

                graph.add_factor(factor);
            }
        }
    }

    fn init_intervals_through(&mut self, interval_start_index: u32) {
        let first_missing_interval = self
            .highest_initialized_interval
            .map_or(0, |index| index + 1);
        if first_missing_interval > interval_start_index {
            return;
        }

        let is_long_gap = interval_start_index.saturating_sub(first_missing_interval)
            >= LONG_GAP_MIN_EMPTY_INTERVALS;
        if is_long_gap {
            reset_state_velocity(&mut self.values, State(first_missing_interval));
        }
        for index in first_missing_interval..=interval_start_index {
            init_interval_states(
                &mut self.values,
                self.optimizer.graph_mut(),
                &self.config,
                &self.initial_state,
                index,
                is_long_gap && index < interval_start_index,
            );
        }
        self.highest_initialized_interval = Some(interval_start_index);
    }

    fn interval_states_available(&self, interval_start_index: u32) -> bool {
        self.values.get(State(interval_start_index)).is_some()
            && self.values.get(State(interval_start_index + 1)).is_some()
    }

    fn update_last_knot_time(&mut self, time: SystemTime) {
        self.last_knot_time = Some(self.last_knot_time.map_or(time, |t| t.max(time)));
    }

    fn interval_groups<T>(
        &self,
        measurements: Vec<T>,
        time_of: impl Fn(&T) -> SystemTime,
    ) -> Vec<IntervalGroup<T>> {
        let mut interval_groups = Vec::new();
        for (key, chunk) in measurements
            .into_iter()
            .chunk_by(|measurement| {
                self.interval_assigner
                    .current_interval_start_time(time_of(measurement))
            })
            .into_iter()
        {
            let Some(start_time) = key else {
                log::debug!("dropping measurements before earliest solver time");
                continue;
            };
            let Some(start_index) = self.interval_assigner.assign_interval(start_time) else {
                log::debug!("dropping measurements before earliest solver time");
                continue;
            };

            interval_groups.push(IntervalGroup {
                start_index,
                start_time,
                end_time: start_time + self.config.knot_spacing,
                measurements: chunk.collect(),
            });
        }

        interval_groups
    }

    fn prepare_interval_for_measurements(
        &mut self,
        interval_start_index: u32,
        sensor_name: &str,
    ) -> bool {
        self.init_intervals_through(interval_start_index);
        if self.interval_states_available(interval_start_index) {
            return true;
        }

        log::debug!(
            "skipping {sensor_name} measurements for marginalized interval {interval_start_index}"
        );
        false
    }

    fn ingest_until_empty(&mut self) -> Result<(), VinsBackendError> {
        let mut measurements = IntervalMeasurements::new();
        loop {
            match self.measurement_receiver.try_recv() {
                Ok(measurement) => measurements.push(measurement),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    return Err(VinsBackendError::FrontendDisconnected);
                }
            }
        }

        self.ingest_measurements(measurements)
    }

    fn ingest_sensor_measurements(
        &mut self,
        measurements: impl IntoIterator<Item = SensorMeasurement>,
    ) -> Result<(), VinsBackendError> {
        let mut interval_measurements = IntervalMeasurements::new();
        for measurement in measurements {
            interval_measurements.push(measurement);
        }

        self.ingest_measurements(interval_measurements)
    }

    fn ingest_measurements(
        &mut self,
        mut new_measurements: IntervalMeasurements,
    ) -> Result<(), VinsBackendError> {
        if let Some(global_pose) = new_measurements.latest_global_pose().cloned() {
            self.reset_to_global_pose(global_pose.clone());
            new_measurements.retain_at_or_after(global_pose.time);
        }

        self.ingest_imu(new_measurements.imu);
        self.ingest_visual(new_measurements.visual);
        self.ingest_pose_hint_visual(new_measurements.pose_hint_visual);
        self.ingest_visual_odometry(new_measurements.visual_odometry);
        self.ingest_foot_heights(new_measurements.foot_heights);

        Ok(())
    }

    fn reset_to_global_pose(&mut self, global_pose: GlobalPoseMeasurement) {
        let camera_intrinsics = self
            .values
            .get(CameraIntrinsics(0))
            .cloned()
            .unwrap_or_else(|| self.initial_state.camera_intrinsics.clone());
        self.initial_state = InitialState::new(
            SE23::from_rot_vel_trans(
                global_pose.robot_to_field.rot().clone(),
                Vector3::zeros(),
                global_pose.robot_to_field.xyz().into_owned(),
            ),
            camera_intrinsics,
        );

        let (graph, values) = initialize_graph(&self.initial_state, &self.config);
        self.optimizer = optimizer_from_graph(&self.config, graph);
        self.values = values;
        self.interval_assigner = IntervalAssigner::new(self.config.knot_spacing);
        let _ = self.interval_assigner.assign_interval(global_pose.time);
        self.last_knot_time = Some(global_pose.time);
        self.highest_initialized_interval = None;
        self.last_imu_attitude_measurement = None;
        self.latest_imu_attitude_measurement = None;
        self.next_imu_attitude_knot_index = 0;
        self.last_imu_knot_orientation = None;
        self.last_optimizer_status = None;
        self.last_imu_kinematics_measurement_time = None;
        self.init_intervals_through(0);
    }

    fn optimize(&mut self) -> Option<OptimizationResult> {
        self.last_optimizer_status = None;
        let time = self.last_knot_time?;
        log::info!("solving graph with {} values", self.values.len());

        self.remove_current_spline_orientation_factor();

        self.add_current_spline_orientation_factor();

        let optimizer_status = match self.optimizer.optimize(&mut self.values) {
            Ok(OptStatus::Converged) => BackendOptimizerStatus::Converged,
            Ok(OptStatus::MaxIterations) => {
                log::warn!("optimizer failed to converge: max iterations reached");
                BackendOptimizerStatus::MaxIterations
            }
            Err(OptError::FailedToStep) => {
                log::warn!("optimizer failed: failed to step");
                BackendOptimizerStatus::FailedToStep
            }
            Err(OptError::InvalidSystem) => {
                log::warn!("optimizer failed: invalid system");
                BackendOptimizerStatus::InvalidSystem
            }
        };
        self.remove_current_spline_orientation_factor();

        if matches!(
            optimizer_status,
            BackendOptimizerStatus::Converged | BackendOptimizerStatus::MaxIterations
        ) {
            // Marginalize only after the current batch has influenced the optimized state.
            let cutoff_time = time - self.config.max_optimization_window;
            if let Some(smallest_interval_index_in_window) =
                self.interval_assigner.assign_interval(cutoff_time)
            {
                marginalize(
                    &mut self.optimizer,
                    &mut self.values,
                    State(smallest_interval_index_in_window),
                );
            }
        }

        self.last_optimizer_status = Some(optimizer_status);

        let interval_start_time = self.interval_assigner.current_interval_start_time(time)?;
        let interval_start_index = self
            .interval_assigner
            .assign_interval(interval_start_time)?;
        let start = self.values.get(State(interval_start_index))?.clone();
        let end = self.values.get(State(interval_start_index + 1))?.clone();
        let interval_end_time = interval_start_time + self.config.knot_spacing;
        let latest_pose = SE23Spline::new(start, end, self.config.knot_spacing.as_secs_f64())
            .evaluate(tau(interval_start_time, interval_end_time, time));
        let camera_intrinsics = self.values.get(CameraIntrinsics(0))?.clone();

        Some(OptimizationResult {
            time,
            latest_pose,
            camera_intrinsics,
        })
    }

    fn solve_diagnostics(
        &self,
        optimizer_status: BackendOptimizerStatus,
    ) -> BackendSolveDiagnostics {
        let graph = self.optimizer.graph();
        let mut visual_odometry = self.residual_diagnostics::<VisualOdometryFactor>();
        visual_odometry.extend(self.residual_diagnostics::<AdjacentVisualOdometryFactor>());
        let visual_reprojection = self.residual_diagnostics::<VisualReprojectionFactor>();

        BackendSolveDiagnostics {
            optimizer_status,
            value_count: self.values.len(),
            factor_count: graph.len(),
            total_error: graph.error(&self.values),
            visual_odometry: visual_odometry.finish(),
            visual_reprojection: visual_reprojection.finish(),
            gaussian_process_prior: self
                .residual_diagnostics::<GaussianProcessPriorFactor>()
                .finish(),
        }
    }

    fn residual_diagnostics<R>(&self) -> ResidualDiagnosticsAccumulator
    where
        R: ErasedResidual + 'static,
    {
        let graph = self.optimizer.graph();
        let mut diagnostics = ResidualDiagnosticsAccumulator::default();
        for index in 0..graph.len() {
            let factor = graph.at(index);
            if !factor.is_residual::<R>() {
                continue;
            }
            let Ok(error) = factor.try_error(&self.values) else {
                continue;
            };
            let Ok(dim) = factor.try_dim_out(&self.values) else {
                continue;
            };
            diagnostics.add(error, dim);
        }
        diagnostics
    }
}

#[derive(Debug, Default)]
struct ResidualDiagnosticsAccumulator {
    factor_count: usize,
    residual_dim: usize,
    sum_squared_norm: f64,
    max_rms: f64,
}

impl ResidualDiagnosticsAccumulator {
    fn add(&mut self, factor_error: f64, residual_dim: usize) {
        if residual_dim == 0 {
            return;
        }
        let squared_norm = 2.0 * factor_error;
        let rms = (squared_norm / residual_dim as f64).sqrt();
        self.factor_count += 1;
        self.residual_dim += residual_dim;
        self.sum_squared_norm += squared_norm;
        self.max_rms = self.max_rms.max(rms);
    }

    fn extend(&mut self, other: Self) {
        self.factor_count += other.factor_count;
        self.residual_dim += other.residual_dim;
        self.sum_squared_norm += other.sum_squared_norm;
        self.max_rms = self.max_rms.max(other.max_rms);
    }

    fn finish(self) -> ResidualDiagnostics {
        ResidualDiagnostics {
            factor_count: self.factor_count,
            residual_dim: self.residual_dim,
            mean_rms: if self.residual_dim == 0 {
                0.0
            } else {
                (self.sum_squared_norm / self.residual_dim as f64).sqrt()
            },
            max_rms: self.max_rms,
        }
    }
}

fn visual_frame_time(visual: &[VisualReprojectionMeasurement]) -> SystemTime {
    visual
        .first()
        .expect("visual frames must contain at least one measurement")
        .time
}

fn split_visual_odometry_segment(
    previous_time: SystemTime,
    current_time: SystemTime,
    total_duration: Duration,
    tangent: &VectorX,
) -> VisualOdometryMeasurement {
    let segment_duration = current_time
        .duration_since(previous_time)
        .expect("split segment times must be ordered");
    let fraction = segment_duration.as_secs_f64() / total_duration.as_secs_f64();
    let scaled_tangent = tangent * fraction;
    VisualOdometryMeasurement {
        previous_time,
        current_time,
        robot_delta: SE3::exp(scaled_tangent.as_view()),
    }
}

struct IntervalGroup<T> {
    start_index: u32,
    start_time: SystemTime,
    end_time: SystemTime,
    measurements: Vec<T>,
}

#[derive(Debug, Clone)]
struct ImuKnotOrientation {
    index: u32,
    orientation: SO3,
}

enum TimedVisualOdometryDelta {
    SameInterval(IntervalVisualOdometryDelta),
    AdjacentInterval(IntervalVisualOdometryDelta),
}

struct IntervalVisualOdometryDelta {
    interval_start_time: SystemTime,
    delta: VisualOdometryDelta,
}

fn reset_state_velocity(values: &mut Values, state: State) {
    if let Some(pose) = values.get_mut(state) {
        *pose = zero_velocity_pose(pose);
    }
}

fn zero_velocity_pose(pose: &SE23) -> SE23 {
    SE23::from_rot_vel_trans(
        pose.rot().clone(),
        Vector3::zeros(),
        pose.xyz().into_owned(),
    )
}

fn init_interval_states(
    values: &mut Values,
    graph: &mut Graph,
    config: &BackendConfiguration,
    initial_state: &InitialState,
    interval_start_index: u32,
    is_empty_bridge_interval: bool,
) {
    let start = State(interval_start_index);
    let end = State(interval_start_index + 1);

    if values.init_if_missing(&initial_state.pose, start, false) {
        add_field_containment_factor(graph, start, config);
    }
    if values.init_if_missing(&initial_state.pose, end, is_empty_bridge_interval) {
        add_field_containment_factor(graph, end, config);
        let gp_residual = if is_empty_bridge_interval {
            GaussianProcessPriorFactor::new_zero_start_velocity_bridge(
                config.knot_spacing.as_secs_f64(),
                &config.gyroscope_process_noise,
                &config.accelerometer_process_noise,
                EMPTY_INTERVAL_PROCESS_COVARIANCE_SCALE,
            )
        } else {
            GaussianProcessPriorFactor::new(
                config.knot_spacing.as_secs_f64(),
                &config.gyroscope_process_noise,
                &config.accelerometer_process_noise,
            )
        };

        let gp_factor = FactorBuilder::new(gp_residual, (start, end)).build();

        graph.add_factor(gp_factor);
    }
}

fn add_field_containment_factor(graph: &mut Graph, state: State, config: &BackendConfiguration) {
    let containment = config.field_containment;
    let residual =
        FieldContainmentFactor::new(containment.x_limit, containment.y_limit, containment.sigma);
    graph.add_factor(FactorBuilder::new(residual, state).build());
}

#[derive(Debug, Clone)]
struct IntervalAssigner {
    first_observed_time: OnceCell<SystemTime>,
    interval_length: Duration,
}

impl IntervalAssigner {
    pub fn new(interval_length: Duration) -> Self {
        Self {
            first_observed_time: OnceCell::new(),
            interval_length,
        }
    }

    pub fn assign_interval(&self, measurement_time: SystemTime) -> Option<u32> {
        let start_time = self.first_observed_time.get_or_init(|| measurement_time);
        let time_since_start = measurement_time.duration_since(*start_time).ok()?;
        Some(self.index_of_duration(time_since_start))
    }

    pub fn current_interval_start_time(&self, measurement_time: SystemTime) -> Option<SystemTime> {
        let index = self.assign_interval(measurement_time)?;
        self.interval_start_time(index)
    }

    pub fn interval_start_time(&self, index: u32) -> Option<SystemTime> {
        let start_time = *self.first_observed_time.get()?;
        Some(start_time + self.interval_length * index)
    }

    pub fn index_of_duration(&self, duration: Duration) -> u32 {
        let index = duration.as_nanos() / self.interval_length.as_nanos();
        index.try_into().expect("does not fit interval index")
    }
}

trait InitStateExt {
    fn init_if_missing(&mut self, initial: &SE23, index: State, reset_velocity: bool) -> bool;
}

impl InitStateExt for Values {
    fn init_if_missing(&mut self, initial: &SE23, index: State, reset_velocity: bool) -> bool {
        if self.get(index).is_some() {
            return false;
        }

        let previous = index
            .0
            .checked_sub(1)
            .and_then(|i| self.get(State(i)))
            .unwrap_or(initial);

        self.insert(
            index,
            if reset_velocity {
                zero_velocity_pose(previous)
            } else {
                previous.clone()
            },
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use super::*;
    use booster::ImuState;
    use factrs::{
        core::{SE3, SO3, Values},
        optimizers::OptObserver,
        traits::{Optimizer, Variable},
    };
    use linear_algebra::IntoFramed;

    struct StatePresenceObserver {
        state: State,
        seen: Arc<AtomicBool>,
    }

    impl OptObserver for StatePresenceObserver {
        fn on_step(&self, values: &Values, _time: i64) {
            if values.get_raw(self.state).is_some() {
                self.seen.store(true, Ordering::SeqCst);
            }
        }
    }

    fn backend_configuration() -> BackendConfiguration {
        BackendConfiguration {
            knot_spacing: Duration::from_millis(200),
            max_optimization_window: Duration::from_secs(3),
            optimizer_max_iterations: 1,
            gyroscope_noise: Matrix3::identity() * 0.05_f64.powi(2),
            accelerometer_noise: Matrix3::identity() * 0.5_f64.powi(2),
            use_accelerometer_measurements: false,
            gyroscope_process_noise: Matrix3::identity() * 0.01,
            accelerometer_process_noise: Matrix3::identity() * 0.01,
            roll_pitch_yaw_noise: Matrix3::identity() * 0.01,
            visual_feature_noise: Matrix2::identity() * 5.0,
            pose_hint_visual_feature_noise: Matrix2::identity() * 100.0,
            pose_hint_visual_huber_threshold: 2.0,
            visual_odometry_noise: SMatrix::<f64, 6, 6>::identity() * 0.05,
            foot_ground_sigma: 0.01,
            field_containment: FieldContainmentConfiguration::default(),
            gravity: Vector3::new(0.0, 0.0, 9.81),
        }
    }

    fn assigner(interval: Duration) -> IntervalAssigner {
        let assigner = IntervalAssigner::new(interval);
        assigner
            .first_observed_time
            .set(SystemTime::UNIX_EPOCH)
            .expect("could not set start time");
        assigner
    }

    fn stationary_imu(time: SystemTime) -> SensorMeasurement {
        SensorMeasurement::Imu(ImuMeasurement {
            time,
            state: ImuState {
                roll_pitch_yaw: Vector3::zeros().framed(),
                angular_velocity: Vector3::zeros().framed(),
                linear_acceleration: Vector3::new(0.0, 0.0, 9.81).framed(),
            },
        })
    }

    fn visual_odometry(
        previous_time: SystemTime,
        current_time: SystemTime,
        x: f64,
    ) -> SensorMeasurement {
        SensorMeasurement::VisualOdometry(VisualOdometryMeasurement {
            previous_time,
            current_time,
            robot_delta: SE3::from_rot_trans(SO3::identity(), Vector3::new(x, 0.0, 0.0)),
        })
    }

    fn global_pose(time: SystemTime, x: f64, y: f64, z: f64) -> SensorMeasurement {
        SensorMeasurement::GlobalPose(GlobalPoseMeasurement {
            time,
            robot_to_field: SE3::from_rot_trans(SO3::identity(), Vector3::new(x, y, z)),
        })
    }

    fn visual_reprojection_measurement(
        time: SystemTime,
        detection_x: f64,
    ) -> VisualReprojectionMeasurement {
        VisualReprojectionMeasurement {
            time,
            detection: nalgebra::point![detection_x, 0.0],
            field_point: nalgebra::point![0.0, 0.0, 2.0],
            robot_to_camera: SE3::identity(),
        }
    }

    fn visual_reprojection(time: SystemTime) -> SensorMeasurement {
        SensorMeasurement::Visual(vec![visual_reprojection_measurement(time, 0.0)])
    }

    fn pose_hint_visual_reprojection(time: SystemTime) -> SensorMeasurement {
        SensorMeasurement::PoseHintVisual(vec![visual_reprojection_measurement(time, 0.0)])
    }

    fn visual_reprojection_factor_count(backend: &mut VinsBackend, state: State) -> usize {
        backend
            .optimizer
            .graph_mut()
            .factors_for_residual::<VisualReprojectionFactor, _>((
                state,
                State(state.0 + 1),
                CameraIntrinsics(0),
            ))
            .count()
    }

    fn visual_reprojection_factor_dimensions(backend: &VinsBackend, state: State) -> Vec<usize> {
        backend
            .optimizer
            .graph()
            .factors_for_residual::<VisualReprojectionFactor, _>((
                state,
                State(state.0 + 1),
                CameraIntrinsics(0),
            ))
            .map(|factor| {
                factor
                    .try_dim_out(backend.values())
                    .expect("visual reprojection factor should have a dimension")
            })
            .collect()
    }

    fn visual_odometry_factor_count(backend: &mut VinsBackend, state: State) -> usize {
        backend
            .optimizer
            .graph_mut()
            .factors_for_residual::<VisualOdometryFactor, _>((state, State(state.0 + 1)))
            .count()
    }

    fn visual_odometry_factor_dimensions(backend: &VinsBackend, state: State) -> Vec<usize> {
        backend
            .optimizer
            .graph()
            .factors_for_residual::<VisualOdometryFactor, _>((state, State(state.0 + 1)))
            .map(|factor| {
                factor
                    .try_dim_out(backend.values())
                    .expect("visual odometry factor should have a dimension")
            })
            .collect()
    }

    fn adjacent_visual_odometry_factor_count(backend: &mut VinsBackend, state: State) -> usize {
        backend
            .optimizer
            .graph_mut()
            .factors_for_residual::<AdjacentVisualOdometryFactor, _>((
                state,
                State(state.0 + 1),
                State(state.0 + 2),
            ))
            .count()
    }

    fn adjacent_visual_odometry_factor_dimensions(
        backend: &VinsBackend,
        state: State,
    ) -> Vec<usize> {
        backend
            .optimizer
            .graph()
            .factors_for_residual::<AdjacentVisualOdometryFactor, _>((
                state,
                State(state.0 + 1),
                State(state.0 + 2),
            ))
            .map(|factor| {
                factor
                    .try_dim_out(backend.values())
                    .expect("adjacent visual odometry factor should have a dimension")
            })
            .collect()
    }

    fn field_containment_factor_count(backend: &VinsBackend, state: State) -> usize {
        backend
            .optimizer
            .graph()
            .factors_for_residual::<FieldContainmentFactor, _>(state)
            .count()
    }

    fn foot_heights(time: SystemTime) -> SensorMeasurement {
        SensorMeasurement::FootHeights(FootHeightMeasurement {
            time,
            left_sole_in_robot: nalgebra::Point3::new(0.0, 0.05, -0.2),
            right_sole_in_robot: nalgebra::Point3::new(0.0, -0.05, -0.2),
        })
    }

    fn moving_initial_state(velocity: Vector3<f64>) -> InitialState {
        InitialState {
            pose: SE23::from_rot_vel_trans(SO3::identity(), velocity, Vector3::zeros()),
            ..InitialState::default()
        }
    }

    fn initial_state_with_intrinsics(
        focal_lengths: nalgebra::Vector2<f64>,
        optical_center: nalgebra::Vector2<f64>,
    ) -> InitialState {
        InitialState {
            camera_intrinsics: crate::camera_intrinsics::CameraIntrinsics::new(
                focal_lengths,
                optical_center,
            ),
            ..InitialState::default()
        }
    }

    #[test]
    fn solve_once_before_measurements_preserves_initial_values() {
        let (_measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );

        let _ = backend.solve_once().expect("empty solve should succeed");
        let _ = backend
            .solve_once()
            .expect("repeated empty solve should succeed");

        assert!(backend.values().get_raw(CameraIntrinsics(0)).is_some());
        assert!(backend.values().get_raw(State(0)).is_some());
    }

    #[test]
    fn field_containment_factors_are_attached_to_initialized_states() {
        let (_measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        assert_eq!(field_containment_factor_count(&backend, State(0)), 1);

        backend
            .ingest_sensor_measurements([stationary_imu(start)])
            .expect("IMU should ingest");
        backend
            .ingest_sensor_measurements([stationary_imu(start + Duration::from_millis(50))])
            .expect("IMU should ingest");

        assert_eq!(field_containment_factor_count(&backend, State(0)), 1);
        assert_eq!(field_containment_factor_count(&backend, State(1)), 1);
    }

    #[test]
    fn solve_once_result_includes_camera_intrinsics() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let initial_state = initial_state_with_intrinsics(
            nalgebra::vector![200.0, 210.0],
            nalgebra::vector![250.0, 240.0],
        );
        let mut backend = VinsBackend::new(
            backend_configuration(),
            initial_state,
            measurement_receiver,
            result_sender,
        );

        measurement_sender
            .send(stationary_imu(SystemTime::UNIX_EPOCH))
            .expect("IMU should send");

        let result = backend
            .solve_once()
            .expect("solve should succeed")
            .expect("result should be available");

        assert_eq!(
            result.camera_intrinsics.focals(),
            nalgebra::vector![200.0, 210.0]
        );
        assert_eq!(
            result.camera_intrinsics.optical_center(),
            nalgebra::vector![250.0, 240.0]
        );
    }

    #[test]
    fn visual_reprojection_measurements_create_interval_factor() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        measurement_sender
            .send(visual_reprojection(start))
            .expect("visual reprojection should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert!(backend.values().get_raw(State(0)).is_some());
        assert!(backend.values().get_raw(State(1)).is_some());
        assert_eq!(visual_reprojection_factor_count(&mut backend, State(0)), 1);
    }

    #[test]
    fn pose_hint_visual_reprojection_measurements_create_separate_factor() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        measurement_sender
            .send(pose_hint_visual_reprojection(start))
            .expect("pose-hint visual reprojection should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert_eq!(visual_reprojection_factor_count(&mut backend, State(0)), 1);
    }

    #[test]
    fn first_visual_batch_measurement_is_ingested() {
        let (_measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        backend
            .ingest_sensor_measurements([visual_reprojection(start)])
            .expect("first measurement should ingest");

        assert!(backend.values().get_raw(State(0)).is_some());
        assert!(backend.values().get_raw(State(1)).is_some());
        assert_eq!(visual_reprojection_factor_count(&mut backend, State(0)), 1);
    }

    #[test]
    fn visual_reprojection_huber_is_per_measurement_residual_block() {
        let (_measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        backend
            .ingest_sensor_measurements([SensorMeasurement::Visual(vec![
                visual_reprojection_measurement(start, 0.0),
                visual_reprojection_measurement(start, 1.0),
            ])])
            .expect("visual measurements should ingest");

        assert_eq!(visual_reprojection_factor_count(&mut backend, State(0)), 2);
        assert_eq!(
            visual_reprojection_factor_dimensions(&backend, State(0)),
            vec![2, 2]
        );
    }

    #[test]
    fn visual_odometry_measurements_create_interval_factor() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        measurement_sender
            .send(visual_odometry(
                start,
                start + Duration::from_millis(100),
                0.1,
            ))
            .expect("visual odometry should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert!(backend.values().get_raw(State(0)).is_some());
        assert!(backend.values().get_raw(State(1)).is_some());
        assert_eq!(visual_odometry_factor_count(&mut backend, State(0)), 1);
    }

    #[test]
    fn visual_odometry_huber_is_per_delta_residual_block() {
        let (_measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        backend
            .ingest_sensor_measurements([
                visual_odometry(start, start + Duration::from_millis(50), 0.05),
                visual_odometry(
                    start + Duration::from_millis(60),
                    start + Duration::from_millis(100),
                    0.04,
                ),
            ])
            .expect("visual odometry measurements should ingest");

        assert_eq!(visual_odometry_factor_count(&mut backend, State(0)), 2);
        assert_eq!(
            visual_odometry_factor_dimensions(&backend, State(0)),
            vec![6, 6]
        );
    }

    #[test]
    fn visual_odometry_measurements_create_adjacent_interval_factor() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        measurement_sender
            .send(stationary_imu(start))
            .expect("IMU should send");
        measurement_sender
            .send(visual_odometry(
                start + Duration::from_millis(100),
                start + Duration::from_millis(200),
                0.1,
            ))
            .expect("visual odometry should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert_eq!(visual_odometry_factor_count(&mut backend, State(0)), 0);
        assert_eq!(
            adjacent_visual_odometry_factor_count(&mut backend, State(0)),
            1
        );
    }

    #[test]
    fn adjacent_visual_odometry_huber_is_per_delta_residual_block() {
        let (_measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        backend
            .ingest_sensor_measurements([
                stationary_imu(start),
                visual_odometry(
                    start + Duration::from_millis(100),
                    start + Duration::from_millis(200),
                    0.1,
                ),
                visual_odometry(
                    start + Duration::from_millis(120),
                    start + Duration::from_millis(200),
                    0.08,
                ),
            ])
            .expect("adjacent visual odometry measurements should ingest");

        assert_eq!(
            adjacent_visual_odometry_factor_count(&mut backend, State(0)),
            2
        );
        assert_eq!(
            adjacent_visual_odometry_factor_dimensions(&backend, State(0)),
            vec![6, 6]
        );
    }

    #[test]
    fn invalid_visual_odometry_measurement_does_not_create_empty_factor() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        measurement_sender
            .send(visual_odometry(start, start, 0.0))
            .expect("visual odometry should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert_eq!(visual_odometry_factor_count(&mut backend, State(0)), 0);

        measurement_sender
            .send(visual_odometry(
                start,
                start + Duration::from_millis(100),
                0.1,
            ))
            .expect("visual odometry should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert_eq!(visual_odometry_factor_count(&mut backend, State(0)), 1);
    }

    #[test]
    fn global_pose_measurement_resets_backend_state() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;
        measurement_sender
            .send(visual_odometry(
                start,
                start + Duration::from_millis(100),
                3.0,
            ))
            .expect("visual odometry should send");
        let _ = backend.solve_once().expect("initial solve should succeed");

        let reset_time = start + Duration::from_secs(1);
        measurement_sender
            .send(global_pose(reset_time, 1.0, 2.0, 0.45))
            .expect("global pose should send");

        let result = backend
            .solve_once()
            .expect("reset solve should succeed")
            .expect("reset should produce a result");

        assert_eq!(result.time, reset_time);
        assert!((result.latest_pose.xyz() - Vector3::new(1.0, 2.0, 0.45)).norm() < 1.0e-9);
        assert!(result.latest_pose.uvw().norm() < 1.0e-9);
        assert_eq!(visual_odometry_factor_count(&mut backend, State(0)), 0);
    }

    #[test]
    fn foot_height_measurements_create_interval_factor() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let start = SystemTime::UNIX_EPOCH;

        measurement_sender
            .send(foot_heights(start))
            .expect("foot heights should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert!(backend.values().get_raw(State(0)).is_some());
        assert!(backend.values().get_raw(State(1)).is_some());
        assert_eq!(
            backend
                .optimizer
                .graph_mut()
                .factors_for_residual::<IntervalFootAboveGroundFactor, _>((State(0), State(1)))
                .count(),
            1
        );
    }

    #[test]
    fn foot_height_measurements_before_solver_start_are_dropped() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let solver_start = SystemTime::UNIX_EPOCH + Duration::from_secs(1);

        measurement_sender
            .send(stationary_imu(solver_start))
            .expect("IMU should send");
        measurement_sender
            .send(foot_heights(solver_start - Duration::from_millis(100)))
            .expect("foot heights should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert_eq!(
            backend
                .optimizer
                .graph_mut()
                .factors_for_residual::<IntervalFootAboveGroundFactor, _>((State(0), State(1)))
                .count(),
            0
        );
    }

    #[test]
    fn imu_gaps_are_bridged_with_empty_intervals() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );

        let start = SystemTime::UNIX_EPOCH;
        measurement_sender
            .send(stationary_imu(start))
            .expect("first IMU should send");
        measurement_sender
            .send(stationary_imu(start + Duration::from_secs(1)))
            .expect("second IMU should send");

        let _ = backend.solve_once().expect("solve should succeed");

        for index in 0..=6 {
            assert!(
                backend.values().get_raw(State(index)).is_some(),
                "state {index} should be initialized"
            );
        }
        assert_eq!(backend.highest_initialized_interval, Some(5));
    }

    #[test]
    fn long_gap_bridge_states_do_not_inherit_stale_velocity() {
        let (_measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut backend = VinsBackend::new(
            backend_configuration(),
            moving_initial_state(Vector3::new(0.0, 3.0, 0.0)),
            measurement_receiver,
            result_sender,
        );

        backend
            .interval_assigner
            .assign_interval(SystemTime::UNIX_EPOCH)
            .expect("first interval should be assigned");

        backend.init_intervals_through(0);
        backend.init_intervals_through(LONG_GAP_MIN_EMPTY_INTERVALS + 1);

        let boundary_state = backend
            .values()
            .get(State(1))
            .expect("gap boundary state should be initialized");
        let bridge_state = backend
            .values()
            .get(State(2))
            .expect("bridge state should be initialized");
        let target_state = backend
            .values()
            .get(State(LONG_GAP_MIN_EMPTY_INTERVALS + 1))
            .expect("target state should be initialized");

        assert!(boundary_state.uvw().norm() < 1.0e-9);
        assert!(bridge_state.uvw().norm() < 1.0e-9);
        assert!(target_state.uvw().norm() < 1.0e-9);
    }

    #[test]
    fn marginalization_happens_after_optimizer_step() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut config = backend_configuration();
        config.max_optimization_window = Duration::from_millis(400);
        let mut backend = VinsBackend::new(
            config,
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );
        let state_seen_during_optimization = Arc::new(AtomicBool::new(false));
        backend.optimizer.add_observer(StatePresenceObserver {
            state: State(0),
            seen: Arc::clone(&state_seen_during_optimization),
        });

        let start = SystemTime::UNIX_EPOCH;
        measurement_sender
            .send(stationary_imu(start))
            .expect("first IMU should send");
        measurement_sender
            .send(stationary_imu(start + Duration::from_secs(2)))
            .expect("later IMU should send");
        measurement_sender
            .send(visual_odometry(
                start,
                start + Duration::from_millis(100),
                0.1,
            ))
            .expect("visual odometry should send");

        let _ = backend.solve_once().expect("solve should succeed");

        assert!(
            state_seen_during_optimization.load(Ordering::SeqCst),
            "state 0 must still be available while optimizing the current batch"
        );
        assert!(
            backend.values().get_raw(State(0)).is_none(),
            "state 0 should be marginalized after optimization"
        );
    }

    #[test]
    fn late_measurements_for_marginalized_intervals_are_skipped() {
        let (measurement_sender, measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::watch::channel(None);
        let mut config = backend_configuration();
        config.max_optimization_window = Duration::from_millis(400);
        let mut backend = VinsBackend::new(
            config,
            InitialState::default(),
            measurement_receiver,
            result_sender,
        );

        let start = SystemTime::UNIX_EPOCH;
        measurement_sender
            .send(stationary_imu(start))
            .expect("first IMU should send");
        measurement_sender
            .send(stationary_imu(start + Duration::from_secs(2)))
            .expect("later IMU should send");
        let _ = backend.solve_once().expect("solve should succeed");
        assert!(
            backend.values().get_raw(State(0)).is_none(),
            "state 0 should have been marginalized"
        );

        measurement_sender
            .send(stationary_imu(start + Duration::from_millis(100)))
            .expect("late IMU should send");
        let _ = backend
            .solve_once()
            .expect("late marginalized measurement should be skipped");

        assert!(
            backend.values().get_raw(State(0)).is_none(),
            "late measurement must not reintroduce marginalized state 0"
        );
    }

    #[test]
    fn test_get_previous_interval_start_time() {
        let interval = Duration::from_millis(200);
        let assigner = assigner(interval);

        // 1. Middle of an interval
        // 265ms since epoch should snap back to 200ms
        let t1 = SystemTime::UNIX_EPOCH + Duration::from_millis(265);
        let expected1 = SystemTime::UNIX_EPOCH + interval;
        assert_eq!(assigner.current_interval_start_time(t1), Some(expected1));

        // 2. Exact boundary
        // 200ms since epoch should stay at 200ms
        let t2 = SystemTime::UNIX_EPOCH + Duration::from_millis(200);
        let expected2 = SystemTime::UNIX_EPOCH + interval;
        assert_eq!(assigner.current_interval_start_time(t2), Some(expected2));

        // 3. Just before a boundary
        // 399ms since epoch should snap back to 200ms
        let t3 = SystemTime::UNIX_EPOCH + Duration::from_millis(399);
        let expected3 = SystemTime::UNIX_EPOCH + interval;
        assert_eq!(assigner.current_interval_start_time(t3), Some(expected3));

        // 4. Very early time
        // 50ms since epoch with 100ms interval should snap to 0 (Unix Epoch)
        let t4 = SystemTime::UNIX_EPOCH + Duration::from_millis(50);
        let expected4 = SystemTime::UNIX_EPOCH;
        assert_eq!(assigner.current_interval_start_time(t4), Some(expected4));
    }

    #[test]
    fn test_large_intervals() {
        let interval = Duration::from_secs(1);
        let assigner = assigner(interval);

        // 10.9 seconds -> 10.0 seconds
        let t = SystemTime::UNIX_EPOCH + Duration::from_millis(10900);
        let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        assert_eq!(assigner.current_interval_start_time(t), Some(expected));
    }
}

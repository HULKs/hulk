use std::time::SystemTime;

use factrs::core::{GaussNewton, Values};

use crate::{
    initial_state::InitialState,
    interval_measurement::IntervalMeasurements,
    measurements::{ImuMeasurement, SensorMeasurement},
};

use tokio::sync::{
    mpsc::{UnboundedReceiver, error::TryRecvError},
    watch,
};

mod configuration;
mod diagnostics;
mod error;
mod foot_height;
mod graph;
mod imu;
mod interval_assigner;
mod odometer;
mod optimization_result;
mod optimize;
mod reset;
mod state;
mod visual;
mod visual_odometry;

pub use configuration::{BackendConfiguration, FieldContainmentConfiguration};
use diagnostics::ResidualDiagnosticsAccumulator;
pub use diagnostics::{BackendOptimizerStatus, BackendSolveDiagnostics, ResidualDiagnostics};
pub use error::VinsBackendError;
use graph::initialize_graph;
use graph::optimizer_from_graph;
use interval_assigner::IntervalAssigner;
pub use optimization_result::OptimizationResult;

const INITIAL_CAMERA_INTRINSICS_PRIOR_SIGMA: f64 = 1e-3;
const INITIAL_POSE_PRIOR_SIGMA: f64 = 10.0;
// Empty intervals are inserted only to keep graph components connected across
// dropped recording data. They use zero-start-velocity GP priors so stale
// pre-gap velocity is not treated as measured ballistic motion.
const EMPTY_INTERVAL_PROCESS_COVARIANCE_SCALE: f64 = 10.0;
const LONG_GAP_MIN_EMPTY_INTERVALS: u32 = 5;

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
    last_imu_knot_orientation: Option<imu::ImuKnotOrientation>,
    /// Optimizer status from the most recent solve.
    last_optimizer_status: Option<BackendOptimizerStatus>,
    last_imu_kinematics_measurement_time: Option<SystemTime>,
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
        let latest_reset = new_measurements.latest_reset().cloned();
        let latest_global_pose = new_measurements.latest_global_pose().cloned();
        match (latest_reset, latest_global_pose) {
            (Some(reset), Some(global_pose)) if reset.time >= global_pose.time => {
                self.reset_to_initial_state(reset.initial_state, reset.time);
                new_measurements.retain_at_or_after(reset.time);
            }
            (Some(_), Some(global_pose)) | (None, Some(global_pose)) => {
                self.reset_to_global_pose(global_pose.clone());
                new_measurements.retain_at_or_after(global_pose.time);
            }
            (Some(reset), None) => {
                self.reset_to_initial_state(reset.initial_state, reset.time);
                new_measurements.retain_at_or_after(reset.time);
            }
            (None, None) => {}
        }

        self.ingest_imu(new_measurements.imu);
        self.ingest_visual(new_measurements.visual);
        self.ingest_pose_hint_visual(new_measurements.pose_hint_visual);
        self.ingest_visual_odometry(new_measurements.visual_odometry);
        self.ingest_odometer(new_measurements.odometer);
        self.ingest_foot_heights(new_measurements.foot_heights);

        Ok(())
    }
}

#[cfg(test)]
mod tests;

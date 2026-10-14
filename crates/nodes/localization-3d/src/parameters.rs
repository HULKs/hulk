use std::time::Duration;

use coordinate_systems::Robot;
use linear_algebra::Vector3;
use ros_z::Message;
use serde::{Deserialize, Serialize};

/// Restart-required timing and interpolation policy. Durations are seconds/nanoseconds.
#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct TimingParameters {
    pub trajectory_spacing: Duration,
    pub bias_spacing: Duration,
    pub preintegration_interval: Duration,
    pub optimization_window: Duration,
    pub max_imu_gap: Duration,
    pub max_camera_gap: Duration,
    pub startup_kinematics_freshness: Duration,
}

impl Default for TimingParameters {
    fn default() -> Self {
        Self {
            trajectory_spacing: Duration::from_millis(200),
            bias_spacing: Duration::from_secs(5),
            preintegration_interval: Duration::from_millis(100),
            optimization_window: Duration::from_secs(2),
            max_imu_gap: Duration::from_millis(20),
            max_camera_gap: Duration::from_millis(20),
            startup_kinematics_freshness: Duration::from_millis(100),
        }
    }
}

impl TimingParameters {
    pub(crate) fn knot_ns(&self) -> i64 {
        self.trajectory_spacing.as_nanos() as i64
    }
    pub(crate) fn bias_ns(&self) -> i64 {
        self.bias_spacing.as_nanos() as i64
    }
    pub(crate) fn interval_ns(&self) -> i64 {
        self.preintegration_interval.as_nanos() as i64
    }
    pub(crate) fn window_ns(&self) -> i64 {
        self.optimization_window.as_nanos() as i64
    }
}

/// Restart-required factor weights and numerical sampling policy.
#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ModelParameters {
    /// Robust loss threshold in whitened residual units.
    pub huber_threshold: f64,
    /// Bearing-factor minimum landmark distance, metres.
    pub min_landmark_range: f64,
    /// Intrinsic prior standard deviation, pixels.
    pub intrinsic_prior_sigma: f64,
    /// Gauge standard deviations, metres and radians respectively.
    pub anchor_xy_sigma: f64,
    pub anchor_yaw_sigma: f64,
    /// Ground contact standard deviation, metres.
    pub foot_sigma: f64,
    /// VO standard deviation, metres/radians.
    pub visual_odometry_sigma: f64,
    /// Relative yaw variance per observation, radians squared.
    pub relative_yaw_variance: f64,
    /// Rotational process spectral density, rad²/s.
    pub rotation_process_variance: f64,
    /// Variance multiplier for motion priors spanning missing data.
    pub gap_uncertainty_multiplier: f64,
    pub prediction_gap_segments: i64,
    /// Gravitational acceleration, m/s².
    pub gravity: f64,
    /// Up-direction standard deviation at a gap boundary, radians.
    pub gap_tilt_sigma: f64,
    /// Fraction of each trajectory segment sampled for field containment, [0, 1].
    pub containment_tau: f64,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            huber_threshold: 2.0,
            min_landmark_range: 0.01,
            intrinsic_prior_sigma: 0.001,
            anchor_xy_sigma: 0.001,
            anchor_yaw_sigma: 0.001,
            foot_sigma: 0.01,
            visual_odometry_sigma: 0.1,
            relative_yaw_variance: 2e-5,
            rotation_process_variance: 0.01,
            gap_uncertainty_multiplier: 10.0,
            prediction_gap_segments: 5,
            gravity: 9.81,
            gap_tilt_sigma: 0.1,
            containment_tau: 0.5,
        }
    }
}

/// Live solver settings; tolerances use fagra's gradient/step/cost norms.
#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SolverParameters {
    pub max_trials: usize,
    pub max_iterations: usize,
    pub gradient_tolerance: f64,
    pub step_tolerance: f64,
    pub cost_tolerance: f64,
}
impl Default for SolverParameters {
    fn default() -> Self {
        Self {
            max_trials: 8,
            max_iterations: 10,
            gradient_tolerance: 1e-3,
            step_tolerance: 1e-5,
            cost_tolerance: 1e-8,
        }
    }
}

/// Live visual admission/validation gates, also used during alignment and recovery.
#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct VisualParameters {
    pub min_associations: usize,
    pub max_associations: usize,
    /// Metres in front of the camera.
    pub min_reprojection_depth: f64,
    pub min_detection_separation_px: f32,
    pub min_landmark_separation_m: f32,
    pub max_rms_px: f64,
    /// Minimum downward component of the rotated pinhole bearing for ground-plane seeding.
    pub min_downward_ray: f64,
}
impl Default for VisualParameters {
    fn default() -> Self {
        Self {
            min_associations: 3,
            max_associations: 32,
            min_reprojection_depth: 0.01,
            min_detection_separation_px: 1.0,
            min_landmark_separation_m: 1e-4,
            max_rms_px: 10.0,
            min_downward_ray: 1e-6,
        }
    }
}

/// Restart-required queue/cache capacities, in messages.
#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct InputParameters {
    pub imu_queue: usize,
    pub kinematics_queue: usize,
    pub visual_queue: usize,
    pub visual_odometry_queue: usize,
    pub kinematic_odometry_queue: usize,
    pub camera_cache: usize,
    pub kinematics_cache: usize,
    pub field_cache: usize,
}
impl Default for InputParameters {
    fn default() -> Self {
        Self {
            imu_queue: 500,
            kinematics_queue: 500,
            visual_queue: 60,
            visual_odometry_queue: 60,
            kinematic_odometry_queue: 60,
            camera_cache: 1500,
            kinematics_cache: 1000,
            field_cache: 1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ImuPreintegrationParameters {
    /// Continuous gyro noise density, rad/s sqrt(s).
    pub gyroscope_noise_density: f64,
    /// Position integration/model-error density, m/sqrt(s), as in GTSAM.
    pub integration_noise_density: f64,
    /// SDK up-direction sigma for reference_duration of covered measurements.
    pub tilt_sigma: f64,
    /// Reference exposure duration for tilt information, independent of interval partitioning.
    pub reference_duration: Duration,
    /// Instantaneous, not integrated, newest gyro measurement sigma (rad/s).
    pub terminal_gyroscope_sigma: f64,
    /// Reintegrate between solves beyond these reference-bias changes.
    pub gyroscope_reintegration_threshold: f64,
    pub accelerometer_reintegration_threshold: f64,
}

impl Default for ImuPreintegrationParameters {
    fn default() -> Self {
        Self {
            gyroscope_noise_density: (2e-5_f64).sqrt(),
            integration_noise_density: 1e-4,
            tilt_sigma: 0.02,
            reference_duration: Duration::from_millis(100),
            terminal_gyroscope_sigma: 0.1,
            gyroscope_reintegration_threshold: 0.01,
            accelerometer_reintegration_threshold: 0.1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImuBiasParameters {
    pub accelerometer_initial_sigma: f64,
    pub gyroscope_initial_sigma: f64,
    /// Bias random-walk standard deviation per sqrt(second).
    pub accelerometer_random_walk: f64,
    pub gyroscope_random_walk: f64,
}

impl Default for ImuBiasParameters {
    fn default() -> Self {
        Self {
            accelerometer_initial_sigma: 1.0,
            gyroscope_initial_sigma: 0.02,
            accelerometer_random_walk: 0.002,
            gyroscope_random_walk: 0.0002,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccelerometerParameters {
    /// SDK specific force is already expressed in Robot axes, in m/s².
    pub bias: Vector3<Robot, f64>,
    /// Per-axis multiplicative calibration, applied after subtracting bias.
    pub scale: nalgebra::Vector3<f64>,
    /// Robot origin to IMU, expressed in Robot, in metres.
    pub position: Vector3<Robot, f64>,
    /// Specific-force white-noise density in m/s² sqrt(s), including model error.
    pub noise_density: f64,
}

impl Default for AccelerometerParameters {
    fn default() -> Self {
        Self {
            bias: Vector3::zeros(),
            scale: nalgebra::Vector3::repeat(1.0),
            position: Vector3::zeros(),
            noise_density: 0.3,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KinematicOdometryNoise {
    /// Forward/lateral displacement noise floor, in metres.
    pub position_sigma: nalgebra::Vector2<f64>,
    /// Forward/lateral displacement variance growth, in m²/s.
    pub translation_variance_per_second: nalgebra::Vector2<f64>,
}

impl Default for KinematicOdometryNoise {
    fn default() -> Self {
        Self {
            position_sigma: nalgebra::Vector2::repeat(0.005),
            translation_variance_per_second: nalgebra::Vector2::repeat(0.001),
        }
    }
}

/// Runtime parameters for the 3D localization node.
#[derive(Clone, Debug, Deserialize, Serialize, Message, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Localization3dParameters {
    #[serde(default)]
    pub timing: TimingParameters,
    #[serde(default)]
    pub model: ModelParameters,
    #[serde(default)]
    pub solver: SolverParameters,
    #[serde(default)]
    pub visual: VisualParameters,
    #[serde(default)]
    pub inputs: InputParameters,
    #[serde(default)]
    pub kinematic_odometry_noise: Option<KinematicOdometryNoise>,
    pub accelerometer: Option<AccelerometerParameters>,
    #[serde(default)]
    pub imu_bias: ImuBiasParameters,
    #[serde(default)]
    pub imu_preintegration: ImuPreintegrationParameters,
    /// Broad initialization distributions, not measured standing height or zero velocity.
    pub initial_height_sigma: f64,
    pub initial_velocity_sigma: f64,
    /// Squared normalized innovation allowed between old and candidate height predictions.
    pub recovery_height_gate: f64,
    /// Maximum discrepancy between estimated and measured up directions, in radians.
    pub max_tilt_error: f64,
    /// Translational white-noise-on-acceleration spectral density.
    pub accelerometer_process_noise_variance: f64,
    /// Pixel-noise variance used to set isotropic angular noise for accepted visual
    /// associations: sigma_theta = sqrt(variance) / sqrt(fx * fy), using the frame's
    /// fixed calibration. This is a near-axis approximation, not a pixel likelihood.
    pub visual_feature_noise_variance: f64,
    /// Soft field containment sigma in meters outside field plus border strip.
    pub field_containment_sigma: f64,
    /// Maximum field-heading innovation against propagated IMU heading, in radians (< pi/2).
    pub max_heading_error: f64,
    /// Maximum correction of the IMU-to-field reference during visual tracking, in rad/s.
    pub max_heading_reference_drift_per_second: f64,
    /// Time without a converged aligned backend result before localization is declared lost.
    #[serde(default = "default_tracking_timeout")]
    pub tracking_timeout: Duration,
    /// Time without a newly acknowledged, valid visual reprojection before tracking is lost.
    #[serde(default = "default_tracking_timeout")]
    pub visual_tracking_timeout: Duration,
}

fn default_tracking_timeout() -> Duration {
    Duration::from_secs(2)
}

impl Localization3dParameters {
    /// Only solver settings and pure acceptance/lifecycle gates are live. Everything
    /// else is baked into graph factors, marginal priors, interval caches or resources.
    pub(crate) fn validate_update(&self, candidate: &Self) -> Result<(), String> {
        candidate.validate()?;
        let mut live = self.clone();
        live.solver = candidate.solver.clone();
        live.visual = candidate.visual.clone();
        live.recovery_height_gate = candidate.recovery_height_gate;
        live.max_tilt_error = candidate.max_tilt_error;
        live.max_heading_error = candidate.max_heading_error;
        live.max_heading_reference_drift_per_second =
            candidate.max_heading_reference_drift_per_second;
        live.tracking_timeout = candidate.tracking_timeout;
        live.visual_tracking_timeout = candidate.visual_tracking_timeout;
        if &live != candidate {
            return Err(
                "localization timing, model, calibration, noise and input changes require restart"
                    .into(),
            );
        }
        Ok(())
    }

    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        let t = &self.timing;
        for duration in [
            t.trajectory_spacing,
            t.bias_spacing,
            t.preintegration_interval,
            t.optimization_window,
            t.max_imu_gap,
            t.max_camera_gap,
            t.startup_kinematics_freshness,
            self.imu_preintegration.reference_duration,
        ] {
            if duration.is_zero() || duration.as_nanos() > i64::MAX as u128 {
                return Err(
                    "timing durations must be positive and representable in signed nanoseconds"
                        .into(),
                );
            }
        }
        if t.bias_ns() % t.knot_ns() != 0
            || t.knot_ns() % t.interval_ns() != 0
            || t.optimization_window < t.trajectory_spacing.saturating_mul(2)
        {
            return Err("bias spacing must be a multiple of trajectory spacing, trajectory spacing a multiple of the IMU interval; window must cover at least two trajectory segments".into());
        }
        let m = &self.model;
        let v = &self.visual;
        let s = &self.solver;
        for value in [
            m.huber_threshold,
            m.min_landmark_range,
            m.intrinsic_prior_sigma,
            m.anchor_xy_sigma,
            m.anchor_yaw_sigma,
            m.foot_sigma,
            m.visual_odometry_sigma,
            m.relative_yaw_variance,
            m.rotation_process_variance,
            m.gap_uncertainty_multiplier,
            m.gravity,
            m.gap_tilt_sigma,
            v.min_reprojection_depth,
            f64::from(v.min_detection_separation_px),
            f64::from(v.min_landmark_separation_m),
            v.max_rms_px,
            v.min_downward_ray,
        ] {
            if !valid_scale(value) || !(value * value).is_finite() {
                return Err("model, visual and solver scales must be finite and positive with representable squares".into());
            }
        }
        if [s.gradient_tolerance, s.step_tolerance, s.cost_tolerance]
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("solver tolerances must be finite and nonnegative".into());
        }
        if m.prediction_gap_segments <= 0
            || m.gap_uncertainty_multiplier < 1.0
            || !m.containment_tau.is_finite()
            || !(0.0..=1.0).contains(&m.containment_tau)
            || v.min_associations < 3
            || v.max_associations < v.min_associations
            || s.max_trials == 0
            || s.max_iterations == 0
        {
            return Err(
                "invalid model sampling, visual association limits or solver iteration limits"
                    .into(),
            );
        }
        let i = &self.inputs;
        if [
            i.imu_queue,
            i.kinematics_queue,
            i.visual_queue,
            i.visual_odometry_queue,
            i.kinematic_odometry_queue,
            i.camera_cache,
            i.kinematics_cache,
            i.field_cache,
        ]
        .contains(&0)
        {
            return Err("input queue/cache capacities must be positive".into());
        }
        let p = &self.imu_preintegration;
        for value in [
            p.gyroscope_noise_density,
            p.integration_noise_density,
            p.tilt_sigma,
            p.terminal_gyroscope_sigma,
            p.gyroscope_reintegration_threshold,
            p.accelerometer_reintegration_threshold,
        ] {
            if !valid_scale(value) || !(value * value).is_finite() {
                return Err(
                    "IMU preintegration noise and thresholds must be finite and positive".into(),
                );
            }
        }
        for value in [
            self.imu_bias.accelerometer_initial_sigma,
            self.imu_bias.gyroscope_initial_sigma,
            self.imu_bias.accelerometer_random_walk,
            self.imu_bias.gyroscope_random_walk,
        ] {
            if !valid_scale(value) || !(value * value).is_finite() {
                return Err("IMU bias uncertainties must be finite and positive".into());
            }
        }
        if !self.max_heading_error.is_finite()
            || self.max_heading_error <= 0.0
            || self.max_heading_error >= std::f64::consts::FRAC_PI_2
            || !self.max_heading_reference_drift_per_second.is_finite()
            || self.max_heading_reference_drift_per_second < 0.0
        {
            return Err(
                "heading error must be in (0, pi/2), and heading drift finite and >= 0".into(),
            );
        }
        if let Some(noise) = &self.kinematic_odometry_noise
            && !noise
                .position_sigma
                .iter()
                .chain(noise.translation_variance_per_second.iter())
                .all(|v| valid_scale(*v) && (*v * *v).is_finite())
        {
            return Err("kinematic odometry noise must be finite and > 0".into());
        }
        for value in [
            self.initial_height_sigma,
            self.initial_velocity_sigma,
            self.recovery_height_gate,
        ] {
            if !valid_scale(value) || !(value * value).is_finite() {
                return Err(
                    "initial uncertainties and recovery height gate must be finite and positive"
                        .into(),
                );
            }
        }
        if !self.max_tilt_error.is_finite()
            || self.max_tilt_error <= 0.0
            || self.max_tilt_error >= std::f64::consts::FRAC_PI_2
        {
            return Err("tilt error must be in (0, pi/2)".into());
        }
        if let Some(accel) = &self.accelerometer
            && (!accel
                .bias
                .inner
                .iter()
                .chain(accel.position.inner.iter())
                .all(|v| v.is_finite())
                || !accel
                    .scale
                    .iter()
                    .all(|v| valid_scale(*v) && (v * v).is_finite())
                || !valid_scale(accel.noise_density)
                || !(accel.noise_density * accel.noise_density).is_finite())
        {
            return Err("invalid accelerometer calibration or noise density".into());
        }
        if !valid_scale(self.accelerometer_process_noise_variance) {
            return Err("accelerometer_process_noise_variance must be finite and > 0".to_string());
        }
        if !valid_scale(self.visual_feature_noise_variance) {
            return Err("visual_feature_noise_variance must be finite and > 0".to_string());
        }
        if !valid_scale(self.field_containment_sigma) {
            return Err("field_containment_sigma must be finite and > 0".to_string());
        }
        for (name, timeout) in [
            ("tracking_timeout", self.tracking_timeout),
            ("visual_tracking_timeout", self.visual_tracking_timeout),
        ] {
            if timeout.is_zero() || timeout.as_nanos() > i64::MAX as u128 {
                return Err(format!(
                    "{name} must be positive and representable in signed nanoseconds"
                ));
            }
        }
        Ok(())
    }
}

fn valid_scale(value: f64) -> bool {
    value.is_finite() && value >= f64::MIN_POSITIVE.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_json_defaults_and_grid_validation() {
        let configured: Localization3dParameters = json5::from_str(include_str!(
            "../../../../etc/parameters/base/localization3d.json5"
        ))
        .unwrap();
        configured.validate().unwrap();
        let mut legacy = serde_json::to_value(&configured).unwrap();
        for key in ["timing", "model", "solver", "visual", "inputs"] {
            legacy.as_object_mut().unwrap().remove(key);
        }
        legacy["imu_preintegration"]
            .as_object_mut()
            .unwrap()
            .remove("reference_duration");
        let legacy: Localization3dParameters = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy, configured);
        for (trajectory, bias, interval, window) in [
            (0, 5000, 100, 2000),
            (200, 5100, 100, 2000),
            (200, 5000, 30, 2000),
            (200, 5000, 100, 200),
        ] {
            let mut invalid = configured.clone();
            invalid.timing.trajectory_spacing = Duration::from_millis(trajectory);
            invalid.timing.bias_spacing = Duration::from_millis(bias);
            invalid.timing.preintegration_interval = Duration::from_millis(interval);
            invalid.timing.optimization_window = Duration::from_millis(window);
            assert!(invalid.validate().is_err());
        }
        let mut invalid = configured.clone();
        invalid.visual.min_associations = 2;
        assert!(invalid.validate().is_err());
        invalid = configured.clone();
        invalid.inputs.imu_queue = 0;
        assert!(invalid.validate().is_err());
        let mut long_timeout = configured;
        long_timeout.tracking_timeout = Duration::from_secs(48 * 3600);
        assert!(long_timeout.validate().is_ok());
        long_timeout.tracking_timeout = Duration::MAX;
        assert!(long_timeout.validate().is_err());
    }

    #[test]
    fn validation_rejects_unusable_whitening_and_timer_ranges() {
        let parameters = Localization3dParameters {
            timing: Default::default(),
            model: Default::default(),
            solver: Default::default(),
            visual: Default::default(),
            inputs: Default::default(),
            imu_preintegration: Default::default(),
            imu_bias: Default::default(),
            kinematic_odometry_noise: Some(Default::default()),
            accelerometer: None,
            initial_height_sigma: 1.0,
            initial_velocity_sigma: 5.0,
            recovery_height_gate: 9.0,
            max_tilt_error: 20.0_f64.to_radians(),
            accelerometer_process_noise_variance: 10.0,
            visual_feature_noise_variance: 10000.0,
            field_containment_sigma: 1.0,
            max_heading_error: 20.0_f64.to_radians(),
            max_heading_reference_drift_per_second: 0.5_f64.to_radians(),
            tracking_timeout: Duration::from_secs(2),
            visual_tracking_timeout: Duration::from_secs(2),
        };
        assert!(parameters.validate().is_ok());
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut invalid = parameters.clone();
            invalid.imu_bias.accelerometer_initial_sigma = value;
            assert!(invalid.validate().is_err());
            invalid = parameters.clone();
            invalid.imu_bias.gyroscope_random_walk = value;
            assert!(invalid.validate().is_err());
        }
        let mut calibrated = parameters.clone();
        calibrated.accelerometer = Some(AccelerometerParameters::default());
        assert!(calibrated.validate().is_ok());
        calibrated.accelerometer.as_mut().unwrap().scale.x = 0.0;
        assert!(calibrated.validate().is_err());
        calibrated.accelerometer = Some(AccelerometerParameters {
            noise_density: f64::NAN,
            ..Default::default()
        });
        assert!(calibrated.validate().is_err());
        for error in [0.0, f64::NAN, std::f64::consts::FRAC_PI_2] {
            let mut invalid = parameters.clone();
            invalid.max_heading_error = error;
            assert!(invalid.validate().is_err());
        }
        for drift in [-1.0, f64::INFINITY, f64::NAN] {
            let mut invalid = parameters.clone();
            invalid.max_heading_reference_drift_per_second = drift;
            assert!(invalid.validate().is_err());
        }
        let mut invalid = parameters.clone();
        invalid.accelerometer_process_noise_variance = 1.0e-250;
        assert!(invalid.validate().is_err());
        invalid = parameters.clone();
        invalid.visual_feature_noise_variance = f64::INFINITY;
        assert!(invalid.validate().is_err());
        invalid = parameters.clone();
        invalid.field_containment_sigma = 1.0e-250;
        assert!(invalid.validate().is_err());
        for timeout in [Duration::ZERO, Duration::MAX] {
            invalid = parameters.clone();
            invalid.tracking_timeout = timeout;
            assert!(invalid.validate().is_err());
            invalid = parameters.clone();
            invalid.visual_tracking_timeout = timeout;
            assert!(invalid.validate().is_err());
        }
    }
}

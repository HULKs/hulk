use crate::report::GlobalLock as GlobalVisualLock;
use booster::ImuState;
use color_eyre::{Result, eyre::eyre};
use coordinate_systems::{Camera, Field, Ground, Head, Local, Robot};
use field_mark_association::{
    AssociationInput, DetectedVisualFeatures, FieldMarkAssociationParameters,
    associate_visual_features, raw_detections,
};
use linear_algebra::{Framed, IntoTransform, Isometry3 as FramedIsometry3, Orientation3};
use localization_3d::{Localization, SolveDiagnostics};
use nalgebra::{Isometry3, Point2, Translation3, UnitQuaternion, Vector3};
use projection::camera_matrix::CameraMatrix;
use ros_z::time::Time;
use types::camera_geometry::CameraGeometry;
use types::{
    field_dimensions::FieldDimensions,
    localization::{LocalizationEstimate, LocalizationState},
    time_wrapper::TimeWrapper,
    visual_localization::{AssociationGeometry, FieldMarkAssociation, VisualLocalizationFrame},
    visual_odometry::VisualOdometryDelta,
};

use crate::{
    config::{
        AssociationMode, FIELD_MARK_INTERVAL, SimulationConfig, TICK_INTERVAL, VisualOdometryMode,
        production_association_parameters, production_localization_parameters,
    },
    production_vo::{ProductionVisualOdometry, ProductionVoDiagnostics},
    sensors::{LandmarkObservations, SyntheticSensors},
    trajectory::{Scenario, fixed_robot_to_camera, robot_to_field_from_camera_to_field},
};

#[derive(Clone, Debug)]
/// Counts from one generated field-mark frame.
pub struct LandmarkFrameCounts {
    /// Ground-truth landmarks geometrically visible before synthetic sensor effects.
    pub ideal_visible: usize,
    /// In-bounds detections emitted after dropout and pixel noise.
    pub emitted_detections: usize,
    /// Proposed correspondences; localization still validates freshness and backend acceptance.
    pub associated: usize,
    /// Pixel detections emitted after noise and dropout, including their semantic class.
    pub detections: Vec<LandmarkDetection>,
    /// Pixel-to-field correspondences actually passed to localization.
    pub associations: Vec<FieldMarkAssociation>,
}

#[derive(Clone, Copy, Debug)]
pub struct LandmarkDetection {
    pub class: LandmarkClass,
    pub pixel: [f32; 2],
    pub confidence: f32,
}

pub use field_mark_association::VisualFeatureClass as LandmarkClass;

#[derive(Clone, Debug)]
/// One fixed-tick snapshot retained for inspection and regression tests.
pub struct SimulationHistorySample {
    /// Logical sample time.
    pub time: Time,
    /// Timestamp of the latest accepted solve; held poses are never relabeled as current.
    pub estimate_time: Option<Time>,
    /// Synthetic IMU input ingested directly into the estimator.
    pub imu: ImuState,
    /// Ground-truth robot-to-field pose.
    pub truth_robot_to_field: FramedIsometry3<Robot, Field>,
    /// Latest accepted field pose, in solver precision.
    pub raw_backend_robot_to_field: Option<FramedIsometry3<Robot, Field, f64>>,
    /// Display-precision copy of the accepted field pose; held between solves.
    pub live_robot_to_field: Option<FramedIsometry3<Robot, Field>>,
    /// Global visual lock state at this tick.
    pub global_visual_lock: GlobalVisualLock,
    /// Authoritative localization lifecycle state.
    pub state: LocalizationState,
    /// Diagnostics from the latest backend solve.
    pub diagnostics: Option<SolveDiagnostics>,
    /// Field-mark counts when this tick emitted a field-mark frame.
    pub landmark_frame: Option<LandmarkFrameCounts>,
    /// Accumulated noisy camera-to-visual-odometer transform.
    pub noisy_cumulative_camera_to_visual_odometer: Isometry3<f32>,
    /// Noisy VO transition ingested at this tick.
    pub visual_odometry_delta: Option<VisualOdometryDelta>,
    /// Diagnostics from production stereo VO, when enabled.
    pub production_vo_diagnostics: Option<ProductionVoDiagnostics>,
}

/// Deterministic synchronous runner that retains every fixed-tick result.
pub struct LocalizationSimulation {
    scenario: Scenario,
    config: SimulationConfig,
    history: Vec<SimulationHistorySample>,
    localization: Localization,
    latest_estimate: Option<LocalizationEstimate>,
    sensors: SyntheticSensors,
    field_dimensions: FieldDimensions,
    association_parameters: FieldMarkAssociationParameters,
    step_index: usize,
    previous_camera_geometry: Option<CameraGeometry>,
    latest_diagnostics: Option<SolveDiagnostics>,
    production_visual_odometry: Option<ProductionVisualOdometry>,
}

impl LocalizationSimulation {
    /// Creates an isolated deterministic localization simulation.
    pub fn new(scenario: Scenario, config: SimulationConfig) -> Result<Self> {
        config.validate().map_err(|message| eyre!(message))?;
        let field_dimensions = FieldDimensions::SPL_2025;
        let initial_camera_to_field = scenario.sample_camera_to_field(0.0);
        let truth_robot_to_field = robot_to_field_from_camera_to_field(&initial_camera_to_field);
        let initial_camera_matrix = camera_matrix(&truth_robot_to_field);
        let (roll, pitch, _) = truth_robot_to_field.inner.rotation.euler_angles();
        let initial_robot_to_local: FramedIsometry3<Robot, Local> = Isometry3::from_parts(
            Translation3::new(0.0, 0.0, truth_robot_to_field.translation().z()),
            UnitQuaternion::from_euler_angles(roll, pitch, 0.0),
        )
        .framed_transform();
        let localization_parameters =
            production_localization_parameters().map_err(|message| eyre!(message))?;
        let association_parameters =
            production_association_parameters().map_err(|message| eyre!(message))?;
        let production_visual_odometry = matches!(
            config.visual_odometry_mode,
            VisualOdometryMode::ProductionStereo
        )
        .then(ProductionVisualOdometry::new)
        .transpose()?;
        let localization = Localization::new(
            Time::from_nanos(0),
            0,
            &localization_parameters,
            &field_dimensions,
            &CameraGeometry::from(&initial_camera_matrix),
            initial_robot_to_local,
        )?;
        Ok(Self {
            scenario,
            sensors: SyntheticSensors::new(&config, &field_dimensions),
            config,
            history: Vec::new(),
            localization,
            field_dimensions,
            association_parameters,
            step_index: 0,
            previous_camera_geometry: None,
            latest_diagnostics: None,
            latest_estimate: None,
            production_visual_odometry,
        })
    }

    /// Recreates all estimator and synthetic-sensor state from the original inputs.
    pub fn reset(&mut self) -> Result<()> {
        *self = Self::new(self.scenario.clone(), self.config.clone())?;
        Ok(())
    }

    /// Returns the active validated trajectory.
    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    /// Returns the immutable synthetic sensor configuration.
    pub fn config(&self) -> &SimulationConfig {
        &self.config
    }

    /// Returns all generated history samples.
    pub fn history(&self) -> &[SimulationHistorySample] {
        &self.history
    }

    fn association_geometry(&self) -> Option<TimeWrapper<AssociationGeometry>> {
        let estimate = self.latest_estimate.as_ref()?;
        Some(TimeWrapper {
            time: estimate.time,
            inner: AssociationGeometry::from_estimate(estimate, &self.localization.status())?,
        })
    }

    fn state(&self) -> LocalizationState {
        self.localization.status().state
    }

    /// Resets and deterministically runs the complete scenario.
    pub fn replay(&mut self) -> Result<&[SimulationHistorySample]> {
        self.reset()?;
        self.run_to_end()
    }

    /// Runs the remaining bounded scenario synchronously.
    pub fn run_to_end(&mut self) -> Result<&[SimulationHistorySample]> {
        while self.step()? {}
        Ok(&self.history)
    }

    /// Processes one fixed simulation tick, returning `false` after the final tick.
    pub fn step(&mut self) -> Result<bool> {
        self.step_with_observation_filter(|_, _| {})
    }

    // Test scenarios can restrict sensor observations without bypassing production association.
    fn step_with_observation_filter(
        &mut self,
        filter: impl FnOnce(Time, &mut LandmarkObservations),
    ) -> Result<bool> {
        if self.step_index > self.scenario.tick_count() {
            return Ok(false);
        }
        let elapsed = self.step_index as f32 * TICK_INTERVAL.as_secs_f32();
        let time = Time::from_nanos(
            i64::try_from(self.step_index).unwrap_or(i64::MAX) * TICK_INTERVAL.as_nanos() as i64,
        );
        let camera_to_field = self.scenario.sample_camera_to_field(elapsed);
        let robot_to_field = robot_to_field_from_camera_to_field(&camera_to_field);
        let current_camera_matrix = camera_matrix(&robot_to_field);
        let imu = imu_state(&self.scenario, elapsed, &robot_to_field, &camera_to_field);
        self.localization.ingest_imu(time, imu)?;

        let visual_odometry = match self.config.visual_odometry_mode {
            VisualOdometryMode::SyntheticDelta => {
                self.sensors.measure_visual_odometry(time, &camera_to_field)
            }
            VisualOdometryMode::ProductionStereo => {
                let delay =
                    std::time::Duration::from_secs_f32(self.config.right_camera_delay_ms / 1_000.0);
                let right_time = Time::from_nanos(
                    time.as_nanos()
                        .saturating_add(i64::try_from(delay.as_nanos()).unwrap_or(i64::MAX)),
                );
                let delayed_left_camera_to_field = self
                    .scenario
                    .sample_camera_to_field(elapsed + delay.as_secs_f32());
                let right_camera_to_field = delayed_left_camera_to_field
                    * FramedIsometry3::<Camera, Camera>::from_translation(
                        crate::stereo_render::BASELINE,
                        0.0,
                        0.0,
                    );
                let reported_right_time = if self.config.assume_synchronized_stereo_timestamps {
                    time
                } else {
                    right_time
                };
                self.production_visual_odometry
                    .as_mut()
                    .expect("production VO is initialized for production stereo mode")
                    .measure(
                        time,
                        reported_right_time,
                        &camera_to_field.inner,
                        &right_camera_to_field.inner,
                    )
            }
        };
        let robot_to_camera = framed_robot_to_camera();
        self.localization.advance_time(time);
        self.localization.ingest_visual_odometry(
            visual_odometry.odometer.clone(),
            self.previous_camera_geometry.as_ref(),
            Some(&CameraGeometry::from(&current_camera_matrix)),
        )?;
        let mut landmark_frame = None;
        if self
            .step_index
            .is_multiple_of(interval_steps(FIELD_MARK_INTERVAL))
        {
            let geometry = self.association_geometry();
            let status = self.localization.status();
            let mut observations = self
                .sensors
                .observe_landmarks(&camera_to_field, &current_camera_matrix);
            filter(time, &mut observations);
            let rpy = imu.roll_pitch_yaw.inner;
            let association = match self.config.association_mode {
                AssociationMode::KnownCorrespondences => {
                    field_mark_association::AssociationResult {
                        associations: observations.true_associations,
                        source: if status.state == LocalizationState::Tracking {
                            types::visual_localization::VisualAssociationSource::Tracking
                        } else {
                            types::visual_localization::VisualAssociationSource::Global
                        },
                        ..Default::default()
                    }
                }
                AssociationMode::ProductionAssociation => associate_visual_features(
                    AssociationInput {
                        status: &status,
                        attitude: Some(Orientation3::from_euler_angles(rpy.x, rpy.y, rpy.z)),
                        tracking: geometry.as_ref().map(|g| &g.inner),
                        visual_features: &observations.detections,
                        robot_to_camera,
                        camera_intrinsic: current_camera_matrix.intrinsics,
                        field_dimensions: &self.field_dimensions,
                        time,
                    },
                    &self.association_parameters,
                ),
            };
            let associations = association.associations;
            landmark_frame = Some(LandmarkFrameCounts {
                ideal_visible: observations.ideal_visible_count,
                emitted_detections: observations.detections.supported_feature_count(),
                associated: associations.len(),
                detections: flatten_detections(&observations.detections),
                associations: associations.clone(),
            });
            if !associations.is_empty() {
                let frame = VisualLocalizationFrame {
                    epoch: status.epoch,
                    generation: status.generation,
                    source: association.source,
                    robot_to_camera,
                    camera_intrinsic: current_camera_matrix.intrinsics,
                    associations,
                };
                self.localization
                    .ingest_visual_localization_frame(TimeWrapper { time, inner: frame })?;
            }
        }

        let output = self.localization.solve(time);
        self.latest_diagnostics = Some(output.diagnostics);
        if let Some(estimate) = output.estimate {
            self.latest_estimate = Some(estimate);
        }
        let live_robot_to_field = self
            .latest_estimate
            .and_then(|estimate| estimate.robot_to_field)
            .map(|field| field.pose)
            .map(|pose| pose.inner.cast().framed_transform());

        let sample = SimulationHistorySample {
            time,
            estimate_time: self.latest_estimate.map(|estimate| estimate.time),
            imu,
            truth_robot_to_field: robot_to_field,
            raw_backend_robot_to_field: self
                .latest_estimate
                .and_then(|estimate| estimate.robot_to_field)
                .map(|field| field.pose),
            live_robot_to_field,
            global_visual_lock: if self.localization.status().state == LocalizationState::Tracking {
                GlobalVisualLock::Locked
            } else {
                GlobalVisualLock::Unlocked
            },
            state: self.state(),
            diagnostics: self.latest_diagnostics.clone(),
            landmark_frame,
            noisy_cumulative_camera_to_visual_odometer: visual_odometry
                .odometer
                .current_left_camera_to_visual_odometer,
            visual_odometry_delta: visual_odometry.delta,
            production_vo_diagnostics: visual_odometry.production_diagnostics,
        };
        self.previous_camera_geometry = Some(CameraGeometry::from(&current_camera_matrix));
        self.step_index += 1;
        self.history.push(sample);
        Ok(true)
    }
}

fn flatten_detections(detections: &DetectedVisualFeatures) -> Vec<LandmarkDetection> {
    let mut flattened = Vec::with_capacity(detections.supported_feature_count());
    flattened.extend(
        raw_detections(detections).map(|(class, feature)| LandmarkDetection {
            class,
            pixel: [feature.pixel.inner.x, feature.pixel.inner.y],
            confidence: feature.confidence,
        }),
    );
    flattened
}

fn interval_steps(interval: std::time::Duration) -> usize {
    (interval.as_nanos() / TICK_INTERVAL.as_nanos()) as usize
}

fn framed_robot_to_camera() -> FramedIsometry3<Robot, Camera> {
    fixed_robot_to_camera()
}

pub(crate) fn camera_matrix(robot_to_field: &FramedIsometry3<Robot, Field>) -> CameraMatrix {
    let (_, _, yaw) = robot_to_field.inner.rotation.euler_angles();
    let ground_to_field: FramedIsometry3<Ground, Field> = Isometry3::from_parts(
        Translation3::new(
            robot_to_field.translation().x(),
            robot_to_field.translation().y(),
            0.0,
        ),
        UnitQuaternion::from_euler_angles(0.0, 0.0, yaw),
    )
    .framed_transform();
    let ground_to_robot: FramedIsometry3<Ground, Robot> =
        robot_to_field.inverse() * ground_to_field;
    let robot_to_head: FramedIsometry3<Robot, Head> = Isometry3::identity().framed_transform();
    let head_to_camera: FramedIsometry3<Head, Camera> =
        fixed_robot_to_camera() * robot_to_head.inverse();
    CameraMatrix::from_normalized_focal_and_center(
        nalgebra::vector![0.625, 0.625],
        Point2::new(0.5, 0.5),
        Framed::wrap(nalgebra::vector![640.0, 480.0]),
        ground_to_robot,
        robot_to_head,
        head_to_camera,
    )
}

fn imu_state(
    scenario: &Scenario,
    elapsed: f32,
    robot_to_field: &FramedIsometry3<Robot, Field>,
    camera_to_field: &FramedIsometry3<Camera, Field>,
) -> ImuState {
    let dt = TICK_INTERVAL.as_secs_f32();
    let (roll, pitch, yaw) = robot_to_field.inner.rotation.euler_angles();
    let (rotation_start, rotation_end, rotation_dt) = if elapsed + dt <= scenario.duration_seconds()
    {
        let next_camera = scenario.sample_camera_to_field(elapsed + dt);
        (
            robot_to_field.inner.rotation,
            robot_to_field_from_camera_to_field(&next_camera)
                .inner
                .rotation,
            dt,
        )
    } else {
        let previous_camera = scenario.sample_camera_to_field((elapsed - dt).max(0.0));
        (
            robot_to_field_from_camera_to_field(&previous_camera)
                .inner
                .rotation,
            robot_to_field.inner.rotation,
            dt,
        )
    };
    let angular_velocity = (rotation_start.inverse() * rotation_end).scaled_axis() / rotation_dt;

    let global_acceleration = trajectory_acceleration(scenario, elapsed, camera_to_field, dt);
    let gravity_in_robot = robot_to_field
        .inner
        .rotation
        .inverse_transform_vector(&(global_acceleration + Vector3::new(0.0, 0.0, 9.81)));
    ImuState {
        roll_pitch_yaw: Framed::wrap(Vector3::new(roll, pitch, yaw)),
        angular_velocity: Framed::wrap(angular_velocity),
        linear_acceleration: Framed::wrap(gravity_in_robot),
    }
}

fn trajectory_acceleration(
    scenario: &Scenario,
    elapsed: f32,
    camera_to_field: &FramedIsometry3<Camera, Field>,
    dt: f32,
) -> Vector3<f32> {
    let position = camera_to_field.translation().coords().inner;
    if elapsed + 2.0 * dt <= scenario.duration_seconds() {
        let next = scenario
            .sample_camera_to_field(elapsed + dt)
            .inner
            .translation
            .vector;
        let after_next = scenario
            .sample_camera_to_field(elapsed + 2.0 * dt)
            .inner
            .translation
            .vector;
        return (after_next - 2.0 * next + position) / dt.powi(2);
    }
    if elapsed >= 2.0 * dt {
        let previous = scenario
            .sample_camera_to_field(elapsed - dt)
            .inner
            .translation
            .vector;
        let before_previous = scenario
            .sample_camera_to_field(elapsed - 2.0 * dt)
            .inner
            .translation
            .vector;
        return (position - 2.0 * previous + before_previous) / dt.powi(2);
    }
    Vector3::zeros()
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn sparse_scenario(duration: f32, opponent_half: bool) -> Scenario {
        use crate::trajectory::PoseKeyframe;

        let own = Scenario::stationary().sample_camera_to_field(0.0);
        let mut poses = vec![(0.0, own)];
        let end = if opponent_half {
            let mirror = Isometry3::from_parts(
                Translation3::identity(),
                UnitQuaternion::from_euler_angles(0.0, 0.0, std::f32::consts::PI),
            )
            .framed_transform::<Field, Field>()
                * own;
            // Smooth start/stop: this fixture tests field-branch recovery, not
            // the response to infinite acceleration in a piecewise-linear path.
            poses.extend((0..=150).map(|step| {
                let u = step as f32 / 150.0;
                let blend = u.powi(3) * (10.0 - 15.0 * u + 6.0 * u * u);
                (
                    1.0 + 3.0 * u,
                    own.inner
                        .lerp_slerp(&mirror.inner, blend)
                        .framed_transform(),
                )
            }));
            mirror
        } else {
            own
        };
        poses.push((duration, end));
        Scenario::new(
            "sparse_lifecycle",
            duration,
            poses
                .into_iter()
                .map(|(time, pose)| PoseKeyframe::from_camera_to_field(time, pose))
                .collect(),
        )
        .unwrap()
    }

    fn run_sparse(
        scenario: Scenario,
        config: SimulationConfig,
        count_at: impl Fn(i64) -> usize,
    ) -> LocalizationSimulation {
        let mut simulation = LocalizationSimulation::new(scenario, config).unwrap();
        assert_eq!(simulation.state(), LocalizationState::Startup);
        while simulation
            .step_with_observation_filter(|time, observations| {
                // Restrict the emitted sensor frame, not the associator's output or its pose prior.
                let retained = count_at(time.as_nanos());
                observations.true_associations.truncate(retained);
                for features in [
                    &mut observations.detections.goalposts,
                    &mut observations.detections.l_spots,
                    &mut observations.detections.t_spots,
                    &mut observations.detections.x_spots,
                    &mut observations.detections.penalty_spots,
                ] {
                    features.retain(|feature| {
                        observations
                            .true_associations
                            .iter()
                            .any(|association| association.detection == feature.pixel)
                    });
                }
                assert!(observations.detections.supported_feature_count() <= retained);
            })
            .unwrap()
        {}
        simulation
    }

    fn exact_sensor_config() -> SimulationConfig {
        SimulationConfig {
            landmark_pixel_sigma: 0.0,
            vo_translation_sigma_m: 0.0,
            vo_rotation_sigma_rad: 0.0,
            ..Default::default()
        }
    }

    fn state_transitions(simulation: &LocalizationSimulation) -> Vec<(i64, &'static str)> {
        let mut transitions = Vec::new();
        let mut maximum_translation_error = 0.0_f32;
        let mut maximum_rotation_error = 0.0_f32;
        for sample in simulation.history() {
            let state = match sample.state {
                LocalizationState::Startup => "Startup",
                LocalizationState::Tracking => "Tracking",
                LocalizationState::LostTrack => "LostTrack",
            };
            if transitions
                .last()
                .is_none_or(|&(_, previous)| previous != state)
            {
                transitions.push((sample.time.as_nanos(), state));
                if sample.state == LocalizationState::Tracking {
                    assert_eq!(sample.estimate_time, Some(sample.time));
                    assert!(sample.diagnostics.as_ref().unwrap().measurement_count >= 3);
                }
            }
            if let Some(pose) = sample.live_robot_to_field {
                let truth_time = sample.estimate_time.expect("pose has a solve timestamp");
                let truth = robot_to_field_from_camera_to_field(
                    &simulation
                        .scenario
                        .sample_camera_to_field(truth_time.as_nanos() as f32 * 1.0e-9),
                );
                let error = (pose.translation() - truth.translation()).norm();
                maximum_translation_error = maximum_translation_error.max(error);
                maximum_rotation_error =
                    maximum_rotation_error.max(pose.inner.rotation.angle_to(&truth.inner.rotation));
                // Accuracy is checked by the stationary and smooth-trajectory tests.
                // These discontinuous-motion fixtures test lifecycle and field branch.
                assert!(error.is_finite());
                assert!(pose.inner.rotation.angle_to(&truth.inner.rotation) < 5.0_f32.to_radians());
            }
            if let Some(frame) = &sample.landmark_frame
                && frame.emitted_detections < 3
                && simulation.config.association_mode == AssociationMode::ProductionAssociation
            {
                assert_eq!(frame.associated, 0);
            }
        }
        eprintln!(
            "sparse transitions: {transitions:?}; max_error={maximum_translation_error:.4} m / {:.3} deg",
            maximum_rotation_error.to_degrees()
        );
        transitions
    }

    #[test]
    fn sparse_stationary_startup_tracking_loss_and_recovery() {
        let simulation = run_sparse(
            sparse_scenario(10.0, false),
            exact_sensor_config(),
            |time| match time / 1_000_000_000 {
                0 => 0,
                1 => 1,
                2 | 4 | 6..=8 => 2,
                _ => 3,
            },
        );
        let transitions = state_transitions(&simulation);
        assert_eq!(
            transitions,
            vec![
                (0, "Startup"),
                // Bootstrap preserves the already-ingested IMU/VO window.
                (3_000_000_000, "Tracking"),
                (7_900_000_000, "LostTrack"),
                (9_000_000_000, "Tracking"),
            ]
        );
        for sample in simulation.history() {
            if sample.state == LocalizationState::LostTrack {
                assert_eq!(sample.global_visual_lock, GlobalVisualLock::Unlocked);
                assert_eq!(sample.estimate_time, Some(sample.time));
            }
        }
    }

    #[test]
    fn sparse_recovery_keeps_opponent_half_and_heading() {
        for association_mode in [
            AssociationMode::ProductionAssociation,
            AssociationMode::KnownCorrespondences,
        ] {
            let simulation = run_sparse(
                sparse_scenario(10.0, true),
                SimulationConfig {
                    association_mode,
                    ..exact_sensor_config()
                },
                |time| {
                    if (5_000_000_000..8_000_000_000).contains(&time) {
                        2
                    } else {
                        3
                    }
                },
            );
            let transitions = state_transitions(&simulation);
            assert_eq!(
                transitions,
                vec![
                    (0, "Startup"),
                    (100_000_000, "Tracking"),
                    (6_900_000_000, "LostTrack"),
                    (8_000_000_000, "Tracking"),
                ]
            );
            for sample in simulation
                .history()
                .iter()
                .filter(|sample| sample.time.as_nanos() >= 4_000_000_000)
            {
                assert_ne!(
                    sample.state,
                    LocalizationState::Startup,
                    "recovery must not restart globally"
                );
                assert!(sample.raw_backend_robot_to_field.unwrap().translation().x() > 0.0);
            }
        }
    }

    #[test]
    fn sparse_expired_tracking_prior_recovers_with_heading() {
        let simulation = run_sparse(
            sparse_scenario(10.0, false),
            exact_sensor_config(),
            |time| {
                if (1_000_000_000..9_000_000_000).contains(&time) {
                    2
                } else {
                    3
                }
            },
        );
        assert_eq!(
            state_transitions(&simulation),
            vec![
                (0, "Startup"),
                (100_000_000, "Tracking"),
                (2_900_000_000, "LostTrack"),
                (9_000_000_000, "Tracking"),
            ]
        );
        for sample in simulation
            .history()
            .iter()
            .filter(|sample| sample.time.as_nanos() >= 9_000_000_000)
        {
            if let Some(frame) = &sample.landmark_frame {
                assert_eq!(frame.emitted_detections, 3);
                assert_eq!(frame.associated, 3);
            }
        }
    }

    #[test]
    fn sparse_timeout_boundary_requires_a_post_loss_frame() {
        let simulation = run_sparse(sparse_scenario(4.0, false), exact_sensor_config(), |time| {
            if (1_000_000_000..2_900_000_000).contains(&time) {
                2
            } else {
                3
            }
        });
        assert_eq!(
            state_transitions(&simulation),
            vec![
                (0, "Startup"),
                (100_000_000, "Tracking"),
                (2_900_000_000, "LostTrack"),
                (3_000_000_000, "Tracking"),
            ]
        );
        let boundary = simulation
            .history()
            .iter()
            .find(|sample| sample.time.as_nanos() == 2_900_000_000)
            .unwrap();
        assert_eq!(boundary.landmark_frame.as_ref().unwrap().associated, 0);
        assert!(
            boundary.state == LocalizationState::LostTrack,
            "shared dispatch requires exposure after the loss boundary"
        );
    }

    #[test]
    fn sparse_noisy_three_feature_recovery_without_motion() {
        for (index, seed) in [0, 1, 2, 3, 0x5eed].into_iter().enumerate() {
            let sparse_count = index % 3;
            let simulation = run_sparse(
                sparse_scenario(6.0, false),
                SimulationConfig {
                    seed,
                    ..Default::default()
                },
                |time| {
                    if (1_000_000_000..4_000_000_000).contains(&time) {
                        sparse_count
                    } else {
                        3
                    }
                },
            );
            eprintln!("seed={seed}, gap_features={sparse_count}");
            let transitions = state_transitions(&simulation);
            assert_eq!(
                transitions
                    .iter()
                    .map(|&(_, state)| state)
                    .collect::<Vec<_>>(),
                vec!["Startup", "Tracking", "LostTrack", "Tracking"]
            );
            assert!((2_800_000_000..=3_000_000_000).contains(&transitions[2].0));
            assert!(
                (4_000_000_000..=4_600_000_000).contains(&transitions[3].0),
                "recovery must occur promptly after restoring three detections"
            );
        }
    }

    #[test]
    fn sparse_recovery_with_four_or_five_features() {
        for count in [4, 5] {
            let simulation =
                run_sparse(sparse_scenario(6.0, false), exact_sensor_config(), |time| {
                    if (1_000_000_000..4_000_000_000).contains(&time) {
                        2
                    } else {
                        count
                    }
                });
            assert_eq!(
                simulation.history()[0]
                    .landmark_frame
                    .as_ref()
                    .unwrap()
                    .emitted_detections,
                count
            );
            eprintln!("restored_features={count}");
            assert_eq!(
                state_transitions(&simulation),
                vec![
                    (0, "Startup"),
                    (100_000_000, "Tracking"),
                    (2_900_000_000, "LostTrack"),
                    (4_000_000_000, "Tracking"),
                ]
            );
        }
    }

    #[test]
    fn sparse_boundary_view_with_two_l_spots_and_a_penalty_spot() {
        let robot = Isometry3::from_parts(
            Translation3::new(-4.5, 0.0, 0.55),
            UnitQuaternion::from_euler_angles(0.0, 0.2, std::f32::consts::FRAC_PI_6),
        );
        let camera =
            FramedIsometry3::<Robot, Field>::wrap(robot) * fixed_robot_to_camera().inverse();
        let scenario = Scenario::new(
            "sparse_boundary",
            6.0,
            [0.0, 6.0]
                .map(|time| crate::trajectory::PoseKeyframe::from_camera_to_field(time, camera))
                .to_vec(),
        )
        .unwrap();
        let mut simulation =
            LocalizationSimulation::new(scenario, SimulationConfig::default()).unwrap();
        while simulation
            .step_with_observation_filter(|time, observations| {
                let features = &mut observations.detections;
                features.goalposts.clear();
                features.t_spots.clear();
                features.x_spots.clear();
                features
                    .l_spots
                    .sort_by(|a, b| b.pixel.y().total_cmp(&a.pixel.y()));
                features.l_spots.truncate(2);
                features
                    .penalty_spots
                    .sort_by(|a, b| b.pixel.y().total_cmp(&a.pixel.y()));
                features.penalty_spots.truncate(usize::from(
                    !(1_000_000_000..4_000_000_000).contains(&time.as_nanos()),
                ));
                observations.true_associations.retain(|association| {
                    features
                        .l_spots
                        .iter()
                        .chain(&features.penalty_spots)
                        .any(|feature| feature.pixel == association.detection)
                });
                assert_eq!(features.l_spots.len(), 2);
            })
            .unwrap()
        {}
        assert_eq!(
            state_transitions(&simulation),
            vec![
                (0, "Startup"),
                (100_000_000, "Tracking"),
                (2_900_000_000, "LostTrack"),
                (4_000_000_000, "Tracking"),
            ]
        );
    }

    #[test]
    fn distant_stationary_view_does_not_fabricate_a_startup_lock() {
        let robot = Isometry3::from_parts(
            Translation3::new(-3.5, 1.5, 0.55),
            UnitQuaternion::from_euler_angles(0.0, -0.1, 0.5),
        );
        let camera =
            FramedIsometry3::<Robot, Field>::wrap(robot) * fixed_robot_to_camera().inverse();
        let scenario = Scenario::new(
            "distant_stationary",
            3.0,
            [0.0, 3.0]
                .map(|time| crate::trajectory::PoseKeyframe::from_camera_to_field(time, camera))
                .to_vec(),
        )
        .unwrap();
        let mut simulation = LocalizationSimulation::new(
            scenario,
            SimulationConfig {
                landmark_pixel_sigma: 0.0,
                vo_translation_sigma_m: 0.0,
                vo_rotation_sigma_rad: 0.0,
                ..Default::default()
            },
        )
        .unwrap();
        simulation.run_to_end().unwrap();
        assert!(
            simulation
                .history
                .iter()
                .all(|sample| sample.state == LocalizationState::Startup
                    && sample.live_robot_to_field.is_none())
        );
        for counts in simulation
            .history
            .iter()
            .filter_map(|sample| sample.landmark_frame.as_ref())
        {
            assert_eq!(counts.emitted_detections, 7);
            assert_eq!(counts.associated, 0);
        }
    }

    #[test]
    fn stationary_known_correspondences_lock_with_finite_output_near_truth() {
        check_stationary_lock(AssociationMode::KnownCorrespondences, 0.5_f32.to_radians());
    }

    #[test]
    fn production_association_acquires_global_lock() {
        check_stationary_lock(AssociationMode::ProductionAssociation, 5.0_f32.to_radians());
    }

    fn check_stationary_lock(association_mode: AssociationMode, rotation_tolerance: f32) {
        let mut simulation = LocalizationSimulation::new(
            Scenario::stationary(),
            SimulationConfig {
                association_mode,
                ..exact_sensor_config()
            },
        )
        .unwrap();
        simulation.run_to_end().unwrap();
        assert!(
            simulation.history.iter().any(|sample| sample
                .landmark_frame
                .as_ref()
                .is_some_and(|frame| frame.associated > 0)),
            "{association_mode:?} emitted no associations"
        );
        let last = simulation.history.last().unwrap();
        assert_eq!(last.global_visual_lock, GlobalVisualLock::Locked);
        let raw = last.raw_backend_robot_to_field.as_ref().unwrap();
        assert!(
            raw.inner
                .translation
                .vector
                .iter()
                .chain(raw.inner.rotation.coords.iter())
                .all(|value| value.is_finite())
        );
        let estimate = last
            .live_robot_to_field
            .expect("stationary localization has a live pose")
            .inner;
        let truth = last.truth_robot_to_field.inner;
        let translation_error = (estimate.translation.vector - truth.translation.vector).norm();
        assert!(
            translation_error < 0.1,
            "translation error was {translation_error}"
        );
        assert!(estimate.rotation.angle_to(&truth.rotation) < rotation_tolerance);
    }

    #[test]
    fn failed_production_association_preserves_counts_and_startup_geometry() {
        let mut simulation = LocalizationSimulation::new(
            Scenario::stationary(),
            SimulationConfig {
                association_mode: AssociationMode::ProductionAssociation,
                landmark_dropout_probability: 1.0,
                ..Default::default()
            },
        )
        .unwrap();

        simulation.step().unwrap();

        let counts = simulation.history[0].landmark_frame.as_ref().unwrap();
        assert!(counts.ideal_visible > 0);
        assert_eq!(counts.emitted_detections, 0);
        assert_eq!(counts.associated, 0);
        assert!(counts.associations.is_empty());
        assert!(simulation.association_geometry().is_none());
        assert_eq!(simulation.state(), LocalizationState::Startup);
    }

    #[test]
    fn production_startup_recovers_height_from_pixels_despite_bad_localizer_height() {
        let mut simulation = LocalizationSimulation::new(
            Scenario::stationary(),
            SimulationConfig {
                landmark_pixel_sigma: 0.0,
                vo_translation_sigma_m: 0.0,
                vo_rotation_sigma_rad: 0.0,
                ..Default::default()
            },
        )
        .unwrap();
        // The provisional localizer height must not determine startup projection scale.
        let truth =
            robot_to_field_from_camera_to_field(&simulation.scenario.sample_camera_to_field(0.0));
        let (roll, pitch, _) = truth.inner.rotation.euler_angles();
        let initial_pose = Isometry3::from_parts(
            Translation3::new(0.0, 0.0, 6.0),
            UnitQuaternion::from_euler_angles(roll, pitch, 0.0),
        )
        .framed_transform();
        simulation.localization = Localization::new(
            Time::from_nanos(0),
            0,
            &production_localization_parameters().unwrap(),
            &simulation.field_dimensions,
            &CameraGeometry::from(&camera_matrix(&truth)),
            initial_pose,
        )
        .unwrap();
        assert_eq!(
            simulation.config.association_mode,
            AssociationMode::ProductionAssociation
        );
        for _ in 0..=interval_steps(FIELD_MARK_INTERVAL) {
            simulation.step().unwrap();
        }
        let last = simulation.history.last().unwrap();
        let counts = last.landmark_frame.as_ref().unwrap();
        assert!(counts.emitted_detections >= 3);
        let fitted_height = simulation
            .association_geometry()
            .unwrap()
            .inner
            .estimate
            .pose
            .translation()
            .z();
        assert!((fitted_height - f64::from(truth.translation().z())).abs() < 0.01);
        assert!(counts.associated >= 3);
    }

    #[test]
    fn noisy_production_associations_match_sensor_correspondences() {
        let field = FieldDimensions::SPL_2025;
        let scenario = Scenario::stationary();
        let config = SimulationConfig {
            seed: 0,
            association_mode: AssociationMode::ProductionAssociation,
            landmark_pixel_sigma: 2.0,
            landmark_dropout_probability: 0.2,
            ..Default::default()
        };
        let camera_to_field = scenario.sample_camera_to_field(0.0);
        let robot_to_field = robot_to_field_from_camera_to_field(&camera_to_field);
        let camera_matrix = camera_matrix(&robot_to_field);
        let mut sensors = SyntheticSensors::new(&config, &field);
        let parameters = production_association_parameters().unwrap();
        let simulation = LocalizationSimulation::new(scenario, config).unwrap();
        let mut accepted = false;
        for _ in 0..30 {
            let observations = sensors.observe_landmarks(&camera_to_field, &camera_matrix);
            let result = associate_visual_features(
                AssociationInput {
                    visual_features: &observations.detections,
                    robot_to_camera: framed_robot_to_camera(),
                    status: &simulation.localization.status(),
                    tracking: None,
                    attitude: Some(Orientation3::wrap(robot_to_field.inner.rotation)),
                    camera_intrinsic: camera_matrix.intrinsics,
                    field_dimensions: &field,
                    time: Time::from_nanos(0),
                },
                &parameters,
            );
            if result.associations.is_empty() {
                continue;
            }
            accepted = true;
            assert!(
                [1.0, -1.0]
                    .into_iter()
                    .any(|sign| result.associations.iter().all(|association| {
                        observations.true_associations.iter().any(|expected| {
                            (expected.detection - association.detection).inner.norm() < 1.0e-4
                                && (expected.field_point.inner.coords * sign
                                    - association.field_point.inner.coords)
                                    .norm()
                                    < 1.0e-4
                        })
                    }))
            );
        }
        assert!(accepted, "no noisy frame produced a certified assignment");
    }

    #[test]
    fn complete_simulation_is_deterministic_for_the_same_seed() {
        let config = SimulationConfig {
            seed: 0x5eed,
            landmark_pixel_sigma: 1.0,
            vo_translation_sigma_m: 0.002,
            vo_rotation_sigma_rad: 0.001,
            ..Default::default()
        };
        let mut simulation = LocalizationSimulation::new(Scenario::six_dof_loop(), config)
            .expect("simulation initializes");
        simulation.run_to_end().expect("simulation runs");
        let first = simulation.history.clone();
        simulation.replay().expect("simulation replays");
        let second = &simulation.history;

        assert_eq!(first.len(), second.len());
        for (left, right) in first.iter().zip(second) {
            assert_eq!(left.time, right.time);
            assert_eq!(left.global_visual_lock, right.global_visual_lock);
            assert_eq!(
                left.landmark_frame.as_ref().map(|counts| (
                    counts.ideal_visible,
                    counts.emitted_detections,
                    counts.associated
                )),
                right.landmark_frame.as_ref().map(|counts| (
                    counts.ideal_visible,
                    counts.emitted_detections,
                    counts.associated
                ))
            );
            assert_relative_eq!(
                left.noisy_cumulative_camera_to_visual_odometer
                    .translation
                    .vector,
                right
                    .noisy_cumulative_camera_to_visual_odometer
                    .translation
                    .vector,
                epsilon = 1.0e-6
            );
            match (
                left.raw_backend_robot_to_field.as_ref(),
                right.raw_backend_robot_to_field.as_ref(),
            ) {
                (Some(left), Some(right)) => {
                    assert_relative_eq!(
                        left.inner.translation.vector,
                        right.inner.translation.vector,
                        epsilon = 1.0e-9
                    );
                    assert!(left.inner.rotation.angle_to(&right.inner.rotation) < 1.0e-9);
                }
                (None, None) => {}
                _ => panic!("backend output availability differs"),
            }
            match (left.live_robot_to_field, right.live_robot_to_field) {
                (Some(left), Some(right)) => {
                    assert_relative_eq!(
                        left.inner.translation.vector,
                        right.inner.translation.vector,
                        epsilon = 1.0e-6
                    );
                    assert!(left.inner.rotation.angle_to(&right.inner.rotation) < 1.0e-6);
                }
                (None, None) => {}
                _ => panic!("live output availability differs"),
            }
        }
    }

    #[test]
    fn production_figure_eight_is_accurate_at_estimate_timestamps() {
        check_figure_eight_accuracy(AssociationMode::ProductionAssociation);
    }

    #[test]
    fn known_correspondence_figure_eight_isolates_estimator_accuracy() {
        check_figure_eight_accuracy(AssociationMode::KnownCorrespondences);
    }

    fn check_figure_eight_accuracy(association_mode: AssociationMode) {
        let mut simulation = LocalizationSimulation::new(
            Scenario::field_figure_eight_twice(),
            SimulationConfig {
                association_mode,
                ..exact_sensor_config()
            },
        )
        .expect("simulation initializes");
        simulation.run_to_end().expect("simulation runs");

        let first = simulation
            .history
            .iter()
            .find(|sample| sample.live_robot_to_field.is_some())
            .unwrap_or_else(|| {
                panic!(
                    "bootstrap must eventually localize: {:?}",
                    simulation
                        .history
                        .iter()
                        .step_by(10)
                        .take(8)
                        .map(|s| &s.diagnostics)
                        .collect::<Vec<_>>()
                )
            });
        assert!(
            first.truth_robot_to_field.translation().x() < 0.0,
            "bootstrap must acquire on the own half: {:?}",
            first.time
        );
        let mut squared_errors = 0.0;
        let mut sample_count = 0;
        for current in &simulation.history[1..simulation.history.len() - 1] {
            let Some(estimate) = current.live_robot_to_field.map(|pose| pose.inner) else {
                continue;
            };
            let truth_time = current.estimate_time.expect("pose has a solve timestamp");
            let truth = robot_to_field_from_camera_to_field(
                &simulation
                    .scenario
                    .sample_camera_to_field(truth_time.as_nanos() as f32 * 1.0e-9),
            );
            let error = estimate.translation.vector - truth.inner.translation.vector;
            squared_errors += error.norm_squared();
            sample_count += 1;
        }
        assert!(
            sample_count > simulation.history.len() / 2,
            "{association_mode:?} localized only {sample_count}/{} interior samples",
            simulation.history.len() - 2
        );
        let translation_rms = (squared_errors / sample_count as f32).sqrt();
        eprintln!("{association_mode:?}: figure8 rms={translation_rms} samples={sample_count}");

        assert!(
            translation_rms < 0.1,
            "translation RMS was {translation_rms}"
        );
    }
}

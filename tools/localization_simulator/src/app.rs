use std::time::{Duration, Instant};

use eframe::{
    App, CreationContext, Frame,
    egui::{self, CentralPanel, Color32, ComboBox, Key, Panel, RichText, Slider, Ui, Widget},
};
use egui_bevy::BevyWidget;
use localization_simulator::{
    AssociationMode, LocalizationSimulation, PoseKeyframe, Scenario, SimulationConfig,
    SimulationHistorySample, VisualOdometryMode, VisualOdometryOutlier,
    config::TICK_INTERVAL,
    trajectory::{fixed_robot_to_camera, robot_to_field_from_camera_to_field},
};
use nalgebra::{Isometry3, UnitQuaternion, Vector3};

use crate::scene::{self, SceneState};

const MAX_STEPS_PER_FRAME: usize = 100;
const MAX_WALL_DELTA: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Eq, PartialEq)]
enum ScenarioChoice {
    Stationary,
    SixDofLoop,
    FieldFigureEightTwice,
    PoseTeleport,
    VoFault,
    Custom,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SceneSource {
    Simulation,
    Recorder,
}

impl ScenarioChoice {
    fn label(self) -> &'static str {
        match self {
            Self::Stationary => "stationary",
            Self::SixDofLoop => "six_dof_loop",
            Self::FieldFigureEightTwice => "field_figure_eight_twice",
            Self::PoseTeleport => "pose_teleport",
            Self::VoFault => "vo_fault",
            Self::Custom => "Custom",
        }
    }
}

struct FlightRecorder {
    pose: linear_algebra::Isometry3<coordinate_systems::Camera, coordinate_systems::Field>,
    elapsed: Duration,
    accumulator: Duration,
    keyframes: Vec<PoseKeyframe>,
}

pub(crate) struct LocalizationSimulatorApp {
    widget: BevyWidget,
    simulation: Option<LocalizationSimulation>,
    config: SimulationConfig,
    scenario_choice: ScenarioChoice,
    applied_scenario_choice: ScenarioChoice,
    custom_scenario: Option<Scenario>,
    playing: bool,
    fast_forward: bool,
    playback_speed: f32,
    playback_accumulator: Duration,
    last_frame: Instant,
    inspect_index: usize,
    scenario_path: String,
    status: Option<String>,
    recorder: Option<FlightRecorder>,
    configuration_dirty: bool,
    simulation_failed: bool,
    scene_source: Option<SceneSource>,
    scene_history_len: usize,
}

impl LocalizationSimulatorApp {
    pub(crate) fn new(creation_context: &CreationContext) -> Self {
        creation_context.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut widget = BevyWidget::new(
            creation_context
                .wgpu_render_state
                .clone()
                .expect("no wgpu render state found"),
        );
        scene::configure(&mut widget.bevy_app);
        widget.bevy_app.finish();
        widget.bevy_app.cleanup();

        let config = SimulationConfig::default();
        let (simulation, status) =
            match LocalizationSimulation::new(Scenario::six_dof_loop(), config.clone()) {
                Ok(mut simulation) => match simulation.step() {
                    Ok(true) => (Some(simulation), None),
                    Ok(false) => (
                        Some(simulation),
                        Some("Scenario contains no samples".to_string()),
                    ),
                    Err(error) => (
                        None,
                        Some(format!("Initial simulation step failed: {error:#}")),
                    ),
                },
                Err(error) => (None, Some(format!("Initialization failed: {error:#}"))),
            };
        let inspect_index = simulation
            .as_ref()
            .map_or(0, |simulation| simulation.history().len().saturating_sub(1));
        Self {
            widget,
            simulation,
            config,
            scenario_choice: ScenarioChoice::SixDofLoop,
            applied_scenario_choice: ScenarioChoice::SixDofLoop,
            custom_scenario: None,
            playing: false,
            fast_forward: false,
            playback_speed: 1.0,
            playback_accumulator: Duration::ZERO,
            last_frame: Instant::now(),
            inspect_index,
            scenario_path: "localization_scenario.json5".to_string(),
            status,
            recorder: None,
            configuration_dirty: false,
            simulation_failed: false,
            scene_source: None,
            scene_history_len: 0,
        }
    }

    fn selected_scenario(&self) -> Result<Scenario, String> {
        match self.scenario_choice {
            ScenarioChoice::Stationary => Ok(Scenario::stationary()),
            ScenarioChoice::SixDofLoop => Ok(Scenario::six_dof_loop()),
            ScenarioChoice::FieldFigureEightTwice => Ok(Scenario::field_figure_eight_twice()),
            ScenarioChoice::PoseTeleport => Ok(Scenario::pose_teleport()),
            ScenarioChoice::VoFault => Ok(Scenario::vo_fault()),
            ScenarioChoice::Custom => self
                .custom_scenario
                .clone()
                .ok_or_else(|| "no custom scenario has been loaded or recorded".to_string()),
        }
    }

    fn rebuild(&mut self) -> bool {
        let result = self.selected_scenario().and_then(|scenario| {
            let mut simulation = LocalizationSimulation::new(scenario, self.config.clone())
                .map_err(|error| format!("{error:#}"))?;
            simulation
                .step()
                .and_then(|stepped| {
                    stepped
                        .then_some(())
                        .ok_or_else(|| color_eyre::eyre::eyre!("scenario contains no samples"))
                })
                .map_err(|error| format!("initial step failed: {error:#}"))?;
            Ok(simulation)
        });
        match result {
            Ok(simulation) => {
                self.simulation = Some(simulation);
                self.playing = false;
                self.fast_forward = false;
                self.playback_accumulator = Duration::ZERO;
                self.inspect_index = 0;
                self.configuration_dirty = false;
                self.applied_scenario_choice = self.scenario_choice;
                self.simulation_failed = false;
                self.scene_source = None;
                self.scene_history_len = 0;
                self.status = Some("Simulation rebuilt".to_string());
                true
            }
            Err(error) => {
                self.status = Some(format!("Rebuild failed: {error}"));
                false
            }
        }
    }

    fn step_once(&mut self) {
        let Some(simulation) = &mut self.simulation else {
            return;
        };
        match simulation.step() {
            Ok(true) => self.inspect_index = simulation.history().len() - 1,
            Ok(false) => {
                self.playing = false;
                self.fast_forward = false;
            }
            Err(error) => {
                self.playing = false;
                self.fast_forward = false;
                self.simulation_failed = true;
                self.status = Some(format!("Simulation step failed: {error:#}"));
            }
        }
    }

    fn advance_playback(&mut self, wall_delta: Duration) {
        if self.recorder.is_some() {
            return;
        }
        if self.fast_forward {
            for _ in 0..MAX_STEPS_PER_FRAME {
                let before = self
                    .simulation
                    .as_ref()
                    .map_or(0, |simulation| simulation.history().len());
                self.step_once();
                let after = self
                    .simulation
                    .as_ref()
                    .map_or(0, |simulation| simulation.history().len());
                if after == before || !self.fast_forward {
                    break;
                }
            }
            return;
        }
        if !self.playing {
            return;
        }
        self.playback_accumulator += wall_delta.mul_f32(self.playback_speed);
        let mut steps = 0;
        while self.playback_accumulator >= TICK_INTERVAL && steps < MAX_STEPS_PER_FRAME {
            self.playback_accumulator -= TICK_INTERVAL;
            self.step_once();
            steps += 1;
            if !self.playing {
                break;
            }
        }
    }

    fn controls(&mut self, ui: &mut Ui) {
        ui.heading("Localization Simulator");
        ui.label(RichText::new("Deterministic 20 ms logical clock").color(Color32::GRAY));
        ui.separator();

        ui.strong("Scenario and inputs");
        let previous_scenario_choice = self.scenario_choice;
        ComboBox::from_label("Scenario")
            .selected_text(self.scenario_choice.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.scenario_choice,
                    ScenarioChoice::Stationary,
                    "stationary",
                );
                ui.selectable_value(
                    &mut self.scenario_choice,
                    ScenarioChoice::SixDofLoop,
                    "six_dof_loop",
                );
                ui.selectable_value(
                    &mut self.scenario_choice,
                    ScenarioChoice::FieldFigureEightTwice,
                    "field_figure_eight_twice",
                );
                ui.selectable_value(
                    &mut self.scenario_choice,
                    ScenarioChoice::PoseTeleport,
                    "pose_teleport",
                );
                ui.selectable_value(
                    &mut self.scenario_choice,
                    ScenarioChoice::VoFault,
                    "vo_fault",
                );
                if self.custom_scenario.is_some() {
                    ui.selectable_value(
                        &mut self.scenario_choice,
                        ScenarioChoice::Custom,
                        "Custom",
                    );
                }
            });
        if self.scenario_choice != previous_scenario_choice {
            if self.scenario_choice == ScenarioChoice::VoFault {
                self.config.visual_odometry_mode = VisualOdometryMode::SyntheticDelta;
                self.config.vo_outlier = Some(default_vo_outlier());
            } else if previous_scenario_choice == ScenarioChoice::VoFault
                || self.scenario_choice == ScenarioChoice::PoseTeleport
            {
                self.config.vo_outlier = None;
            }
        }
        ComboBox::from_label("Association")
            .selected_text(match self.config.association_mode {
                AssociationMode::KnownCorrespondences => "Known correspondences",
                AssociationMode::ProductionAssociation => "Production association",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.config.association_mode,
                    AssociationMode::KnownCorrespondences,
                    "Known correspondences",
                );
                ui.selectable_value(
                    &mut self.config.association_mode,
                    AssociationMode::ProductionAssociation,
                    "Production association",
                );
            });
        let previous_vo_mode = self.config.visual_odometry_mode;
        ComboBox::from_label("Visual odometry")
            .selected_text(match self.config.visual_odometry_mode {
                VisualOdometryMode::SyntheticDelta => "Synthetic delta",
                VisualOdometryMode::ProductionStereo => "Production stereo",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.config.visual_odometry_mode,
                    VisualOdometryMode::SyntheticDelta,
                    "Synthetic delta",
                );
                ui.add_enabled_ui(self.scenario_choice != ScenarioChoice::VoFault, |ui| {
                    ui.selectable_value(
                        &mut self.config.visual_odometry_mode,
                        VisualOdometryMode::ProductionStereo,
                        "Production stereo",
                    );
                });
            });
        if self.config.visual_odometry_mode == VisualOdometryMode::ProductionStereo {
            if previous_vo_mode != self.config.visual_odometry_mode {
                self.config.vo_translation_sigma_m = 0.0;
                self.config.vo_rotation_sigma_rad = 0.0;
                self.config.vo_translation_bias_per_step = [0.0; 3];
                self.config.vo_rotation_bias_per_step = [0.0; 3];
                self.config.vo_outlier = None;
            }
            ui.label("Synthetic VO noise, bias and outliers are disabled in production stereo.");
            ui.add(
                Slider::new(&mut self.config.right_camera_delay_ms, 0.0..=20.0)
                    .step_by(0.1)
                    .text("Right camera delay (ms)"),
            );
            ui.checkbox(
                &mut self.config.assume_synchronized_stereo_timestamps,
                "Assume synchronized stereo timestamps",
            );
        }
        ui.horizontal(|ui| {
            ui.label("Seed");
            ui.add(egui::DragValue::new(&mut self.config.seed));
        });
        ui.add(
            Slider::new(&mut self.config.landmark_pixel_sigma, 0.0..=10.0)
                .text("Landmark sigma (px)"),
        );
        ui.add(
            Slider::new(&mut self.config.landmark_dropout_probability, 0.0..=1.0)
                .text("Landmark dropout"),
        );
        ui.add_enabled_ui(
            self.config.visual_odometry_mode == VisualOdometryMode::SyntheticDelta,
            |ui| {
                ui.add(
                    Slider::new(&mut self.config.vo_translation_sigma_m, 0.0..=0.05)
                        .text("VO translation sigma (m)"),
                );
                ui.add(
                    Slider::new(&mut self.config.vo_rotation_sigma_rad, 0.0..=0.05)
                        .text("VO rotation sigma (rad)"),
                );
                ui.collapsing("VO bias and one-shot outlier", |ui| {
                    vector_controls(
                        ui,
                        "Translation bias/step (m)",
                        &mut self.config.vo_translation_bias_per_step,
                        0.0001,
                    );
                    vector_controls(
                        ui,
                        "Rotation bias/step (rad)",
                        &mut self.config.vo_rotation_bias_per_step,
                        0.0001,
                    );

                    let mut outlier_enabled = self.config.vo_outlier.is_some();
                    if ui
                        .checkbox(&mut outlier_enabled, "Enable one-shot VO outlier")
                        .changed()
                    {
                        self.config.vo_outlier = outlier_enabled.then_some(default_vo_outlier());
                    }
                    if let Some(outlier) = &mut self.config.vo_outlier {
                        ui.horizontal(|ui| {
                            ui.label("Transition index");
                            ui.add(egui::DragValue::new(&mut outlier.transition_index).speed(1));
                        });
                        vector_controls(ui, "Translation (m)", &mut outlier.translation, 0.1);
                        vector_controls(
                            ui,
                            "Rotation scaled axis (rad)",
                            &mut outlier.rotation_scaled_axis,
                            0.05,
                        );
                    }
                });
            },
        );
        self.configuration_dirty = self.simulation.as_ref().is_none_or(|simulation| {
            simulation.config() != &self.config
                || self.scenario_choice != self.applied_scenario_choice
        });
        if self.configuration_dirty {
            self.playing = false;
            self.fast_forward = false;
            ui.colored_label(
                Color32::YELLOW,
                "Inputs changed. Rebuild before playback so the displayed results match.",
            );
        }
        if ui.button("Apply configuration / rebuild").clicked() {
            self.rebuild();
        }

        ui.separator();
        ui.strong("Playback");
        ui.add(
            Slider::new(&mut self.playback_speed, 0.1..=8.0)
                .logarithmic(true)
                .text("Speed"),
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.recorder.is_none()
                        && !self.fast_forward
                        && (self.playing || (!self.configuration_dirty && !self.simulation_failed)),
                    egui::Button::new(if self.playing { "Pause" } else { "Play" }),
                )
                .clicked()
            {
                self.playing = !self.playing;
                if self.playing {
                    self.inspect_index = self
                        .simulation
                        .as_ref()
                        .map_or(0, |simulation| simulation.history().len().saturating_sub(1));
                }
            }
            if ui
                .add_enabled(
                    !self.playing
                        && self.recorder.is_none()
                        && !self.configuration_dirty
                        && !self.simulation_failed,
                    egui::Button::new("Step"),
                )
                .clicked()
            {
                self.step_once();
            }
            if ui
                .add_enabled(self.recorder.is_none(), egui::Button::new("Restart"))
                .clicked()
            {
                self.rebuild();
            }
            if ui
                .add_enabled(
                    self.recorder.is_none()
                        && (self.fast_forward
                            || (!self.configuration_dirty && !self.simulation_failed)),
                    egui::Button::new(if self.fast_forward {
                        "Stop fast-forward"
                    } else {
                        "Run to end"
                    }),
                )
                .clicked()
            {
                self.playing = false;
                self.fast_forward = !self.fast_forward;
            }
        });

        let history_len = self
            .simulation
            .as_ref()
            .map_or(0, |simulation| simulation.history().len());
        if history_len > 0 {
            ui.add_enabled(
                !self.playing && !self.fast_forward && self.recorder.is_none(),
                Slider::new(&mut self.inspect_index, 0..=history_len - 1)
                    .text("Inspect history (read-only)"),
            );
        } else {
            ui.label("Inspect history (read-only): no samples");
        }
        ui.small("Resuming always returns the display to the latest generated sample.");

        ui.separator();
        self.flight_controls(ui);
        ui.separator();
        self.file_controls(ui);
        ui.separator();
        self.diagnostics(ui);
        ui.separator();
        ui.strong("View");
        ui.colored_label(Color32::from_rgb(0, 230, 242), "Cyan: truth");
        ui.colored_label(Color32::from_rgb(255, 209, 13), "Yellow: raw backend");
        ui.colored_label(Color32::from_rgb(255, 13, 191), "Magenta: live estimate");
        ui.label("Orbit: left drag | Pan: right drag | Zoom: wheel");
        if let Some(status) = &self.status {
            ui.separator();
            ui.label(status);
        }
    }

    fn flight_controls(&mut self, ui: &mut Ui) {
        ui.strong("Flight recorder");
        if self.recorder.is_none() {
            if ui
                .add_enabled(
                    !self.playing && !self.configuration_dirty && !self.simulation_failed,
                    egui::Button::new("Record flight"),
                )
                .clicked()
            {
                let pose = self
                    .displayed_sample()
                    .map(|sample| sample.truth_robot_to_field * fixed_robot_to_camera().inverse())
                    .or_else(|| {
                        self.simulation
                            .as_ref()
                            .map(|simulation| simulation.scenario().sample_camera_to_field(0.0))
                    })
                    .unwrap_or_else(linear_algebra::Isometry3::identity);
                self.recorder = Some(FlightRecorder {
                    pose,
                    elapsed: Duration::ZERO,
                    accumulator: Duration::ZERO,
                    keyframes: vec![PoseKeyframe::from_camera_to_field(0.0, pose)],
                });
                self.status = Some("Recording flight".to_string());
            }
        } else if ui.button("Stop and use recording").clicked() {
            self.stop_recording();
        }
        ui.small("Paused only. W/S camera local z, A/D local x, Q/E field vertical; arrows yaw/pitch, Z/C roll.");
    }

    fn update_flight(&mut self, context: &egui::Context, wall_delta: Duration) {
        let Some(recorder) = &mut self.recorder else {
            return;
        };
        recorder.accumulator += wall_delta;
        let wants_keyboard_input = context.egui_wants_keyboard_input();
        let (forward, right, vertical, yaw, pitch, roll) = context.input(|input| {
            if wants_keyboard_input {
                return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            }
            (
                axis(input.key_down(Key::W), input.key_down(Key::S)),
                axis(input.key_down(Key::D), input.key_down(Key::A)),
                axis(input.key_down(Key::E), input.key_down(Key::Q)),
                axis(
                    input.key_down(Key::ArrowLeft),
                    input.key_down(Key::ArrowRight),
                ),
                axis(input.key_down(Key::ArrowUp), input.key_down(Key::ArrowDown)),
                axis(input.key_down(Key::C), input.key_down(Key::Z)),
            )
        });
        while recorder.accumulator >= TICK_INTERVAL {
            recorder.accumulator -= TICK_INTERVAL;
            recorder.elapsed += TICK_INTERVAL;
            let dt = TICK_INTERVAL.as_secs_f32();
            let local_translation =
                recorder.pose.inner.rotation * Vector3::new(right, 0.0, forward) * dt;
            recorder.pose.inner.translation.vector += local_translation;
            recorder.pose.inner.translation.z += vertical * dt;
            let yaw_rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), yaw * dt);
            let local_rotation = UnitQuaternion::from_euler_angles(pitch * dt, 0.0, roll * dt);
            recorder.pose.inner.rotation =
                yaw_rotation * recorder.pose.inner.rotation * local_rotation;
            recorder.keyframes.push(PoseKeyframe::from_camera_to_field(
                recorder.elapsed.as_secs_f32(),
                recorder.pose,
            ));
        }
    }

    fn stop_recording(&mut self) {
        let Some(mut recorder) = self.recorder.take() else {
            return;
        };
        if recorder.keyframes.len() == 1 {
            recorder.keyframes.push(PoseKeyframe::from_camera_to_field(
                TICK_INTERVAL.as_secs_f32(),
                recorder.pose,
            ));
        }
        let duration_seconds = recorder.keyframes.last().unwrap().time_seconds();
        match Scenario::new("recorded_flight", duration_seconds, recorder.keyframes) {
            Ok(scenario) => {
                self.custom_scenario = Some(scenario);
                self.scenario_choice = ScenarioChoice::Custom;
                if self.rebuild() {
                    self.status =
                        Some("Recorded flight selected and simulation rebuilt".to_string());
                }
            }
            Err(error) => self.status = Some(format!("Recorded scenario invalid: {error}")),
        }
    }

    fn file_controls(&mut self, ui: &mut Ui) {
        ui.strong("Custom scenario JSON5");
        ui.text_edit_singleline(&mut self.scenario_path);
        ui.horizontal(|ui| {
            if ui.button("Load").clicked() {
                match std::fs::read_to_string(&self.scenario_path)
                    .map_err(|error| error.to_string())
                    .and_then(|contents| {
                        json5::from_str::<Scenario>(&contents).map_err(|error| error.to_string())
                    }) {
                    Ok(scenario) => {
                        self.custom_scenario = Some(scenario);
                        self.scenario_choice = ScenarioChoice::Custom;
                        if self.rebuild() {
                            self.status = Some(format!("Loaded {}", self.scenario_path));
                        }
                    }
                    Err(error) => self.status = Some(format!("Load failed: {error}")),
                }
            }
            if ui
                .add_enabled(self.custom_scenario.is_some(), egui::Button::new("Save"))
                .clicked()
                && let Some(scenario) = &self.custom_scenario
            {
                match json5::to_string(scenario)
                    .map_err(|error| error.to_string())
                    .and_then(|contents| {
                        std::fs::write(&self.scenario_path, format!("{contents}\n"))
                            .map_err(|error| error.to_string())
                    }) {
                    Ok(()) => self.status = Some(format!("Saved {}", self.scenario_path)),
                    Err(error) => self.status = Some(format!("Save failed: {error}")),
                }
            }
        });
    }

    fn diagnostics(&self, ui: &mut Ui) {
        ui.strong("Diagnostics");
        let Some(simulation) = &self.simulation else {
            ui.label("Simulation unavailable");
            return;
        };
        let Some(sample) = self.displayed_sample() else {
            ui.label(format!(
                "0.000 / {:.3} s",
                simulation.scenario().duration_seconds()
            ));
            return;
        };
        ui.label(format!(
            "Time: {:.3} / {:.3} s",
            sample.time.as_nanos() as f64 / 1.0e9,
            simulation.scenario().duration_seconds()
        ));
        ui.label(format!("Lock: {:?}", sample.global_visual_lock));
        if let Some(counts) = &sample.landmark_frame {
            ui.label(format!(
                "Marks ideal / detected / associated: {} / {} / {}",
                counts.ideal_visible, counts.emitted_detections, counts.associated
            ));
        } else {
            ui.label("Marks: no field-mark frame at this tick");
        }
        if let Some(diagnostics) = &sample.diagnostics {
            ui.label(format!(
                "Optimizer: {} ({:?} iterations)",
                diagnostics.termination, diagnostics.iterations
            ));
            ui.label(format!("Final cost: {:?}", diagnostics.final_cost));
            ui.label(format!("States: {}", diagnostics.state_count));
            ui.label(format!(
                "Active measurements: {}",
                diagnostics.measurement_count
            ));
        } else {
            ui.label("Optimizer: no solve yet");
        }
        if let Some(time) = sample.estimate_time {
            let truth = robot_to_field_from_camera_to_field(
                &simulation
                    .scenario()
                    .sample_camera_to_field(time.as_nanos() as f32 * 1e-9),
            )
            .inner;
            ui.label(format!(
                "Estimate age: {:.1} ms",
                sample.time.duration_since(time).as_secs_f64() * 1000.0
            ));
            if let Some(backend) = sample.raw_backend_robot_to_field.as_ref() {
                error_labels(
                    ui,
                    "Estimate-time truth vs backend",
                    &truth,
                    &backend.inner.cast::<f32>(),
                );
            }
            if let Some(live) = &sample.live_robot_to_field {
                error_labels(ui, "Estimate-time truth vs live", &truth, &live.inner);
            }
        }
    }

    fn displayed_sample(&self) -> Option<&SimulationHistorySample> {
        let simulation = self.simulation.as_ref()?;
        let index = if self.playing || self.fast_forward {
            simulation.history().len().saturating_sub(1)
        } else {
            self.inspect_index
                .min(simulation.history().len().saturating_sub(1))
        };
        simulation.history().get(index)
    }

    fn update_scene_state(&mut self) {
        if let Some(recorder) = &self.recorder {
            let target_len = recorder.keyframes.len();
            if self.scene_source == Some(SceneSource::Recorder)
                && self.scene_history_len == target_len
            {
                return;
            }
            let reset = self.scene_source != Some(SceneSource::Recorder)
                || target_len < self.scene_history_len;
            let start = if reset { 0 } else { self.scene_history_len };
            let new_truth = recorder.keyframes[start..]
                .iter()
                .map(|keyframe| {
                    robot_to_field_from_camera_to_field(&keyframe.camera_to_field()).inner
                })
                .collect::<Vec<_>>();
            let mut state = self
                .widget
                .bevy_app
                .world_mut()
                .resource_mut::<SceneState>();
            if reset {
                let revision = state.revision.wrapping_add(1);
                *state = SceneState {
                    revision,
                    ..Default::default()
                };
            }
            state.truth = Some(robot_to_field_from_camera_to_field(&recorder.pose).inner);
            state.truth_history.extend(new_truth);
            self.scene_source = Some(SceneSource::Recorder);
            self.scene_history_len = target_len;
            return;
        }

        let Some(simulation) = &self.simulation else {
            return;
        };
        if simulation.history().is_empty() {
            return;
        }
        let target_len = if self.playing || self.fast_forward {
            simulation.history().len()
        } else {
            self.inspect_index.min(simulation.history().len() - 1) + 1
        };
        if self.scene_source == Some(SceneSource::Simulation)
            && self.scene_history_len == target_len
        {
            return;
        }
        let reset = self.scene_source != Some(SceneSource::Simulation)
            || target_len < self.scene_history_len;
        let start = if reset { 0 } else { self.scene_history_len };
        let new_samples = &simulation.history()[start..target_len];
        let new_truth = new_samples
            .iter()
            .map(|sample| sample.truth_robot_to_field.inner)
            .collect::<Vec<_>>();
        let new_backend = new_samples
            .iter()
            .filter_map(|sample| {
                sample
                    .raw_backend_robot_to_field
                    .as_ref()
                    .map(|pose| pose.inner.cast())
            })
            .collect::<Vec<_>>();
        let new_live = new_samples
            .iter()
            .filter_map(|sample| sample.live_robot_to_field.as_ref().map(|pose| pose.inner))
            .collect::<Vec<_>>();
        let current = &simulation.history()[target_len - 1];
        let truth = current.truth_robot_to_field.inner;
        let backend = current
            .raw_backend_robot_to_field
            .as_ref()
            .map(|pose| pose.inner.cast());
        let live = current.live_robot_to_field.as_ref().map(|pose| pose.inner);
        let mut state = self
            .widget
            .bevy_app
            .world_mut()
            .resource_mut::<SceneState>();
        if reset {
            let revision = state.revision.wrapping_add(1);
            *state = SceneState {
                revision,
                ..Default::default()
            };
        }
        state.truth = Some(truth);
        state.backend = backend;
        state.live = live;
        state.truth_history.extend(new_truth);
        state.backend_history.extend(new_backend);
        state.live_history.extend(new_live);
        self.scene_source = Some(SceneSource::Simulation);
        self.scene_history_len = target_len;
    }
}

impl App for LocalizationSimulatorApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let now = Instant::now();
        let wall_delta = now
            .saturating_duration_since(self.last_frame)
            .min(MAX_WALL_DELTA);
        self.last_frame = now;
        self.advance_playback(wall_delta);
        self.update_flight(ui.ctx(), wall_delta);

        Panel::left("controls")
            .resizable(true)
            .default_size(330.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.controls(ui));
            });
        self.update_scene_state();
        CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("3D field");
                ui.label(
                    RichText::new("full 6-DoF poses and generated trails").color(Color32::GRAY),
                );
            });
            self.widget.ui(ui);
        });
        if self.playing || self.fast_forward || self.recorder.is_some() {
            ui.ctx().request_repaint();
        }
    }
}

fn axis(positive: bool, negative: bool) -> f32 {
    positive as u8 as f32 - negative as u8 as f32
}

fn default_vo_outlier() -> VisualOdometryOutlier {
    SimulationConfig::diagnostic_vo_fault()
}

fn error_labels(ui: &mut Ui, name: &str, truth: &Isometry3<f32>, estimate: &Isometry3<f32>) {
    let translation = (truth.translation.vector - estimate.translation.vector).norm();
    let rotation = (truth.rotation.inverse() * estimate.rotation)
        .angle()
        .to_degrees();
    ui.label(format!("{name}: {translation:.3} m / {rotation:.2} deg"));
}

fn vector_controls(ui: &mut Ui, label: &str, values: &mut [f32; 3], speed: f64) {
    ui.label(label);
    ui.horizontal(|ui| {
        for (axis, value) in ["x", "y", "z"].into_iter().zip(values) {
            ui.label(axis);
            ui.add(egui::DragValue::new(value).speed(speed));
        }
    });
}

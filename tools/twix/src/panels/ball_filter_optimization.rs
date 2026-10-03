use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use color_eyre::{Result, eyre::WrapErr};
use coordinate_systems::{Field, Ground};
use eframe::egui::{self, Ui};
use linear_algebra::{Isometry2, Point3, Pose2, point, vector};
use ros_z::time::Time;
use ros_z_debug::{SampleRecord, TopicObservation};
use serde::{Deserialize, Serialize};
use tokio::{sync::oneshot, task::JoinHandle};
use twix_visualization::twix_painter::{Orientation, TwixPainter};
use types::ball_filter_tuning::{
    Metrics, NAMESPACE, OPEN_VIEWER_TOPIC, OPPONENTS_TOPIC, OpponentParameters, PROGRESS_TOPIC,
    Progress, ROUTER, TUNED_PARAMETER_POINTERS, WALKING_SPEED_TOPIC, walking_speed_scale_is_valid,
};
use types::{
    ball_position::BallPosition, field_dimensions::FieldDimensions, time_wrapper::TimeWrapper,
};

use crate::{
    backend::RobotBackend,
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationRepaint, RepaintOnUpdates},
};

pub struct BallFilterOptimizationPanel {
    connection: Option<Connection>,
    pending: Option<PendingConnection>,
    error: Option<String>,
    viewer_request: Option<oneshot::Receiver<Result<()>>>,
    opponents_request: Option<oneshot::Receiver<Result<()>>>,
    opponents_status: Option<String>,
    walking_speed_request: Option<oneshot::Receiver<Result<()>>>,
    walking_speed_status: Option<String>,
    run_history: RunHistory,
    history_request: Option<oneshot::Receiver<Result<RunHistory>>>,
    history_loaded: bool,
    history_error: Option<String>,
    delete_confirmation: Option<String>,
    startup: StartupSettings,
    launch: Option<Arc<Mutex<LaunchStatus>>>,
    auto_connect: bool,
    next_connect_attempt: Instant,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct StartupSettings {
    recordings: String,
    output: String,
    host: String,
    workers: u32,
    manifests: String,
    refresh_minutes: u32,
    opponent_count: u32,
    opponent_width: f32,
    walking_speed_scale: f32,
}

impl Default for StartupSettings {
    fn default() -> Self {
        Self {
            recordings: String::new(),
            output: new_output_directory(),
            host: "remote-compiler".into(),
            workers: 32,
            manifests: String::new(),
            refresh_minutes: 5,
            opponent_count: 2,
            opponent_width: 0.44,
            walking_speed_scale: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum StartupAction {
    Local,
    Remote,
    Connect,
}

#[derive(Clone, Debug)]
struct LaunchStatus {
    active: bool,
    message: String,
    pid: Option<u32>,
    log_path: PathBuf,
    log_tail: String,
}

struct PendingConnection {
    receiver: oneshot::Receiver<Result<Arc<RobotBackend>>>,
    task: JoinHandle<()>,
}

impl Drop for PendingConnection {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Connection {
    progress: TopicObservation<Progress>,
    truth: TopicObservation<TimeWrapper<Vec<Point3<Field>>>>,
    obstacles: TopicObservation<TimeWrapper<Vec<nalgebra::Point3<f32>>>>,
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
    field_dimensions: TopicObservation<FieldDimensions>,
    estimate: TopicObservation<Option<BallPosition<Ground>>>,
    _repaint: ObservationRepaint,
    _backend: Arc<RobotBackend>,
    sample: Option<Arc<SampleRecord<Progress>>>,
    received_at: Instant,
    history: Vec<(u64, f64)>,
    run: String,
}

impl Panel for BallFilterOptimizationPanel {
    const STORAGE_ID: &'static str = "ball_filter_optimization";
    const DISPLAY_NAME: &'static str = "Ball-filter optimization";
    const ICON: &'static str = egui_material_icons::icons::ICON_TUNE.codepoint;

    fn new(context: PanelCreationContext<'_>) -> Self {
        Self {
            connection: None,
            pending: None,
            error: None,
            viewer_request: None,
            opponents_request: None,
            opponents_status: None,
            walking_speed_request: None,
            walking_speed_status: None,
            run_history: RunHistory::default(),
            history_request: None,
            history_loaded: false,
            history_error: None,
            delete_confirmation: None,
            startup: context
                .value
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .unwrap_or_default(),
            launch: None,
            auto_connect: false,
            next_connect_attempt: Instant::now(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        ui.strong("Ball-filter optimization");
        self.startup_ui(ui, &context);
        self.history_ui(ui);
        self.opponents_ui(ui, &context);
        self.walking_speed_ui(ui, &context);
        if self.auto_connect
            && self.connection.is_none()
            && self.pending.is_none()
            && Instant::now() >= self.next_connect_attempt
        {
            self.connect(&context);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.pending.is_none(),
                    egui::Button::new("Connect to simulator / optimizer"),
                )
                .clicked()
            {
                self.auto_connect = false;
                self.connect(&context);
            }
            if let Some(connection) = &self.connection {
                if ui
                    .add_enabled(
                        self.viewer_request.is_none(),
                        egui::Button::new("Open 3D view"),
                    )
                    .clicked()
                {
                    let node = connection._backend.node();
                    let repaint = context.egui_context.clone();
                    let (sender, receiver) = oneshot::channel();
                    context.backend.runtime_handle().spawn(async move {
                        let result = async {
                            let publisher =
                                node.publisher::<bool>(OPEN_VIEWER_TOPIC).build().await?;
                            publisher.publish(&true).await?;
                            Ok::<_, color_eyre::Report>(())
                        };
                        let result = tokio::time::timeout(Duration::from_secs(3), result)
                            .await
                            .map_err(color_eyre::Report::from)
                            .and_then(|result| result);
                        let _ = sender.send(result);
                        repaint.request_repaint();
                    });
                    self.viewer_request = Some(receiver);
                }
            }
            if self.connection.is_some() && ui.button("Disconnect").clicked() {
                self.connection = None;
                self.auto_connect = false;
            }
        });
        if let Some(pending) = &mut self.pending {
            match pending.receiver.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result.and_then(|backend| {
                        let progress = observe_progress(&backend)?;
                        let repaint = progress.repaint_on_updates(&PanelUiContext {
                            backend: &backend,
                            egui_context: context.egui_context,
                        });
                        let truth = backend
                            .observer()
                            .observe_typed("simulation/ball_ground_truth_field")?
                            .spawn();
                        let estimate = backend
                            .observer()
                            .observe_typed("ball_filter/ball_position")?
                            .spawn();
                        let obstacles = backend
                            .observer()
                            .observe_typed("simulation/obstacle_positions_world")?
                            .spawn();
                        let ground_to_field = backend
                            .observer()
                            .observe_typed("ground_to_field")?
                            .policy(ros_z_debug::ObservationPolicy::time_window(
                                Duration::from_secs(1),
                            )?)
                            .spawn();
                        let field_dimensions = backend
                            .observer()
                            .observe_typed("field_dimensions")?
                            .policy(
                                ros_z_debug::ObservationPolicy::latest().with_subscriber_qos(
                                    ros_z::qos::QosProfile {
                                        durability: ros_z::qos::QosDurability::TransientLocal,
                                        ..Default::default()
                                    },
                                ),
                            )
                            .spawn();
                        Ok(Connection {
                            truth,
                            obstacles,
                            ground_to_field,
                            field_dimensions,
                            estimate,
                            progress,
                            _repaint: repaint,
                            _backend: backend,
                            sample: None,
                            received_at: Instant::now(),
                            history: Vec::new(),
                            run: String::new(),
                        })
                    }) {
                        Ok(connection) => {
                            self.connection = Some(connection);
                            self.auto_connect = false;
                            self.error = None;
                        }
                        Err(error) => {
                            self.error = Some(format!("{error:#}"));
                            self.next_connect_attempt = Instant::now() + Duration::from_secs(2);
                        }
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    ui.spinner();
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.pending = None;
                }
            }
        }
        if let Some(request) = &mut self.viewer_request {
            match request.try_recv() {
                Ok(result) => {
                    self.viewer_request = None;
                    if let Err(error) = result {
                        self.error = Some(format!("Could not request 3D view: {error:#}"));
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.viewer_request = None;
                }
            }
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let Some(connection) = &mut self.connection else {
            if self.auto_connect {
                ui.label(
                    "Waiting for the startup helper to bring the local progress bridge online…",
                );
                ui.ctx().request_repaint_after(Duration::from_millis(500));
            }
            ui.label("You can also connect to an optimizer started with:");
            monospace(
                ui,
                "./simulator --tune-ball-filter logs/my-run --keep-tuning-open",
            );
            return;
        };
        ui.ctx().request_repaint_after(Duration::from_millis(500));
        let latest = connection.progress.latest();
        if let Some(sample) = latest {
            if connection
                .sample
                .as_ref()
                .is_none_or(|old| !Arc::ptr_eq(old, &sample))
            {
                connection.received_at = Instant::now();
                if connection.run != sample.value.output_directory {
                    connection.run = sample.value.output_directory.clone();
                    connection.history.clear();
                }
                if let Some(search) = &sample.value.search {
                    let trial = sample
                        .value
                        .remote
                        .as_ref()
                        .map_or(search.trial, |remote| remote.completed_trials);
                    if connection
                        .history
                        .last()
                        .is_none_or(|(previous_trial, _)| *previous_trial != trial)
                    {
                        connection.history.push((trial, search.best.loss));
                    }
                }
                connection.sample = Some(sample);
            }
        }
        let Some(sample) = &connection.sample else {
            ui.label("Waiting for the local simulator / optimizer…");
            monospace(ui, ROUTER);
            match connection.progress.status() {
                ros_z_debug::TopicObservationStatus::Retrying { error, .. } => {
                    ui.label(format!("Retrying connection: {error}"));
                }
                ros_z_debug::TopicObservationStatus::Blocked { reason, .. } => {
                    ui.label(format!("Connection blocked: {reason:?}"));
                }
                ros_z_debug::TopicObservationStatus::Observing { cache } => {
                    ui.label("Observing optimizer progress");
                    if let Some(message) = cache.message() {
                        ui.label(message);
                    }
                }
                _ => {
                    ui.label("Connecting…");
                }
            }
            return;
        };
        let age = connection.received_at.elapsed().as_secs_f32();
        if age > 3.0 {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!("No updates for {age:.0}s — showing the last received state."),
            );
        } else {
            ui.label(if sample.value.remote.is_some() {
                "Connected to the local bridge for remote optimization"
            } else {
                "Connected to local simulator / optimizer"
            });
        }
        let progress = &sample.value;
        let status_width = ui.available_width();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(status_width);
            ui.add(egui::Label::new(egui::RichText::new(&progress.status).strong()).wrap());
            remote_status(ui, progress, unix_seconds());
            if let Some(status) = &progress.viewer_status {
                ui.label(status);
            }
            if let Some(status) = &progress.live_status {
                ui.label(status);
                ui.label(&progress.phase);
            }
            if let Some(error) = &progress.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            if progress.search.is_none() && progress.live_status.is_none() && progress.remote.is_none() {
                ui.label(format!(
                    "Recording {} / {}: {}",
                    progress.recording_index, progress.recordings, progress.recording
                ));
                ui.label(&progress.phase);
                ui.add(
                    egui::ProgressBar::new(
                        (progress.elapsed_seconds / progress.duration_seconds.max(1.0)) as f32,
                    )
                    .text(format!(
                        "{:.1} / {:.0} seconds",
                        progress.elapsed_seconds, progress.duration_seconds
                    )),
                );
            }
            if progress.status == "Recording" || progress.live_trial.is_some() {
                live_ball(ui, connection);
            }
            if let Some(search) = &progress.search {
                if progress.remote.is_some() {
                    ui.label(format!("Accepted best selection revision {}", search.best_trial));
                } else if search.trials == 0 {
                    ui.label(format!("Trial {} · continuous search", search.trial));
                } else {
                    ui.add(
                        egui::ProgressBar::new(search.trial as f32 / search.trials.max(1) as f32)
                            .text(format!("Trial {} / {}", search.trial, search.trials)),
                    );
                }
                ui.label(format!(
                    "Training recordings · {} coordinates",
                    search.reference_frame
                ));
                scores(ui, "training", &search.baseline, &search.best);
                loss_plot(
                    ui,
                    &connection.history,
                    progress.remote.as_ref().map_or(search.trials.max(search.trial), |remote| remote.completed_trials),
                    search.baseline.loss,
                );
                if let (Some(baseline), Some(best)) =
                    (&search.validation_baseline, &search.validation_best)
                {
                    ui.separator();
                    ui.label("Held-out recordings (not used to choose parameters)");
                    scores(ui, "validation", baseline, best);
                }
                ui.label(
                    "Loss includes position error, missed balls and false tracks. Lower is better.",
                );
                if let Ok(mut fixed) = serde_json::to_value(&search.best_parameters) {
                    let mut tuned = serde_json::Map::new();
                    for pointer in TUNED_PARAMETER_POINTERS {
                        if fixed.pointer(pointer).is_some_and(serde_json::Value::is_null) {
                            continue; // Legacy optional rates remain fixed, not searched.
                        }
                        let (parent, key) =
                            pointer.rsplit_once('/').expect("static parameter pointer");
                        if let Some(value) = fixed
                            .pointer_mut(parent)
                            .and_then(serde_json::Value::as_object_mut)
                            .and_then(|map| map.remove(key))
                        {
                            tuned.insert(pointer.trim_start_matches('/').replace('/', "."), value);
                        }
                    }
                    ui.collapsing("Best tuned values", |ui| {
                        let variables = 6 + usize::from(search.best_parameters.hidden_validity_decay_rate.is_some())
                            + usize::from(search.best_parameters.visible_missed_validity_decay_rate.is_some())
                            + usize::from(search.best_parameters.competing_hypothesis_validity_decay_rate.is_some());
                        ui.label(format!("{variables} search variables; x/y noise values are coupled."));
                        if let Ok(json) = serde_json::to_string_pretty(&tuned) {
                            monospace(ui, json);
                        }
                    });
                    ui.collapsing("Fixed values (not searched)", |ui| {
                        ui.label("maximum_matching_cost_validity_penalty_factor is retained for old configuration compatibility and is no longer used.");
                        if let Ok(json) = serde_json::to_string_pretty(&fixed) {
                            monospace(ui, json);
                        }
                    });
                }
            }
            ui.separator();
            ui.label("Recordings and results:");
            monospace(ui, &progress.output_directory);
        });
    }

    fn save(&self) -> serde_json::Value {
        serde_json::to_value(&self.startup).unwrap_or_default()
    }
}

impl BallFilterOptimizationPanel {
    fn connect(&mut self, context: &PanelUiContext<'_>) {
        self.connection = None;
        self.error = None;
        let runtime = context.backend.runtime_handle().clone();
        let handle = runtime.clone();
        let repaint = context.egui_context.clone();
        let (sender, receiver) = oneshot::channel();
        let task = runtime.spawn(async move {
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                RobotBackend::new(handle, Some(ROUTER.into()), NAMESPACE.into()),
            )
            .await
            .map_err(color_eyre::Report::from)
            .and_then(|result| result)
            .map(Arc::new);
            let _ = sender.send(result);
            repaint.request_repaint();
        });
        self.pending = Some(PendingConnection { receiver, task });
    }

    fn startup_ui(&mut self, ui: &mut Ui, context: &PanelUiContext<'_>) {
        let status = self
            .launch
            .as_ref()
            .and_then(|launch| launch.try_lock().ok().map(|value| value.clone()));
        let active = self.launch.is_some() && status.as_ref().is_none_or(|status| status.active);
        if self.launch.is_some() && !active && status.is_some() {
            self.auto_connect = false;
        }
        let mut action = None;
        egui::CollapsingHeader::new("Start or attach to optimization").default_open(true).show(ui, |ui| {
            ui.add_enabled_ui(!active, |ui| {
                egui::Grid::new("ball_filter_startup_fields").num_columns(2).show(ui, |ui| {
                    ui.label("Existing recordings").on_hover_text("Optional. Leave empty to capture a new simulation run before optimization.");
                    ui.add(egui::TextEdit::singleline(&mut self.startup.recordings).hint_text("Optional recording directory").desired_width(f32::INFINITY));
                    ui.end_row();
                    ui.label("Output directory");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.startup.output).desired_width(260.0));
                        if ui.button("New path").clicked() {
                            self.startup.output = new_output_directory();
                        }
                    });
                    ui.end_row();
                    ui.label("Remote host");
                    ui.text_edit_singleline(&mut self.startup.host);
                    ui.end_row();
                    ui.label("New recordings every (minutes)").on_hover_text("Remote optimization only. Defaults to 5 minutes. Zero keeps the initial recordings.");
                    ui.add(egui::DragValue::new(&mut self.startup.refresh_minutes).range(0..=1440));
                    ui.end_row();
                    ui.label("Opponents");
                    ui.add(egui::DragValue::new(&mut self.startup.opponent_count).range(0..=8));
                    ui.end_row();
                    ui.label("Opponent diameter (m)");
                    ui.add(egui::DragValue::new(&mut self.startup.opponent_width).range(0.1..=1.2).speed(0.01));
                    ui.end_row();
                    ui.label("Walking speed ×").on_hover_text("Human-controlled multiplier for normal behavior walking commands. Policy speed limits still apply; the optimizer does not tune this value.");
                    ui.add(egui::DragValue::new(&mut self.startup.walking_speed_scale).range(0.1..=3.0).speed(0.05));
                    ui.end_row();
                    ui.label("Remote workers");
                    ui.add(egui::DragValue::new(&mut self.startup.workers).range(1..=32));
                    ui.end_row();
                });
                ui.label("Paths are relative to the repository. Closing this panel leaves the run active.");
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Start local optimization").clicked() { action = Some(StartupAction::Local); }
                    if ui.button("Start remote optimization").clicked() { action = Some(StartupAction::Remote); }
                });
                ui.collapsing("Connect existing remote runs", |ui| {
                    ui.label("Local manifest files, one path per line:");
                    ui.add(egui::TextEdit::multiline(&mut self.startup.manifests).desired_rows(2).desired_width(f32::INFINITY).hint_text("logs/remote-run/manifest.json"));
                    if ui.button("Connect remote runs").clicked() { action = Some(StartupAction::Connect); }
                });
            });
            if let Some(status) = &status {
                ui.add(egui::Label::new(&status.message).wrap());
                if let Some(pid) = status.pid { ui.label(format!("Startup helper PID: {pid}")); }
                ui.horizontal_wrapped(|ui| {
                    ui.label("Startup log:");
                    monospace(ui, status.log_path.display().to_string());
                    if ui.button("Copy log path").clicked() { ui.ctx().copy_text(status.log_path.display().to_string()); }
                });
                if !status.log_tail.is_empty() {
                    ui.collapsing("Startup log tail", |ui| {
                        egui::ScrollArea::vertical().max_height(180.0).stick_to_bottom(true).show(ui, |ui| { monospace(ui, &status.log_tail); });
                    });
                }
            }
        });
        if let Some(action) = action {
            match startup_arguments(&self.startup, action)
                .and_then(|arguments| spawn_startup(arguments, context.egui_context.clone()))
            {
                Ok(launch) => {
                    self.launch = Some(launch);
                    self.error = None;
                    self.pending = None;
                    self.connection = None;
                    self.auto_connect = true;
                    self.next_connect_attempt = Instant::now();
                }
                Err(error) => self.error = Some(format!("Could not start optimization: {error:#}")),
            }
        }
        if active || self.auto_connect {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct RunHistory {
    updated_unix_seconds: f64,
    runs: Vec<HistoryRun>,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct HistoryRun {
    id: String,
    kind: String,
    name: String,
    location: String,
    date: String,
    status: String,
    stale: bool,
    can_delete: bool,
    delete_reason: Option<String>,
    error: Option<String>,
    completed_trials: u64,
    verified: bool,
    report: Option<String>,
    metrics: HistoryMetrics,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct HistoryMetrics {
    training_baseline: Option<HistoryScore>,
    training_optimized: Option<HistoryScore>,
    validation_baseline: Option<HistoryScore>,
    validation_optimized: Option<HistoryScore>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct HistoryScore {
    loss: Option<f64>,
    close_range_position_rmse_metres: Option<f64>,
    missing_seconds: Option<f64>,
    motion_lag_seconds: Option<f64>,
}

impl RunHistory {
    fn mark_cached(&mut self) {
        for run in &mut self.runs {
            run.stale = true;
            run.can_delete = false;
            run.delete_reason = Some("Refresh history to verify current activity".into());
        }
    }
}

#[derive(Clone, Debug)]
enum HistoryAction {
    Cached,
    Refresh,
    Delete(String),
}

fn history_arguments(action: &HistoryAction) -> Vec<String> {
    match action {
        HistoryAction::Cached => Vec::new(),
        HistoryAction::Refresh => vec!["list".into(), "--json".into()],
        HistoryAction::Delete(id) => {
            vec!["delete".into(), "--id".into(), id.clone(), "--json".into()]
        }
    }
}

fn read_run_history(action: HistoryAction) -> Result<RunHistory> {
    let root = repository_root();
    if matches!(action, HistoryAction::Cached) {
        let path = root.join("logs/ball-filter-run-history.json");
        let mut history: RunHistory = match File::open(&path) {
            Ok(file) => {
                serde_json::from_reader(file).wrap_err("could not read saved run history")?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => RunHistory::default(),
            Err(error) => return Err(error.into()),
        };
        history.mark_cached();
        return Ok(history);
    }
    let output = Command::new("python3")
        .arg(root.join("scripts/ball_filter_run_history"))
        .args(history_arguments(&action))
        .current_dir(&root)
        .stdin(Stdio::null())
        .output()
        .wrap_err("could not start run-history helper")?;
    color_eyre::eyre::ensure!(
        output.status.success(),
        "Run-history helper: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).wrap_err("could not decode run history")?;
    let value = if matches!(action, HistoryAction::Delete(_)) {
        value
            .get("history")
            .cloned()
            .ok_or_else(|| color_eyre::eyre::eyre!("missing updated history"))?
    } else {
        value
    };
    serde_json::from_value(value).wrap_err("invalid run history")
}

impl BallFilterOptimizationPanel {
    fn request_history(&mut self, action: HistoryAction, repaint: egui::Context) {
        let (sender, receiver) = oneshot::channel();
        match std::thread::Builder::new()
            .name("ball-filter-history".into())
            .spawn(move || {
                let _ = sender.send(read_run_history(action));
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.history_request = Some(receiver);
                self.history_error = None;
            }
            Err(error) => {
                self.history_error = Some(format!("Could not start history helper: {error}"))
            }
        }
    }

    fn history_ui(&mut self, ui: &mut Ui) {
        if !self.history_loaded {
            self.history_loaded = true;
            self.request_history(HistoryAction::Cached, ui.ctx().clone());
        }
        if let Some(request) = &mut self.history_request {
            match request.try_recv() {
                Ok(result) => {
                    self.history_request = None;
                    match result {
                        Ok(history) => self.run_history = history,
                        Err(error) => self.history_error = Some(format!("{error:#}")),
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    ui.ctx().request_repaint_after(Duration::from_millis(250));
                }
                Err(error) => {
                    self.history_request = None;
                    self.history_error = Some(error.to_string());
                }
            }
        }
        let busy = self.history_request.is_some();
        let mut action = None;
        egui::CollapsingHeader::new(format!("Past optimization runs ({})", self.run_history.runs.len()))
            .id_salt("ball_filter_run_history").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(!busy, egui::Button::new("Refresh history")).clicked() { action = Some(HistoryAction::Refresh); }
                if busy { ui.spinner(); ui.label("Reading run history…"); }
                if self.run_history.updated_unix_seconds > 0.0 {
                    ui.label(format!("Snapshot {:.0} min ago", ((unix_seconds() - self.run_history.updated_unix_seconds) / 60.0).max(0.0)));
                }
            });
            if let Some(error) = &self.history_error { ui.colored_label(ui.visuals().error_fg_color, error); }
            for warning in &self.run_history.warnings { ui.colored_label(ui.visuals().warn_fg_color, warning); }
            ui.label("Each run uses its own recordings. Best candidates are selected by training loss; held-out results remain separate.");
            egui::ScrollArea::vertical().id_salt("ball_filter_history_rows").max_height(330.0).show(ui, |ui| {
                for run in &self.run_history.runs {
                    ui.push_id(&run.id, |ui| {
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(&run.name);
                            ui.label(format!("{} · {} · {} · {} trials", run.kind, run.status, run.date, run.completed_trials));
                            if run.stale { ui.colored_label(ui.visuals().warn_fg_color, "Cached / stale"); }
                        });
                        ui.label(&run.location);
                        history_scores(ui, &run.metrics);
                        if let Some(error) = &run.error { ui.colored_label(ui.visuals().warn_fg_color, error); }
                        ui.collapsing("Report details", |ui| {
                            if let Some(report) = &run.report { ui.label(report); }
                            ui.label(if run.verified { "Recorded output replay verified" } else { "Replay verification unavailable" });
                        });
                        if self.delete_confirmation.as_deref() == Some(&run.id) {
                            ui.label(format!("Move {} and its recordings/results to recoverable trash?", run.location));
                            ui.horizontal(|ui| {
                                if ui.add_enabled(!busy && run.can_delete && !run.stale, egui::Button::new("Confirm move to trash")).clicked() {
                                    action = Some(HistoryAction::Delete(run.id.clone()));
                                    self.delete_confirmation = None;
                                }
                                if ui.button("Cancel").clicked() { self.delete_confirmation = None; }
                            });
                        } else {
                            let response = ui.add_enabled(!busy && run.can_delete && !run.stale, egui::Button::new("Move to trash"));
                            if response.clicked() { self.delete_confirmation = Some(run.id.clone()); }
                            if let Some(reason) = &run.delete_reason { response.on_hover_text(reason); ui.label(reason); }
                        }
                    });
                }
                if self.run_history.runs.is_empty() { ui.label("Refresh to discover local runs and known remote sessions."); }
            });
        });
        if let Some(action) = action {
            self.request_history(action, ui.ctx().clone());
        }
    }

    fn walking_speed_ui(&mut self, ui: &mut Ui, context: &PanelUiContext<'_>) {
        if let Some(request) = &mut self.walking_speed_request {
            match request.try_recv() {
                Ok(result) => {
                    self.walking_speed_request = None;
                    self.walking_speed_status = Some(match result {
                        Ok(()) => "Speed request sent; waiting for the next live episode.".into(),
                        Err(error) => format!("Could not update walking speed: {error:#}"),
                    });
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    ui.ctx().request_repaint_after(Duration::from_millis(250));
                }
                Err(error) => {
                    self.walking_speed_request = None;
                    self.walking_speed_status = Some(error.to_string());
                }
            }
        }
        let Some(connection) = &self.connection else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.label("Live walking speed ×").on_hover_text("Scales normal behavior walking commands within the policy limits. Existing recordings keep their original speed.");
            ui.add(egui::DragValue::new(&mut self.startup.walking_speed_scale).range(0.1..=3.0).speed(0.05));
            if ui.add_enabled(
                self.walking_speed_request.is_none() && walking_speed_scale_is_valid(self.startup.walking_speed_scale),
                egui::Button::new("Apply walking speed"),
            ).clicked() {
                let scale = self.startup.walking_speed_scale;
                let node = connection._backend.node();
                let repaint = context.egui_context.clone();
                let (sender, receiver) = oneshot::channel();
                context.backend.runtime_handle().spawn(async move {
                    let result = async {
                        let publisher = node.publisher::<f32>(WALKING_SPEED_TOPIC).build().await?;
                        publisher.publish(&scale).await?;
                        Ok::<_, color_eyre::Report>(())
                    };
                    let result = tokio::time::timeout(Duration::from_secs(3), result).await
                        .map_err(color_eyre::Report::from).and_then(|result| result);
                    let _ = sender.send(result);
                    repaint.request_repaint();
                });
                self.walking_speed_request = Some(receiver);
                self.walking_speed_status = None;
            }
        });
        if let Some(sample) = connection.progress.latest() {
            let progress = &sample.value;
            let active = progress
                .active_walking_speed_scale
                .map(|scale| format!("{scale:.2}×"))
                .unwrap_or_else(|| "waiting for episode".into());
            ui.label(format!(
                "Walking speed requested: {:.2}× · Active: {active}",
                progress.walking_speed_scale
            ));
        }
        if let Some(status) = &self.walking_speed_status {
            ui.add(egui::Label::new(status).wrap());
        }
    }

    fn opponents_ui(&mut self, ui: &mut Ui, context: &PanelUiContext<'_>) {
        if let Some(request) = &mut self.opponents_request {
            match request.try_recv() {
                Ok(result) => {
                    self.opponents_request = None;
                    self.opponents_status = Some(match result {
                        Ok(()) => "Request sent; waiting for the next live episode.".into(),
                        Err(error) => format!("Could not update opponents: {error:#}"),
                    });
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    ui.ctx().request_repaint_after(Duration::from_millis(250));
                }
                Err(error) => {
                    self.opponents_request = None;
                    self.opponents_status = Some(error.to_string());
                }
            }
        }
        let Some(connection) = &self.connection else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.label("Live opponents");
            ui.add(egui::DragValue::new(&mut self.startup.opponent_count).range(0..=8));
            ui.label("Diameter (m)");
            ui.add(
                egui::DragValue::new(&mut self.startup.opponent_width)
                    .range(0.1..=1.2)
                    .speed(0.01),
            );
            if ui
                .add_enabled(
                    self.opponents_request.is_none(),
                    egui::Button::new("Apply opponents"),
                )
                .clicked()
            {
                let parameters = OpponentParameters {
                    count: self.startup.opponent_count,
                    width: self.startup.opponent_width,
                };
                let node = connection._backend.node();
                let repaint = context.egui_context.clone();
                let (sender, receiver) = oneshot::channel();
                context.backend.runtime_handle().spawn(async move {
                    let result = async {
                        let publisher = node
                            .publisher::<OpponentParameters>(OPPONENTS_TOPIC)
                            .build()
                            .await?;
                        publisher.publish(&parameters).await?;
                        Ok::<_, color_eyre::Report>(())
                    };
                    let result = tokio::time::timeout(Duration::from_secs(3), result)
                        .await
                        .map_err(color_eyre::Report::from)
                        .and_then(|result| result);
                    let _ = sender.send(result);
                    repaint.request_repaint();
                });
                self.opponents_request = Some(receiver);
                self.opponents_status = None;
            }
        });
        if let Some(sample) = connection.progress.latest() {
            let progress = &sample.value;
            let requested = &progress.opponents;
            let actual = progress
                .active_opponents
                .as_ref()
                .map(|actual| format!("{} × {:.2} m", actual.count, actual.width))
                .unwrap_or_else(|| "waiting for episode".into());
            ui.label(format!(
                "Requested: {} × {:.2} m · Active: {actual}",
                requested.count, requested.width
            ));
        }
        if let Some(status) = &self.opponents_status {
            ui.label(status);
        }
        ui.label("Changes restart the live episode; existing recordings remain unchanged.");
    }
}

fn history_scores(ui: &mut Ui, metrics: &HistoryMetrics) {
    fn number(value: Option<f64>) -> String {
        value
            .filter(|value| value.is_finite())
            .map_or_else(|| "—".into(), |value| format!("{value:.3}"))
    }
    egui::Grid::new("run_scores").num_columns(5).show(ui, |ui| {
        for label in [
            "Baseline → best",
            "Loss",
            "Close RMSE (m)",
            "Missing (s)",
            "Motion lag (s)",
        ] {
            ui.label(label);
        }
        ui.end_row();
        for (label, baseline, best) in [
            (
                "Training",
                &metrics.training_baseline,
                &metrics.training_optimized,
            ),
            (
                "Held out",
                &metrics.validation_baseline,
                &metrics.validation_optimized,
            ),
        ] {
            ui.label(label);
            for values in [
                (
                    baseline.as_ref().and_then(|m| m.loss),
                    best.as_ref().and_then(|m| m.loss),
                ),
                (
                    baseline
                        .as_ref()
                        .and_then(|m| m.close_range_position_rmse_metres),
                    best.as_ref()
                        .and_then(|m| m.close_range_position_rmse_metres),
                ),
                (
                    baseline.as_ref().and_then(|m| m.missing_seconds),
                    best.as_ref().and_then(|m| m.missing_seconds),
                ),
                (
                    baseline.as_ref().and_then(|m| m.motion_lag_seconds),
                    best.as_ref().and_then(|m| m.motion_lag_seconds),
                ),
            ] {
                ui.label(format!("{} → {}", number(values.0), number(values.1)));
            }
            ui.end_row();
        }
    });
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("Twix is located at tools/twix")
        .to_path_buf()
}

fn unix_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn new_output_directory() -> String {
    format!(
        "logs/ball-tuning-{}-{}",
        (unix_seconds() * 1000.0) as u64,
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

fn startup_arguments(settings: &StartupSettings, action: StartupAction) -> Result<Vec<String>> {
    color_eyre::eyre::ensure!(
        !settings.output.trim().is_empty(),
        "Output directory is required"
    );
    color_eyre::eyre::ensure!(
        walking_speed_scale_is_valid(settings.walking_speed_scale),
        "Walking speed multiplier must be finite and between 0.1 and 3"
    );
    let mode = match action {
        StartupAction::Local => "local",
        StartupAction::Remote => "remote",
        StartupAction::Connect => "connect",
    };
    let mut arguments = vec![
        mode.into(),
        "--output".into(),
        settings.output.trim().into(),
    ];
    match action {
        StartupAction::Local | StartupAction::Remote => {
            color_eyre::eyre::ensure!(
                settings.opponent_count <= 8
                    && settings.opponent_width.is_finite()
                    && (0.1..=1.2).contains(&settings.opponent_width),
                "Opponents must be 0–8, with diameter 0.1–1.2 metres"
            );
            arguments.extend([
                "--trials".into(),
                "256".into(),
                "--opponents".into(),
                settings.opponent_count.to_string(),
                "--opponent-width".into(),
                settings.opponent_width.to_string(),
            ]);
            if !settings.recordings.trim().is_empty() {
                arguments.extend(["--recordings".into(), settings.recordings.trim().into()]);
            }
            if matches!(action, StartupAction::Remote) {
                color_eyre::eyre::ensure!(
                    !settings.host.trim().is_empty(),
                    "Remote host is required"
                );
                color_eyre::eyre::ensure!(
                    (1..=32).contains(&settings.workers),
                    "Remote workers must be between 1 and 32"
                );
                arguments.extend([
                    "--host".into(),
                    settings.host.trim().into(),
                    "--workers".into(),
                    settings.workers.to_string(),
                    "--refresh-minutes".into(),
                    settings.refresh_minutes.to_string(),
                ]);
            }
        }
        StartupAction::Connect => {
            let manifests: Vec<_> = settings
                .manifests
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect();
            color_eyre::eyre::ensure!(
                !manifests.is_empty(),
                "Add at least one local manifest path"
            );
            for manifest in manifests {
                arguments.extend(["--manifest".into(), manifest.into()]);
            }
        }
    }
    arguments.extend([
        "--walking-speed-scale".into(),
        settings.walking_speed_scale.to_string(),
    ]);
    Ok(arguments)
}

fn spawn_startup(
    arguments: Vec<String>,
    repaint: egui::Context,
) -> Result<Arc<Mutex<LaunchStatus>>> {
    let log_path = std::env::temp_dir().join(format!(
        "hulk-ball-filter-start-{}.log",
        uuid::Uuid::new_v4()
    ));
    let state = Arc::new(Mutex::new(LaunchStatus {
        active: true,
        message: "Starting optimization helper…".into(),
        pid: None,
        log_path,
        log_tail: String::new(),
    }));
    let background_state = state.clone();
    std::thread::Builder::new()
        .name("ball-filter-startup".into())
        .spawn(move || {
            let result = monitor_startup(&arguments, &background_state, &repaint);
            if let Ok(mut state) = background_state.lock() {
                state.active = false;
                state.message = match result {
                    Ok(status) if status.success() => "Startup helper finished.".into(),
                    Ok(status) => format!("Startup helper failed: {status}. See the startup log."),
                    Err(error) => format!("Startup failed: {error:#}"),
                };
                state.log_tail = read_log_tail(&state.log_path).unwrap_or_default();
            }
            repaint.request_repaint();
        })
        .wrap_err("could not start the background launcher")?;
    Ok(state)
}

fn monitor_startup(
    arguments: &[String],
    state: &Arc<Mutex<LaunchStatus>>,
    repaint: &egui::Context,
) -> Result<std::process::ExitStatus> {
    let root = repository_root();
    let log_path = state
        .lock()
        .map_err(|_| color_eyre::eyre::eyre!("launcher state lock poisoned"))?
        .log_path
        .clone();
    let log = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&log_path)
        .wrap_err_with(|| format!("could not create {}", log_path.display()))?;
    let mut command = Command::new("python3");
    command
        .arg(root.join("scripts/ball_filter_optimization"))
        .args(arguments)
        .current_dir(&root)
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .wrap_err("could not launch python3 optimization helper")?;
    if let Ok(mut state) = state.lock() {
        state.pid = Some(child.id());
        state.message = "Optimization helper is running.".into();
    }
    repaint.request_repaint();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Arc::strong_count(state) == 1 {
            // The panel closed. Keep reaping the child without ending its run.
            return child.wait().map_err(Into::into);
        }
        let tail = read_log_tail(&log_path).unwrap_or_default();
        if let Ok(mut state) = state.lock() {
            state.log_tail = tail;
        }
        repaint.request_repaint();
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn read_log_tail(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let start = file.metadata()?.len().saturating_sub(8192);
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = Vec::new();
    file.take(8192).read_to_end(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

#[derive(Debug, PartialEq)]
enum RemoteFreshness {
    Unknown,
    Age(f64),
    FutureTimestamp,
}

fn remote_freshness(updated: Option<f64>, now: f64) -> RemoteFreshness {
    match updated {
        Some(updated) if updated.is_finite() && now.is_finite() && updated > 0.0 => {
            if updated > now + 5.0 {
                RemoteFreshness::FutureTimestamp
            } else {
                RemoteFreshness::Age((now - updated).max(0.0))
            }
        }
        _ => RemoteFreshness::Unknown,
    }
}

fn remote_status(ui: &mut Ui, progress: &Progress, now: f64) {
    let Some(remote) = &progress.remote else {
        return;
    };
    ui.label(format!("Remote host: {}", remote.host));
    match remote_freshness(progress.remote_updated_unix_seconds, now) {
        RemoteFreshness::Age(age) if age > 10.0 => {
            ui.colored_label(ui.visuals().warn_fg_color,
            format!("Remote status is {age:.0}s old. Local bridge updates do not confirm that SSH or remote workers are responding."));
        }
        RemoteFreshness::Age(age) => {
            ui.label(format!("Remote status received {age:.0}s ago"));
        }
        RemoteFreshness::Unknown => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "No successful remote status update has been received yet.",
            );
        }
        RemoteFreshness::FutureTimestamp => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "Remote status timestamp is ahead of this computer's clock; its age is unknown.",
            );
        }
    }
    ui.label(format!(
        "Completed trials in reported rounds: {}",
        remote.completed_trials
    ));
    ui.horizontal_wrapped(|ui| {
        ui.label("Best candidate:");
        monospace(
            ui,
            if remote.best_candidate.is_empty() {
                "None reported"
            } else {
                &remote.best_candidate
            },
        );
    });
    ui.label("Workers report progress by round.");
    ui.collapsing(format!("Workers ({})", remote.workers.len()), |ui| {
        egui::Grid::new("remote_optimizer_workers")
            .striped(true)
            .show(ui, |ui| {
                for heading in ["Run", "Worker", "Round", "Status"] {
                    ui.strong(heading);
                }
                ui.end_row();
                for worker in &remote.workers {
                    ui.label(&worker.run);
                    ui.label(worker.worker.to_string());
                    ui.label(worker.round.to_string());
                    ui.label(&worker.status);
                    ui.end_row();
                }
            });
    });
}

fn monospace(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let size = egui::TextStyle::Body.resolve(ui.style()).size;
    ui.label(egui::RichText::new(text).monospace().size(size))
}

fn observe_progress(backend: &RobotBackend) -> Result<TopicObservation<Progress>> {
    use ros_z::qos::{QosDurability, QosHistory, QosProfile};
    Ok(backend
        .observer()
        .observe_typed(PROGRESS_TOPIC)?
        .policy(
            ros_z_debug::ObservationPolicy::latest().with_subscriber_qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            }),
        )
        .spawn())
}

fn scores(ui: &mut Ui, id: &str, baseline: &Metrics, best: &Metrics) {
    egui::Grid::new(id).striped(true).show(ui, |ui| {
        ui.label("");
        ui.strong("Baseline");
        ui.strong("Best");
        ui.end_row();
        for (name, a, b) in [
            ("Loss", Some(baseline.loss), Some(best.loss)),
            (
                "Position RMSE (m)",
                baseline.position_rmse_metres,
                best.position_rmse_metres,
            ),
            (
                "Close-range RMSE (m)",
                baseline.close_range_position_rmse_metres,
                best.close_range_position_rmse_metres,
            ),
            (
                "Close-range missing (s)",
                Some(baseline.close_range_missing_seconds),
                Some(best.close_range_missing_seconds),
            ),
            (
                "Along-motion lag (ms)",
                baseline.motion_lag_seconds.map(|value| value * 1000.0),
                best.motion_lag_seconds.map(|value| value * 1000.0),
            ),
            (
                "Motion-lag coverage (s)",
                Some(baseline.moving_reference_seconds),
                Some(best.moving_reference_seconds),
            ),
            (
                "Missing ball (s)",
                Some(baseline.missing_seconds),
                Some(best.missing_seconds),
            ),
            (
                "Missing ground-to-field (s)",
                Some(baseline.missing_transform_seconds),
                Some(best.missing_transform_seconds),
            ),
            (
                "Longest missing gap (s)",
                Some(baseline.longest_missing_seconds),
                Some(best.longest_missing_seconds),
            ),
            (
                "False track (s)",
                Some(baseline.false_track_seconds),
                Some(best.false_track_seconds),
            ),
        ] {
            let label = ui.label(name);
            if name == "Loss" {
                label.on_hover_text("Single-ball position and availability loss. Reference balls outside the field receive smoothly decreasing weight with distance beyond the boundary (decay length 0.3 m). Raw error and missing-time metrics below are not downweighted.");
            } else if name == "Missing ball (s)" {
                label.on_hover_text("Total labelled time when a real ball exists but no filter estimate is available in the scoring frame. Includes balls outside the camera view and, for field scoring, estimates without a matching ground-to-field transform. Lower is better.");
            } else if name == "Longest missing gap (s)" {
                label.on_hover_text("Longest continuous labelled interval with a real ball but no usable estimate. Includes startup and missing field transforms; this is not specifically the time to recover after a kick.");
            } else if name == "Along-motion lag (ms)" {
                label.on_hover_text("Signed along-motion position error divided by reference speed at the same timestamp. Positive means behind the ball. Uses adjacent single-ball references in field coordinates with valid motion; multi-ball and missing estimates are excluded. This measures spatial lag, not message delivery latency.");
            } else if name == "Close-range RMSE (m)" {
                label.on_hover_text("Position error against the nearest labelled ball within 1 m of the robot, conditional on an available estimate. Read together with close-range missing time.");
            }
            ui.label(a.map_or_else(|| "—".into(), |v| format!("{v:.4}")));
            ui.label(b.map_or_else(|| "—".into(), |v| format!("{v:.4}")));
            ui.end_row();
        }
    });
}

fn loss_plot(ui: &mut Ui, history: &[(u64, f64)], trials: u64, baseline: f64) {
    if history.len() < 2 {
        return;
    }
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), 120.0),
        egui::Sense::hover(),
    );
    let rect = response.rect.shrink(6.0);
    let points = history
        .iter()
        .map(|(trial, loss)| {
            egui::pos2(
                rect.left() + rect.width() * *trial as f32 / trials.max(1) as f32,
                rect.bottom()
                    - rect.height() * (loss / baseline.max(f64::EPSILON)).clamp(0.0, 1.0) as f32,
            )
        })
        .collect();
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, ui.visuals().weak_text_color()),
    );
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
    ));
    ui.label("Best training loss over trials (received since connecting)");
}

fn recent_pose(connection: &Connection, time: Time) -> Option<Isometry2<Ground, Field>> {
    connection
        .ground_to_field
        .get_nearest(time)
        .filter(|pose| pose.source_time.as_nanos().abs_diff(time.as_nanos()) <= 20_000_000)
        .map(|pose| pose.value)
}

fn live_ball(ui: &mut Ui, connection: &Connection) {
    let Some(dimensions) = connection.field_dimensions.latest() else {
        ui.label("Waiting for field dimensions…");
        return;
    };
    let Some(truth) = connection.truth.latest() else {
        return;
    };
    let robot_pose = recent_pose(connection, truth.value.time);
    let estimate = connection.estimate.latest().filter(|estimate| {
        estimate
            .source_time
            .as_nanos()
            .abs_diff(truth.value.time.as_nanos())
            <= 100_000_000
    });
    let estimate_in_field = estimate.as_ref().and_then(|estimate| {
        let ball = estimate.value.as_ref()?;
        let pose = recent_pose(connection, estimate.source_time)?;
        Some(pose * ball.position)
    });
    let dimensions = &dimensions.value;
    let border = dimensions.border_strip_width;
    let length = dimensions.length + 2.0 * border;
    let width = dimensions.width + 2.0 * border;
    let pixel_width = ui.available_width().min(360.0 * length / width);
    let margin = (ui.available_width() - pixel_width) / 2.0;
    ui.horizontal(|ui| {
        ui.add_space(margin);
        ui.allocate_ui(
            egui::vec2(pixel_width, pixel_width * width / length),
            |ui| {
                let (_, painter) = TwixPainter::<Field>::allocate(
                    ui,
                    vector![length, width],
                    point![length / 2.0, -width / 2.0],
                    Orientation::RightHanded,
                );
                painter.field(dimensions);
                if let Some(pose) = robot_pose {
                    painter.pose(
                        Pose2::new(pose * point![0.0, 0.0], pose.inner.rotation.angle()),
                        0.15,
                        0.4,
                        egui::Color32::WHITE,
                        egui::Stroke::new(0.035, egui::Color32::BLACK),
                    );
                }
                for ball in &truth.value.inner {
                    painter.circle_stroke(
                        ball.xy(),
                        0.14,
                        egui::Stroke::new(0.045, egui::Color32::GREEN),
                    );
                }
                if let Some(obstacles) = connection.obstacles.latest().filter(|sample| {
                    sample
                        .value
                        .time
                        .as_nanos()
                        .abs_diff(truth.value.time.as_nanos())
                        <= 100_000_000
                }) {
                    for obstacle in &obstacles.value.inner {
                        painter.circle_filled(
                            point![obstacle.x, obstacle.y],
                            0.22,
                            egui::Color32::from_rgb(255, 115, 13),
                        );
                    }
                }
                if let Some(ball) = estimate_in_field {
                    painter.circle_filled(ball, 0.08, egui::Color32::LIGHT_BLUE);
                }
            },
        );
    });
    ui.label("Field coordinates · white: robot · green: true balls · blue: live filter · orange: opponents");
    if let Some(estimate) = connection.estimate.latest() {
        let age_ms = (truth.value.time.as_nanos() - estimate.source_time.as_nanos()) as f64 / 1e6;
        ui.label(format!("Filter state is {age_ms:.0} ms behind the latest physics sample"))
            .on_hover_text("Timestamp difference, separate from tracking error. The model is displayed at its recorded state time without extrapolation; fusion and transport can contribute to this age.");
    }
    if robot_pose.is_none() {
        ui.label("Robot pose unavailable: no ground-to-field transform within 20 ms.");
    }
    ui.label(
        "Search scores use single-ball recordings; this map shows the unscored live simulation.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_history_preserves_holdout_tradeoffs_but_disables_deletion() {
        let mut history: RunHistory = serde_json::from_value(serde_json::json!({
            "runs": [{"id": "remote-example", "can_delete": true, "metrics": {
                "validation_baseline": {"loss": 1.408, "close_range_position_rmse_metres": 0.472, "missing_seconds": 4.798},
                "validation_optimized": {"loss": 1.072, "close_range_position_rmse_metres": 0.784, "missing_seconds": 1.076, "motion_lag_seconds": null}
            }}]
        })).unwrap();
        history.mark_cached();
        let run = &history.runs[0];
        assert!(run.stale);
        assert!(!run.can_delete);
        let baseline = run.metrics.validation_baseline.as_ref().unwrap();
        let best = run.metrics.validation_optimized.as_ref().unwrap();
        assert!(best.loss < baseline.loss);
        assert!(best.close_range_position_rmse_metres > baseline.close_range_position_rmse_metres);
        assert!(best.motion_lag_seconds.is_none());
        assert!(run.metrics.training_optimized.is_none());
    }

    #[test]
    fn history_delete_passes_only_the_exact_id_as_a_literal_argument() {
        assert_eq!(
            history_arguments(&HistoryAction::Delete("remote-a; $(not-a-command)".into())),
            ["delete", "--id", "remote-a; $(not-a-command)", "--json"]
        );
        assert_eq!(
            history_arguments(&HistoryAction::Refresh),
            ["list", "--json"]
        );
    }

    #[test]
    fn walking_speed_is_human_configurable_for_all_launch_modes() {
        let mut settings = StartupSettings {
            walking_speed_scale: 2.5,
            manifests: "logs/existing/manifest.json".into(),
            ..Default::default()
        };
        for action in [
            StartupAction::Local,
            StartupAction::Remote,
            StartupAction::Connect,
        ] {
            let arguments = startup_arguments(&settings, action).unwrap();
            assert!(
                arguments
                    .windows(2)
                    .any(|pair| pair == ["--walking-speed-scale", "2.5"])
            );
            for invalid in [0.0, 3.1, f32::NAN, f32::INFINITY] {
                settings.walking_speed_scale = invalid;
                assert!(startup_arguments(&settings, action).is_err());
            }
            settings.walking_speed_scale = 2.5;
        }
        let legacy: StartupSettings = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(legacy.walking_speed_scale, 1.0);
    }

    #[test]
    fn startup_scenario_arguments_validate_ranges_and_disable_remote_refresh() {
        let mut settings = StartupSettings {
            refresh_minutes: 0,
            opponent_count: 8,
            opponent_width: 1.2,
            ..Default::default()
        };
        let remote = startup_arguments(&settings, StartupAction::Remote).unwrap();
        assert!(
            remote
                .windows(2)
                .any(|pair| pair == ["--refresh-minutes", "0"])
        );
        let local = startup_arguments(&settings, StartupAction::Local).unwrap();
        assert!(!local.contains(&"--refresh-minutes".to_string()));
        settings.opponent_count = 9;
        assert!(startup_arguments(&settings, StartupAction::Local).is_err());
        settings.opponent_count = 2;
        settings.opponent_width = f32::NAN;
        assert!(startup_arguments(&settings, StartupAction::Remote).is_err());
    }

    #[test]
    fn startup_arguments_keep_paths_literal_and_capture_when_recordings_are_empty() {
        let settings = StartupSettings {
            output: "logs/new run; $(do-not-run)".into(),
            ..Default::default()
        };
        assert_eq!(
            startup_arguments(&settings, StartupAction::Local).unwrap(),
            [
                "local",
                "--output",
                "logs/new run; $(do-not-run)",
                "--trials",
                "256",
                "--opponents",
                "2",
                "--opponent-width",
                "0.44",
                "--walking-speed-scale",
                "1"
            ]
        );
        let settings = StartupSettings {
            recordings: "logs/existing run".into(),
            ..settings
        };
        let args = startup_arguments(&settings, StartupAction::Remote).unwrap();
        assert_eq!(
            args,
            [
                "remote",
                "--output",
                "logs/new run; $(do-not-run)",
                "--trials",
                "256",
                "--opponents",
                "2",
                "--opponent-width",
                "0.44",
                "--recordings",
                "logs/existing run",
                "--host",
                "remote-compiler",
                "--workers",
                "32",
                "--refresh-minutes",
                "5",
                "--walking-speed-scale",
                "1"
            ]
        );
    }

    #[test]
    fn remote_connect_accepts_multiple_manifest_paths_without_splitting_spaces() {
        let settings = StartupSettings {
            output: "logs/bridge".into(),
            manifests: "  logs/first run/manifest.json  \n\n/another/run.json\n".into(),
            ..Default::default()
        };
        assert_eq!(
            startup_arguments(&settings, StartupAction::Connect).unwrap(),
            [
                "connect",
                "--output",
                "logs/bridge",
                "--manifest",
                "logs/first run/manifest.json",
                "--manifest",
                "/another/run.json",
                "--walking-speed-scale",
                "1"
            ]
        );
    }

    #[test]
    fn incomplete_startup_requests_are_rejected_before_starting_a_process() {
        let mut settings = StartupSettings::default();
        assert!(startup_arguments(&settings, StartupAction::Connect).is_err());
        settings.workers = 0;
        assert!(startup_arguments(&settings, StartupAction::Remote).is_err());
        settings.workers = 33;
        assert!(startup_arguments(&settings, StartupAction::Remote).is_err());
        settings.workers = 32;
        settings.host.clear();
        assert!(startup_arguments(&settings, StartupAction::Remote).is_err());
        settings.output.clear();
        assert!(startup_arguments(&settings, StartupAction::Local).is_err());
    }

    #[test]
    fn startup_defaults_make_unique_repository_relative_outputs_and_restore_settings() {
        let first = StartupSettings::default();
        let second = StartupSettings::default();
        assert_ne!(first.output, second.output);
        assert!(Path::new(&first.output).starts_with("logs"));
        assert!(!Path::new(&first.output).is_absolute());
        assert!(repository_root().join("tools/twix/Cargo.toml").is_file());
        let restored: StartupSettings =
            serde_json::from_value(serde_json::json!({"recordings": "logs/existing"})).unwrap();
        assert_eq!(restored.recordings, "logs/existing");
        assert_eq!(restored.host, "remote-compiler");
        assert_eq!(restored.workers, 32);
    }

    #[test]
    fn remote_freshness_uses_remote_poll_time_not_local_receipt_time() {
        assert_eq!(
            remote_freshness(Some(50.0), 100.0),
            RemoteFreshness::Age(50.0)
        );
        assert_eq!(remote_freshness(None, 100.0), RemoteFreshness::Unknown);
        assert_eq!(remote_freshness(Some(0.0), 100.0), RemoteFreshness::Unknown);
        assert_eq!(
            remote_freshness(Some(f64::NAN), 100.0),
            RemoteFreshness::Unknown
        );
        assert_eq!(
            remote_freshness(Some(110.0), 100.0),
            RemoteFreshness::FutureTimestamp
        );
        assert_eq!(
            remote_freshness(Some(100.2), 100.0),
            RemoteFreshness::Age(0.0)
        );
        let mut legacy = serde_json::to_value(Progress::default()).unwrap();
        legacy.as_object_mut().unwrap().remove("remote");
        legacy
            .as_object_mut()
            .unwrap()
            .remove("remote_updated_unix_seconds");
        let legacy: Progress = serde_json::from_value(legacy).unwrap();
        assert!(legacy.remote.is_none());
        assert!(legacy.remote_updated_unix_seconds.is_none());
    }

    #[test]
    fn startup_log_tail_is_bounded_and_handles_partial_utf8() {
        let path =
            std::env::temp_dir().join(format!("twix-startup-log-test-{}", uuid::Uuid::new_v4()));
        let content = "é".repeat(5000) + "done!";
        std::fs::write(&path, content).unwrap();
        let tail = read_log_tail(&path).unwrap();
        assert!(tail.ends_with("done!"));
        assert!(tail.len() <= 8195);
        std::fs::remove_file(path).unwrap();
    }

    // Exercise the actual network path used by the button, including joining a
    // publisher after its first update and decoding the complete typed message.
    #[tokio::test(flavor = "multi_thread")]
    async fn local_optimizer_progress_reaches_twix_when_connecting_late() {
        use ros_z::{
            prelude::*,
            qos::{QosDurability, QosHistory},
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let server = ContextBuilder::default()
            .with_mode("router")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints([endpoint.as_str()])
            .build()
            .await
            .unwrap();
        let node = server
            .create_node("test_optimizer")
            .with_namespace(NAMESPACE)
            .build()
            .await
            .unwrap();
        let publisher = node
            .publisher::<Progress>(PROGRESS_TOPIC)
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            })
            .build()
            .await
            .unwrap();
        publisher
            .publish(&Progress {
                status: "Complete".into(),
                remote: Some(types::ball_filter_tuning::RemoteProgress {
                    host: "remote-compiler".into(),
                    completed_trials: 32,
                    best_candidate: "worker-2/round-1".into(),
                    workers: vec![types::ball_filter_tuning::RemoteWorker {
                        run: "test-run".into(),
                        worker: 2,
                        round: 1,
                        status: "complete".into(),
                    }],
                }),
                remote_updated_unix_seconds: Some(123.0),
                search: Some(types::ball_filter_tuning::SearchProgress {
                    trial: 32,
                    trials: 32,
                    best: Metrics {
                        loss: 0.5,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await
            .unwrap();
        let backend = RobotBackend::new(
            tokio::runtime::Handle::current(),
            Some(endpoint),
            NAMESPACE.into(),
        )
        .await
        .unwrap();
        let observation = observe_progress(&backend).unwrap();
        let received = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Some(sample) = observation.latest() {
                    break sample;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("late Twix observer should receive retained progress");
        assert_eq!(received.value.status, "Complete");
        let remote = received.value.remote.as_ref().unwrap();
        assert_eq!(remote.host, "remote-compiler");
        assert_eq!(remote.completed_trials, 32);
        assert_eq!(remote.workers[0].worker, 2);
        assert_eq!(remote.workers[0].round, 1);
        assert_eq!(received.value.remote_updated_unix_seconds, Some(123.0));
        let search = received.value.search.as_ref().unwrap();
        assert_eq!(search.trial, 32);
        assert_eq!(search.best.loss, 0.5);
        drop(observation);
        drop(backend);
        drop(publisher);
        drop(node);
        server.shutdown().unwrap();
    }
}

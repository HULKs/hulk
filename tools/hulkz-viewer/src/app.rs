mod bindings;
mod layout;
mod panel_catalog;
mod panels;
mod persistence;
mod state;
mod time_fmt;
mod timeline_state;

use std::time::Duration;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use std::{panic::catch_unwind, panic::AssertUnwindSafe};

use color_eyre::{eyre::WrapErr as _, Result};
use eframe::{egui, App, CreationContext, Frame, Storage};
use egui_dock::{DockArea, DockState};
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::{
    model::{DiscoveryOp, WorkerCommand, WorkerEvent},
    worker::run_worker,
};

use self::layout::{
    apply_overrides_to_primary_text_panel, ensure_stream_tab_exists, highest_parameter_panel_id,
    highest_stream_id, initial_dock_state, sanitize_dock_splits, ViewerTabHost,
};
use self::persistence::{load_persisted_dock_state, load_persisted_ui_state, save_persisted_state};
pub(crate) use self::state::ViewerApp;
pub use self::state::ViewerStartupOverrides;
use self::state::{
    DiscoveryState, ParameterPanelStatus, ParameterPanelTab, RuntimeState, ShellState,
    TextPanelTab, TimelineState, TimelineViewportState, UiState, ViewerTab, WorkspaceState,
    DEFAULT_TIMELINE_LANE_HEIGHT_PX,
};
pub(crate) use self::state::{LaneRenderRow, TimelineRenderRange};
pub(crate) use self::time_fmt::format_timestamp;
pub(crate) use self::timeline_state::is_manual_timeline_navigation;

impl ViewerApp {
    pub fn new(
        creation_context: &CreationContext<'_>,
        startup_overrides: ViewerStartupOverrides,
    ) -> Result<Self> {
        info!("starting hulkz-viewer app runtime");
        let runtime = Runtime::new().wrap_err("failed to create tokio runtime")?;

        let mut config = crate::model::ViewerConfig::default();
        let ViewerStartupOverrides {
            namespace: override_namespace,
            source_expression: override_source_expression,
            storage_path: override_storage_path,
        } = startup_overrides;

        if let Some(namespace) = &override_namespace {
            config.namespace = namespace.clone();
        }
        if let Some(source_expression) = &override_source_expression {
            config.source_expression = source_expression.clone();
        }
        config.storage_path = override_storage_path;

        let default_stream = TextPanelTab::new(0, config.source_expression.clone());
        let default_parameter_panel = ParameterPanelTab::new(0);
        let mut dock_state =
            load_persisted_dock_state(creation_context.storage).unwrap_or_else(|| {
                initial_dock_state(default_stream.clone(), default_parameter_panel.clone())
            });
        ensure_stream_tab_exists(&mut dock_state, default_stream.clone());
        if sanitize_dock_splits(&mut dock_state) {
            warn!("sanitized invalid persisted dock split fractions");
        }

        if override_source_expression.is_some() {
            apply_overrides_to_primary_text_panel(
                &mut dock_state,
                override_source_expression.as_deref(),
            );
        }

        let persisted_ui = load_persisted_ui_state(creation_context.storage);
        let default_namespace_input = override_namespace
            .clone()
            .or_else(|| {
                persisted_ui
                    .as_ref()
                    .map(|state| state.default_namespace.clone())
            })
            .unwrap_or_default();
        let ingest_enabled = persisted_ui
            .as_ref()
            .map(|state| state.ingest_enabled)
            .unwrap_or(true);
        let follow_live = persisted_ui
            .as_ref()
            .map(|state| state.follow_live)
            .unwrap_or(true);
        let show_discovery = persisted_ui
            .as_ref()
            .map(|state| state.show_discovery)
            .unwrap_or(true);
        let show_timeline = persisted_ui
            .as_ref()
            .map(|state| state.show_timeline)
            .unwrap_or(true);

        let mut next_stream_id = persisted_ui
            .as_ref()
            .map(|state| state.next_stream_id)
            .unwrap_or_else(|| highest_stream_id(&dock_state).saturating_add(1));
        let min_next = highest_stream_id(&dock_state).saturating_add(1);
        if next_stream_id < min_next {
            next_stream_id = min_next;
        }

        let mut next_parameter_panel_id = persisted_ui
            .as_ref()
            .map(|state| state.next_parameter_panel_id)
            .unwrap_or_else(|| highest_parameter_panel_id(&dock_state).saturating_add(1));
        let min_next_parameter = highest_parameter_panel_id(&dock_state).saturating_add(1);
        if next_parameter_panel_id < min_next_parameter {
            next_parameter_panel_id = min_next_parameter;
        }

        let cancellation_token = CancellationToken::new();
        let (command_tx, command_rx) = mpsc::channel(config.worker_command_channel_capacity.max(1));
        let (event_tx, event_rx) = mpsc::channel(config.worker_event_channel_capacity.max(1));
        let worker_wake_armed = Arc::new(AtomicBool::new(false));
        let wake_notifier = crate::model::WorkerWakeNotifier::new({
            let egui_ctx = creation_context.egui_ctx.clone();
            let worker_wake_armed = worker_wake_armed.clone();
            let repaint_delay = config.repaint_delay_on_activity;
            move || {
                if !worker_wake_armed.swap(true, Ordering::SeqCst) {
                    egui_ctx.request_repaint_after(repaint_delay);
                }
            }
        });

        let worker_task = runtime.spawn(run_worker(
            config.clone(),
            command_rx,
            event_tx,
            cancellation_token.clone(),
            Some(wake_notifier),
        ));
        info!("worker task spawned");

        let now = std::time::Instant::now();
        let last_scrub_emitted = now.checked_sub(config.scrub_debounce).unwrap_or(now);

        let mut app = Self {
            config,
            shell: ShellState {
                show_discovery,
                show_timeline,
            },
            discovery: DiscoveryState::default(),
            timeline: TimelineState {
                global_timeline: Vec::new(),
                global_timeline_index: None,
                timeline_hover_preview: None,
                timeline_viewport: TimelineViewportState {
                    lane_height_px: DEFAULT_TIMELINE_LANE_HEIGHT_PX,
                    ..TimelineViewportState::default()
                },
                stream_lane_bindings: std::collections::BTreeMap::new(),
                timeline_lanes: std::collections::BTreeMap::new(),
                lane_order_cache: Vec::new(),
                lane_order_dirty: true,
                pending_scrub_anchor: None,
                last_scrub_emitted,
            },
            workspace: WorkspaceState {
                dock_state,
                stream_states: std::collections::BTreeMap::new(),
                binding_cache: std::collections::BTreeMap::new(),
                next_stream_id,
                next_parameter_panel_id,
            },
            runtime: RuntimeState {
                runtime,
                worker_task: Some(worker_task),
                cancellation_token,
                command_tx,
                pending_commands: VecDeque::new(),
                worker_wake_armed,
                event_rx,
                shutdown_started: false,
            },
            ui: UiState {
                ingest_enabled,
                follow_live,
                default_namespace: default_namespace_input.clone(),
                default_namespace_input,
                ready: false,
                last_error: None,
                backend_stats: None,
                frame_last_ms: 0.0,
                frame_ema_ms: 0.0,
                frame_processed_events: 0,
                frame_processed_event_bytes: 0,
            },
        };

        app.reconcile_text_panels();
        app.update_discovery_namespace();

        if !app.ui.ingest_enabled {
            app.send_command(WorkerCommand::SetIngestEnabled(false));
        }

        Ok(app)
    }

    fn send_command(&mut self, command: WorkerCommand) {
        debug!(?command, "sending worker command");
        self.runtime.pending_commands.push_back(command);
    }

    fn run_pending_commands(&mut self) {
        loop {
            let Some(command) = self.runtime.pending_commands.pop_front() else {
                break;
            };
            match self.runtime.command_tx.try_send(command) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(command)) => {
                    self.runtime.pending_commands.push_front(command);
                    break;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    self.ui.last_error = Some("worker command channel is closed".to_string());
                    warn!("failed to send worker command: channel closed");
                    break;
                }
            }
        }
    }

    fn drain_worker_events(&mut self) {
        let started = std::time::Instant::now();
        let mut processed_events = 0_usize;
        let mut processed_event_bytes = 0_usize;

        loop {
            if processed_events >= self.config.max_events_per_frame
                || processed_event_bytes >= self.config.max_event_bytes_per_frame
                || started.elapsed() >= self.config.max_event_ingest_time_per_frame
            {
                break;
            }

            match self.runtime.event_rx.try_recv() {
                Ok(envelope) => {
                    processed_events = processed_events.saturating_add(1);
                    processed_event_bytes =
                        processed_event_bytes.saturating_add(envelope.approx_bytes);
                    match envelope.event {
                        WorkerEvent::StreamHistoryBegin {
                            stream_id,
                            generation,
                        } => {
                            if let Some(state) = self.workspace.stream_states.get_mut(&stream_id) {
                                state.generation = generation;
                                state.history_loading = true;
                                state.history_total_records = 0;
                            }
                        }
                        WorkerEvent::StreamRecordsChunk {
                            stream_id,
                            generation,
                            records,
                            source: _source,
                        } => {
                            if self
                                .workspace
                                .stream_states
                                .get(&stream_id)
                                .is_some_and(|state| state.generation != generation)
                            {
                                continue;
                            }
                            debug!(
                                stream_id,
                                count = records.len(),
                                "received records from worker"
                            );
                            for record in &records {
                                self.insert_global_timestamp(record.timestamp_nanos);
                            }
                            self.append_lane_samples(stream_id, records.as_slice());

                            if let Some(state) = self.workspace.stream_states.get_mut(&stream_id) {
                                state.history_total_records =
                                    state.history_total_records.saturating_add(records.len());
                            }

                            if self.ui.follow_live {
                                if let Some(latest_record) = records.last().cloned() {
                                    let state =
                                        self.workspace.stream_states.entry(stream_id).or_default();
                                    state.current_record = Some(latest_record);
                                }
                                self.jump_latest_internal(false);
                            }
                        }
                        WorkerEvent::StreamHistoryEnd {
                            stream_id,
                            generation,
                            total_records,
                        } => {
                            if self
                                .workspace
                                .stream_states
                                .get(&stream_id)
                                .is_some_and(|state| state.generation != generation)
                            {
                                continue;
                            }
                            if let Some(state) = self.workspace.stream_states.get_mut(&stream_id) {
                                state.history_loading = false;
                                state.history_total_records = total_records;
                            }
                        }
                        WorkerEvent::SourceBound {
                            stream_id,
                            generation,
                            label,
                            binding,
                        } => {
                            info!(stream_id, %label, "worker bound source");
                            let state = self.workspace.stream_states.entry(stream_id).or_default();
                            state.generation = generation;
                            state.source_label = label;
                            state.current_record = None;
                            state.source_stats = None;
                            state.history_loading = true;
                            state.history_total_records = 0;
                            self.bind_stream_lane(stream_id, binding);
                            if let Some(anchor) = self.current_anchor_nanos() {
                                self.timeline.pending_scrub_anchor = Some(anchor);
                            }
                            self.ui.last_error = None;
                        }
                        WorkerEvent::AnchorRecord {
                            stream_id,
                            anchor_nanos,
                            record,
                        } => {
                            if self.current_anchor_nanos() == Some(anchor_nanos) {
                                let state =
                                    self.workspace.stream_states.entry(stream_id).or_default();
                                state.current_record = record;
                            }
                        }
                        WorkerEvent::DiscoveryPatch { op } => {
                            self.apply_discovery_patch(op);
                        }
                        WorkerEvent::DiscoverySnapshot {
                            publishers,
                            parameters,
                            sessions,
                        } => {
                            self.discovery.publishers = publishers;
                            self.discovery.parameters = parameters;
                            self.discovery.sessions = sessions;
                        }
                        WorkerEvent::ParameterValueLoaded {
                            target,
                            value_pretty,
                        } => {
                            for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                                if let ViewerTab::Parameters(panel) = tab {
                                    if panel.selected_parameter_reference.as_ref() == Some(&target)
                                    {
                                        panel.editor_text = value_pretty.clone();
                                        panel.status = Some(ParameterPanelStatus {
                                            success: true,
                                            message: "Parameter loaded".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                        WorkerEvent::ParameterWriteResult {
                            target,
                            success,
                            message,
                        } => {
                            for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                                if let ViewerTab::Parameters(panel) = tab {
                                    if panel.selected_parameter_reference.as_ref() == Some(&target)
                                    {
                                        panel.status = Some(ParameterPanelStatus {
                                            success,
                                            message: message.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        WorkerEvent::StreamStats { stream_id, source } => {
                            self.workspace
                                .stream_states
                                .entry(stream_id)
                                .or_default()
                                .source_stats = Some(*source);
                        }
                        WorkerEvent::BackendStats { backend } => {
                            self.ui.backend_stats = Some(*backend);
                        }
                        WorkerEvent::Error(message) => {
                            warn!(%message, "worker reported error");
                            self.ui.last_error = Some(message);
                        }
                        WorkerEvent::Ready => {
                            info!("worker is ready");
                            self.ui.ready = true;
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.ui.last_error = Some("worker disconnected".to_string());
                    warn!("worker event channel disconnected");
                    break;
                }
            }
        }

        self.ui.frame_processed_events = processed_events;
        self.ui.frame_processed_event_bytes = processed_event_bytes;
        if self.runtime.event_rx.is_empty() {
            self.runtime
                .worker_wake_armed
                .store(false, Ordering::SeqCst);
        }
    }

    fn apply_discovery_patch(&mut self, op: DiscoveryOp) {
        fn upsert_sorted<T: Ord>(items: &mut Vec<T>, item: T) {
            match items.binary_search(&item) {
                Ok(index) => items[index] = item,
                Err(index) => items.insert(index, item),
            }
        }

        fn remove_sorted<T: Ord>(items: &mut Vec<T>, item: &T) {
            if let Ok(index) = items.binary_search(item) {
                items.remove(index);
            }
        }

        match op {
            DiscoveryOp::PublisherUpsert(item) => {
                upsert_sorted(&mut self.discovery.publishers, item)
            }
            DiscoveryOp::PublisherRemove(item) => {
                remove_sorted(&mut self.discovery.publishers, &item)
            }
            DiscoveryOp::ParameterUpsert(item) => {
                upsert_sorted(&mut self.discovery.parameters, item)
            }
            DiscoveryOp::ParameterRemove(item) => {
                remove_sorted(&mut self.discovery.parameters, &item)
            }
            DiscoveryOp::SessionUpsert(item) => upsert_sorted(&mut self.discovery.sessions, item),
            DiscoveryOp::SessionRemove(item) => remove_sorted(&mut self.discovery.sessions, &item),
            DiscoveryOp::ResetNamespace(namespace) => {
                let _ = namespace;
                self.discovery.publishers.clear();
                self.discovery.parameters.clear();
                self.discovery.sessions.clear();
            }
        }
    }

    fn initiate_shutdown(&mut self) {
        if self.runtime.shutdown_started {
            return;
        }
        self.runtime.shutdown_started = true;
        info!("shutting down viewer app");

        self.send_command(WorkerCommand::Shutdown);
        self.run_pending_commands();
        self.runtime.cancellation_token.cancel();

        if let Some(worker_task) = self.runtime.worker_task.take() {
            let _ = self.runtime.runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(2), worker_task).await
            });
        }
        info!("viewer shutdown sequence completed");
    }
}

impl App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let frame_started = std::time::Instant::now();
        self.run_pending_commands();
        self.drain_worker_events();

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            panels::draw_controls(self, ui);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            panels::draw_status_bar(self, ui);
        });

        if self.shell.show_timeline {
            egui::TopBottomPanel::bottom("timeline_shell")
                .resizable(true)
                .show(ctx, |ui| {
                    panels::draw_timeline_panel(self, ui);
                });
        }

        if self.shell.show_discovery {
            egui::SidePanel::left("discovery_shell")
                .resizable(true)
                .show(ctx, |ui| {
                    panels::draw_discovery_panel(self, ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut dock_state = std::mem::replace(
                &mut self.workspace.dock_state,
                DockState::new(vec![ViewerTab::Text(TextPanelTab::new(
                    0,
                    self.config.source_expression.clone(),
                ))]),
            );
            let text_panel_count = dock_state
                .iter_all_tabs()
                .filter(|(_, tab)| matches!(tab, ViewerTab::Text(_)))
                .count();
            if sanitize_dock_splits(&mut dock_state) {
                warn!("sanitized dock split fractions before render");
            }
            let dock_render = catch_unwind(AssertUnwindSafe(|| {
                let mut tab_viewer = ViewerTabHost {
                    app: self,
                    text_panel_count,
                };
                DockArea::new(&mut dock_state)
                    .show_close_buttons(true)
                    .secondary_button_on_modifier(false)
                    .show_inside(ui, &mut tab_viewer);
            }));
            match dock_render {
                Ok(()) => {
                    self.workspace.dock_state = dock_state;
                }
                Err(_) => {
                    warn!("dock rendering panicked; resetting dock layout");
                    self.workspace.dock_state = initial_dock_state(
                        TextPanelTab::new(0, self.config.source_expression.clone()),
                        ParameterPanelTab::new(0),
                    );
                    self.ui.last_error =
                        Some("Dock layout was invalid and has been reset.".to_string());
                }
            }
        });

        self.reconcile_text_panels();
        self.maybe_emit_scrub_command();

        let frame_ms = frame_started.elapsed().as_secs_f32() * 1000.0;
        self.ui.frame_last_ms = frame_ms;
        self.ui.frame_ema_ms = if self.ui.frame_ema_ms <= f32::EPSILON {
            frame_ms
        } else {
            self.ui.frame_ema_ms * 0.9 + frame_ms * 0.1
        };

        let pending_events = !self.runtime.event_rx.is_empty();
        let pending_commands = !self.runtime.pending_commands.is_empty();
        if pending_events || pending_commands || (self.ui.follow_live && self.ui.ingest_enabled) {
            ctx.request_repaint_after(self.config.repaint_delay_on_activity);
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        save_persisted_state(self, storage);
    }
}

impl Drop for ViewerApp {
    fn drop(&mut self) {
        self.initiate_shutdown();
    }
}

fn private_node_from_expression(path_expression: &str) -> Option<&str> {
    let remainder = path_expression.strip_prefix('~')?;
    let (node, _) = remainder.split_once('/')?;
    if node.is_empty() {
        None
    } else {
        Some(node)
    }
}

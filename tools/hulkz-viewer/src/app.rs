mod bindings;
mod layout;
mod panel_catalog;
mod panels;
mod persistence;
mod state;
mod time_fmt;
mod timeline_state;

use std::time::Duration;
use std::{panic::catch_unwind, panic::AssertUnwindSafe};

use color_eyre::{eyre::WrapErr as _, Result};
use eframe::{egui, App, CreationContext, Frame, Storage};
use egui_dock::{DockArea, DockState};
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::{
    model::{WorkerCommand, WorkerEvent},
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
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let worker_task = runtime.spawn(run_worker(
            config.clone(),
            command_rx,
            event_tx,
            cancellation_token.clone(),
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
                event_rx,
                shutdown_started: false,
            },
            ui: UiState {
                ingest_enabled,
                follow_live,
                default_namespace_input,
                ready: false,
                last_error: None,
                backend_stats: None,
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
        if self.runtime.command_tx.send(command).is_err() {
            self.ui.last_error = Some("worker command channel is closed".to_string());
            warn!("failed to send worker command: channel closed");
        }
    }

    fn drain_worker_events(&mut self) {
        loop {
            match self.runtime.event_rx.try_recv() {
                Ok(WorkerEvent::RecordsAppended { stream_id, records }) => {
                    debug!(
                        stream_id,
                        count = records.len(),
                        "received records from worker"
                    );
                    for record in &records {
                        self.insert_global_timestamp(record.timestamp_nanos);
                    }
                    self.append_lane_samples(stream_id, records.as_slice());

                    if self.ui.follow_live {
                        if let Some(latest_record) = records.last().cloned() {
                            let state = self.workspace.stream_states.entry(stream_id).or_default();
                            state.current_record = Some(latest_record);
                        }
                        self.jump_latest_internal(false);
                    }
                }
                Ok(WorkerEvent::SourceBound {
                    stream_id,
                    label,
                    binding,
                }) => {
                    info!(stream_id, %label, "worker bound source");
                    let state = self.workspace.stream_states.entry(stream_id).or_default();
                    state.source_label = label;
                    state.current_record = None;
                    state.source_stats = None;
                    self.bind_stream_lane(stream_id, binding);
                    if let Some(anchor) = self.current_anchor_nanos() {
                        self.timeline.pending_scrub_anchor = Some(anchor);
                    }
                    self.ui.last_error = None;
                }
                Ok(WorkerEvent::AnchorRecord {
                    stream_id,
                    anchor_nanos,
                    record,
                }) => {
                    if self.current_anchor_nanos() == Some(anchor_nanos) {
                        let state = self.workspace.stream_states.entry(stream_id).or_default();
                        state.current_record = record;
                    }
                }
                Ok(WorkerEvent::DiscoverySnapshot {
                    publishers,
                    parameters,
                    sessions,
                }) => {
                    self.discovery.publishers = publishers;
                    self.discovery.parameters = parameters;
                    self.discovery.sessions = sessions;
                }
                Ok(WorkerEvent::ParameterValueLoaded {
                    target,
                    value_pretty,
                }) => {
                    for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                        if let ViewerTab::Parameters(panel) = tab {
                            if panel.selected_parameter_reference.as_ref() == Some(&target) {
                                panel.editor_text = value_pretty.clone();
                                panel.status = Some(ParameterPanelStatus {
                                    success: true,
                                    message: "Parameter loaded".to_string(),
                                });
                            }
                        }
                    }
                }
                Ok(WorkerEvent::ParameterWriteResult {
                    target,
                    success,
                    message,
                }) => {
                    for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                        if let ViewerTab::Parameters(panel) = tab {
                            if panel.selected_parameter_reference.as_ref() == Some(&target) {
                                panel.status = Some(ParameterPanelStatus {
                                    success,
                                    message: message.clone(),
                                });
                            }
                        }
                    }
                }
                Ok(WorkerEvent::StreamStats { stream_id, source }) => {
                    self.workspace
                        .stream_states
                        .entry(stream_id)
                        .or_default()
                        .source_stats = Some(*source);
                }
                Ok(WorkerEvent::BackendStats { backend }) => {
                    self.ui.backend_stats = Some(*backend);
                }
                Ok(WorkerEvent::Error(message)) => {
                    warn!(%message, "worker reported error");
                    self.ui.last_error = Some(message);
                }
                Ok(WorkerEvent::Ready) => {
                    info!("worker is ready");
                    self.ui.ready = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.ui.last_error = Some("worker disconnected".to_string());
                    warn!("worker event channel disconnected");
                    break;
                }
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
        ctx.request_repaint_after(Duration::from_millis(50));
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

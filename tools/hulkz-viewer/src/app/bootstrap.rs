use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use color_eyre::{eyre::WrapErr as _, Result};
use eframe::CreationContext;
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    protocol::{ViewerConfig, WorkerCommand, WorkerWakeNotifier},
    worker::run_worker,
};

use super::{
    layout::{
        apply_overrides_to_primary_text_panel, ensure_text_workspace_panel_exists,
        highest_parameter_panel_id, highest_stream_id, initial_dock_state, sanitize_dock_splits,
    },
    persistence::{load_persisted_dock_state, load_persisted_ui_state},
    state::{
        DiscoveryState, RuntimeState, ShellState, TimelineState, TimelineViewportState, UiState,
        ViewerApp, ViewerStartupOverrides, WorkspaceState, DEFAULT_TIMELINE_LANE_HEIGHT_PX,
    },
    workspace_panels::{ParametersWorkspacePanelState, TextWorkspacePanelState},
};

impl ViewerApp {
    pub fn new(
        creation_context: &CreationContext<'_>,
        startup_overrides: ViewerStartupOverrides,
    ) -> Result<Self> {
        info!("starting hulkz-viewer app runtime");
        let runtime = Runtime::new().wrap_err("failed to create tokio runtime")?;

        let mut config = ViewerConfig::default();
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

        let default_stream = TextWorkspacePanelState::new(0, config.source_expression.clone());
        let default_parameter_panel = ParametersWorkspacePanelState::new(0);
        let mut dock_state =
            load_persisted_dock_state(creation_context.storage).unwrap_or_else(|| {
                initial_dock_state(default_stream.clone(), default_parameter_panel.clone())
            });
        ensure_text_workspace_panel_exists(&mut dock_state, default_stream.clone());
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
        let wake_notifier = WorkerWakeNotifier::new({
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
}

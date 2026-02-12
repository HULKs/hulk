use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    hash::{Hash, Hasher},
    path::PathBuf,
    time::{Duration, Instant},
};

use chrono::{DateTime, Local, Utc};
mod layout;
mod panel_catalog;
mod panels;

use color_eyre::{eyre::WrapErr as _, Result};
use eframe::{egui, App, CreationContext, Frame, Storage};
use egui_dock::{DockArea, DockState};
use hulkz_stream::{BackendStats, PlaneKind, SourceStats};
use serde::{Deserialize, Serialize};
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::{
    model::{
        should_emit_scrub_command, DiscoveredParameter, DiscoveredPublisher, DiscoveredSession,
        DisplayedRecord, ParameterReference, SourceBindingInfo, SourceBindingRequest, StreamId,
        ViewerConfig, WorkerCommand, WorkerEvent,
    },
    timeline_canvas::timeline_lane_window_capacity,
    worker::run_worker,
};

use self::layout::{
    apply_overrides_to_primary_text_panel, ensure_stream_tab_exists, ensure_timeline_tab_exists,
    highest_parameter_panel_id, highest_stream_id, initial_dock_state, load_persisted_dock_state,
    load_persisted_ui_state, ViewerTabHost,
};
use self::panel_catalog::PanelKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
enum NamespaceSelection {
    #[default]
    FollowDefault,
    Override(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TextPanelTab {
    id: StreamId,
    #[serde(default)]
    namespace_selection: NamespaceSelection,
    source_expression: String,
}

impl TextPanelTab {
    fn new(id: StreamId, source_expression: String) -> Self {
        Self {
            id,
            namespace_selection: NamespaceSelection::FollowDefault,
            source_expression,
        }
    }

    fn follows_default_namespace(&self) -> bool {
        matches!(self.namespace_selection, NamespaceSelection::FollowDefault)
    }

    fn set_namespace_override_enabled(&mut self, enabled: bool, default_namespace: &str) {
        if enabled {
            let default_namespace = default_namespace.trim().to_string();
            self.namespace_selection = NamespaceSelection::Override(default_namespace);
        } else {
            self.namespace_selection = NamespaceSelection::FollowDefault;
        }
    }

    fn namespace_override_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.namespace_selection {
            NamespaceSelection::FollowDefault => None,
            NamespaceSelection::Override(value) => Some(value),
        }
    }

    fn effective_namespace(&self, default_namespace: &str) -> Option<String> {
        let raw = match &self.namespace_selection {
            NamespaceSelection::FollowDefault => default_namespace,
            NamespaceSelection::Override(value) => value,
        };
        let namespace = raw.trim();
        if namespace.is_empty() {
            None
        } else {
            Some(namespace.to_string())
        }
    }

    fn binding_request(&self, default_namespace: &str) -> Option<SourceBindingRequest> {
        let namespace = self.effective_namespace(default_namespace)?;
        let path_expression = self.source_expression.trim().to_string();
        if path_expression.is_empty() {
            return None;
        }
        Some(SourceBindingRequest {
            namespace,
            plane: PlaneKind::View,
            path_expression,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ParameterPanelTab {
    id: u64,
    #[serde(default)]
    namespace_selection: NamespaceSelection,
    node_input: String,
    path_input: String,
    editor_text: String,
    #[serde(skip)]
    selected_parameter_reference: Option<ParameterReference>,
    #[serde(default)]
    status: Option<ParameterPanelStatus>,
}

impl ParameterPanelTab {
    fn new(id: u64) -> Self {
        Self {
            id,
            namespace_selection: NamespaceSelection::FollowDefault,
            node_input: String::new(),
            path_input: String::new(),
            editor_text: String::new(),
            selected_parameter_reference: None,
            status: None,
        }
    }

    fn effective_namespace(&self, default_namespace: &str) -> Option<String> {
        let raw = match &self.namespace_selection {
            NamespaceSelection::FollowDefault => default_namespace,
            NamespaceSelection::Override(value) => value,
        };
        let namespace = raw.trim();
        if namespace.is_empty() {
            None
        } else {
            Some(namespace.to_string())
        }
    }

    fn set_selected_parameter_reference(&mut self, target: ParameterReference) {
        self.node_input = target.node.clone();
        self.path_input = target.path_expression.clone();
        self.selected_parameter_reference = Some(target);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ParameterPanelStatus {
    success: bool,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ViewerTab {
    Discovery,
    Timeline,
    #[serde(alias = "TextStreamPanel")]
    Text(TextPanelTab),
    Parameters(ParameterPanelTab),
}

impl ViewerTab {
    fn kind(&self) -> PanelKind {
        match self {
            ViewerTab::Discovery => PanelKind::Discovery,
            ViewerTab::Timeline => PanelKind::Timeline,
            ViewerTab::Text(_) => PanelKind::Text,
            ViewerTab::Parameters(_) => PanelKind::Parameters,
        }
    }

    fn title_label(&self) -> &'static str {
        self.kind().label()
    }

    fn dock_id(&self) -> egui::Id {
        match self {
            ViewerTab::Discovery => egui::Id::new("viewer_tab_discovery"),
            ViewerTab::Timeline => egui::Id::new("viewer_tab_timeline"),
            ViewerTab::Text(stream) => egui::Id::new(("viewer_tab_text", stream.id)),
            ViewerTab::Parameters(panel) => egui::Id::new(("viewer_tab_parameters", panel.id)),
        }
    }

    fn is_closeable(&self, text_panel_count: usize) -> bool {
        if self.kind().is_fixed() {
            return false;
        }
        match self {
            ViewerTab::Text(_) => text_panel_count > 1,
            ViewerTab::Parameters(_) => true,
            ViewerTab::Discovery | ViewerTab::Timeline => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedUiState {
    ingest_enabled: bool,
    follow_live: bool,
    next_stream_id: StreamId,
    #[serde(default = "default_next_parameter_panel_id")]
    next_parameter_panel_id: u64,
    #[serde(default = "default_namespace_value")]
    default_namespace: String,
}

#[derive(Debug, Default)]
struct StreamRuntimeState {
    source_label: String,
    current_record: Option<DisplayedRecord>,
    source_stats: Option<SourceStats>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TimelineViewportState {
    pub(crate) span: Option<Duration>,
    pub(crate) pan_offset_ns: i64,
    pub(crate) lane_scroll_offset: f32,
    pub(crate) lane_height_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimelineRenderRange {
    pub(crate) start_ns: u64,
    pub(crate) end_ns: u64,
}

impl TimelineRenderRange {
    pub(crate) fn span_nanos(self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    pub(crate) fn span(self) -> Duration {
        Duration::from_nanos(self.span_nanos())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TimelineLaneKey {
    pub(crate) namespace: String,
    pub(crate) path_expression: String,
}

#[derive(Debug, Clone)]
struct TimelineLaneState {
    key: TimelineLaneKey,
    sample_timestamps: VecDeque<u64>,
    last_seen_ns: u64,
    active_bindings: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct LaneRenderPoint {
    pub(crate) timestamp_ns: u64,
    pub(crate) count: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct LaneRenderRow {
    pub(crate) key: TimelineLaneKey,
    pub(crate) label: String,
    pub(crate) points: Vec<LaneRenderPoint>,
    pub(crate) color_index: usize,
    pub(crate) active_bindings: usize,
}

const STORAGE_KEY_DOCK_STATE: &str = "hulkz_viewer/dock_state_v3";
const STORAGE_KEY_UI_STATE: &str = "hulkz_viewer/ui_state_v2";
const MIN_TIMELINE_SPAN: Duration = Duration::from_millis(50);
const DEFAULT_TIMELINE_LANE_HEIGHT_PX: f32 = 22.0;

pub struct ViewerApp {
    dock_state: DockState<ViewerTab>,
    stream_states: BTreeMap<StreamId, StreamRuntimeState>,
    binding_cache: BTreeMap<StreamId, Option<SourceBindingRequest>>,
    config: ViewerConfig,
    command_tx: mpsc::UnboundedSender<WorkerCommand>,
    event_rx: mpsc::UnboundedReceiver<WorkerEvent>,
    runtime: Runtime,
    worker_task: Option<tokio::task::JoinHandle<()>>,
    cancellation_token: CancellationToken,
    ingest_enabled: bool,
    default_namespace_input: String,
    follow_live: bool,
    global_timeline: Vec<u64>,
    global_timeline_index: Option<usize>,
    timeline_hover_preview: Option<u64>,
    timeline_viewport: TimelineViewportState,
    stream_lane_bindings: BTreeMap<StreamId, TimelineLaneKey>,
    timeline_lanes: BTreeMap<TimelineLaneKey, TimelineLaneState>,
    pending_scrub_anchor: Option<u64>,
    last_scrub_emitted: Instant,
    next_stream_id: StreamId,
    next_parameter_panel_id: u64,
    last_error: Option<String>,
    backend_stats: Option<BackendStats>,
    ready: bool,
    shutdown_started: bool,
    discovered_publishers: Vec<DiscoveredPublisher>,
    discovered_parameters: Vec<DiscoveredParameter>,
    discovered_sessions: Vec<DiscoveredSession>,
}

#[derive(Debug, Clone, Default)]
pub struct ViewerStartupOverrides {
    pub namespace: Option<String>,
    pub source_expression: Option<String>,
    pub storage_path: Option<PathBuf>,
}

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

        let default_stream = TextPanelTab::new(0, config.source_expression.clone());
        let default_parameter_panel = ParameterPanelTab::new(0);
        let mut dock_state =
            load_persisted_dock_state(creation_context.storage).unwrap_or_else(|| {
                initial_dock_state(default_stream.clone(), default_parameter_panel.clone())
            });
        ensure_stream_tab_exists(&mut dock_state, default_stream.clone());
        ensure_timeline_tab_exists(&mut dock_state);

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

        let now = Instant::now();
        let last_scrub_emitted = now.checked_sub(config.scrub_debounce).unwrap_or(now);

        let mut app = Self {
            dock_state,
            stream_states: BTreeMap::new(),
            binding_cache: BTreeMap::new(),
            config,
            command_tx,
            event_rx,
            runtime,
            worker_task: Some(worker_task),
            cancellation_token,
            ingest_enabled,
            default_namespace_input,
            follow_live,
            global_timeline: Vec::new(),
            global_timeline_index: None,
            timeline_hover_preview: None,
            timeline_viewport: TimelineViewportState {
                lane_height_px: DEFAULT_TIMELINE_LANE_HEIGHT_PX,
                ..TimelineViewportState::default()
            },
            stream_lane_bindings: BTreeMap::new(),
            timeline_lanes: BTreeMap::new(),
            pending_scrub_anchor: None,
            last_scrub_emitted,
            next_stream_id,
            next_parameter_panel_id,
            last_error: None,
            backend_stats: None,
            ready: false,
            shutdown_started: false,
            discovered_publishers: Vec::new(),
            discovered_parameters: Vec::new(),
            discovered_sessions: Vec::new(),
        };

        app.reconcile_text_panels();
        app.update_discovery_namespace();

        if !app.ingest_enabled {
            app.send_command(WorkerCommand::SetIngestEnabled(false));
        }

        Ok(app)
    }

    fn send_command(&mut self, command: WorkerCommand) {
        debug!(?command, "sending worker command");
        if self.command_tx.send(command).is_err() {
            self.last_error = Some("worker command channel is closed".to_string());
            warn!("failed to send worker command: channel closed");
        }
    }

    fn update_discovery_namespace(&mut self) {
        self.send_command(WorkerCommand::SetDiscoveryNamespace(
            self.default_namespace_input.trim().to_string(),
        ));
    }

    fn apply_panel_binding(&mut self, panel: &TextPanelTab) {
        let stream_id = panel.id;
        if let Some(request) = panel.binding_request(&self.default_namespace_input) {
            self.send_command(WorkerCommand::BindStream { stream_id, request });
        } else {
            self.send_command(WorkerCommand::RemoveStream { stream_id });
            if let Some(state) = self.stream_states.get_mut(&stream_id) {
                state.source_label = "unbound (set namespace/path)".to_string();
            }
        }
    }

    fn reconcile_text_panels(&mut self) {
        let panels = self.text_panels();
        let panel_ids = panels.iter().map(|panel| panel.id).collect::<BTreeSet<_>>();

        for stream_id in self
            .stream_states
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|stream_id| !panel_ids.contains(stream_id))
            .collect::<Vec<_>>()
        {
            self.stream_states.remove(&stream_id);
        }
        for (stream_id, previous_request) in self
            .binding_cache
            .clone()
            .into_iter()
            .filter(|(stream_id, _)| !panel_ids.contains(stream_id))
            .collect::<Vec<_>>()
        {
            if previous_request.is_some() {
                self.send_command(WorkerCommand::RemoveStream { stream_id });
            }
            self.unbind_stream_lane(stream_id);
            self.binding_cache.remove(&stream_id);
        }

        for panel in panels {
            self.stream_states
                .entry(panel.id)
                .or_insert_with(|| StreamRuntimeState {
                    source_label: "unbound".to_string(),
                    ..StreamRuntimeState::default()
                });

            let desired_request = panel.binding_request(&self.default_namespace_input);
            let previous_request = self.binding_cache.get(&panel.id);
            if previous_request != Some(&desired_request) {
                self.apply_panel_binding(&panel);
                if desired_request.is_none() {
                    self.unbind_stream_lane(panel.id);
                }
                self.binding_cache.insert(panel.id, desired_request);
            }
        }

        self.evict_inactive_lanes_if_needed();
    }

    fn text_panels(&self) -> Vec<TextPanelTab> {
        self.dock_state
            .iter_all_tabs()
            .filter_map(|(_, tab)| match tab {
                ViewerTab::Text(stream) => Some(stream.clone()),
                _ => None,
            })
            .collect()
    }

    fn drain_worker_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
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

                    if self.follow_live {
                        if let Some(latest_record) = records.last().cloned() {
                            let state = self.stream_states.entry(stream_id).or_default();
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
                    let state = self.stream_states.entry(stream_id).or_default();
                    state.source_label = label;
                    state.current_record = None;
                    state.source_stats = None;
                    self.bind_stream_lane(stream_id, binding);
                    if let Some(anchor) = self.current_anchor_nanos() {
                        self.pending_scrub_anchor = Some(anchor);
                    }
                    self.last_error = None;
                }
                Ok(WorkerEvent::AnchorRecord {
                    stream_id,
                    anchor_nanos,
                    record,
                }) => {
                    if self.current_anchor_nanos() == Some(anchor_nanos) {
                        let state = self.stream_states.entry(stream_id).or_default();
                        state.current_record = record;
                    }
                }
                Ok(WorkerEvent::DiscoverySnapshot {
                    publishers,
                    parameters,
                    sessions,
                }) => {
                    self.discovered_publishers = publishers;
                    self.discovered_parameters = parameters;
                    self.discovered_sessions = sessions;
                }
                Ok(WorkerEvent::ParameterValueLoaded {
                    target,
                    value_pretty,
                }) => {
                    for (_, tab) in self.dock_state.iter_all_tabs_mut() {
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
                    for (_, tab) in self.dock_state.iter_all_tabs_mut() {
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
                    self.stream_states
                        .entry(stream_id)
                        .or_default()
                        .source_stats = Some(*source);
                }
                Ok(WorkerEvent::BackendStats { backend }) => {
                    self.backend_stats = Some(*backend);
                }
                Ok(WorkerEvent::Error(message)) => {
                    warn!(%message, "worker reported error");
                    self.last_error = Some(message);
                }
                Ok(WorkerEvent::Ready) => {
                    info!("worker is ready");
                    self.ready = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.last_error = Some("worker disconnected".to_string());
                    warn!("worker event channel disconnected");
                    break;
                }
            }
        }
    }

    fn current_anchor_nanos(&self) -> Option<u64> {
        self.global_timeline_index
            .and_then(|index| self.global_timeline.get(index).copied())
    }

    fn timeline_full_range(&self) -> Option<TimelineRenderRange> {
        let start_ns = self.global_timeline.first().copied()?;
        let end_ns = self.global_timeline.last().copied()?;
        Some(TimelineRenderRange { start_ns, end_ns })
    }

    fn timeline_render_range(&self) -> Option<TimelineRenderRange> {
        let full_range = self.timeline_full_range()?;
        Some(derive_timeline_render_range(
            full_range,
            self.timeline_viewport,
            self.follow_live,
        ))
    }

    fn insert_global_timestamp(&mut self, timestamp_nanos: u64) {
        match self.global_timeline.binary_search(&timestamp_nanos) {
            Ok(_) => {}
            Err(index) => {
                self.global_timeline.insert(index, timestamp_nanos);
                if let Some(current_index) = self.global_timeline_index {
                    if index <= current_index {
                        self.global_timeline_index = Some(current_index.saturating_add(1));
                    }
                }
            }
        }
        trim_timeline_to_capacity(
            &mut self.global_timeline,
            &mut self.global_timeline_index,
            self.config.max_timeline_points,
        );
    }

    fn jump_latest_internal(&mut self, queue_scrub: bool) {
        if self.global_timeline.is_empty() {
            self.global_timeline_index = None;
            return;
        }

        self.timeline_viewport.pan_offset_ns = 0;
        let latest_index = self.global_timeline.len().saturating_sub(1);
        self.global_timeline_index = Some(latest_index);
        let latest_anchor = self.global_timeline[latest_index];
        if queue_scrub {
            self.pending_scrub_anchor = Some(latest_anchor);
        }
    }

    fn set_global_timeline_anchor_by_timestamp(&mut self, timestamp_ns: u64, queue_scrub: bool) {
        if self.global_timeline.is_empty() {
            self.global_timeline_index = None;
            return;
        }

        let index = nearest_timestamp_index(&self.global_timeline, timestamp_ns)
            .unwrap_or_else(|| self.global_timeline.len().saturating_sub(1));
        self.set_global_timeline_index(index, queue_scrub);
    }

    fn set_global_timeline_index(&mut self, index: usize, queue_scrub: bool) {
        if self.global_timeline.is_empty() {
            self.global_timeline_index = None;
            return;
        }
        let clamped = index.min(self.global_timeline.len().saturating_sub(1));
        self.global_timeline_index = Some(clamped);
        if queue_scrub {
            self.pending_scrub_anchor = self.global_timeline.get(clamped).copied();
        }
    }

    fn mark_manual_timeline_navigation(&mut self) {
        self.follow_live = false;
    }

    fn set_timeline_hover_preview(&mut self, timestamp_ns: Option<u64>) {
        self.timeline_hover_preview = timestamp_ns;
    }

    fn apply_timeline_zoom(&mut self, zoom_factor: f32, focus_timestamp_ns: u64) {
        let Some(full_range) = self.timeline_full_range() else {
            return;
        };
        let Some(current_range) = self.timeline_render_range() else {
            return;
        };
        let Some(latest_ns) = self.global_timeline.last().copied() else {
            return;
        };

        let next_range = zoom_range_around_focus(
            full_range,
            current_range,
            focus_timestamp_ns,
            zoom_factor,
            MIN_TIMELINE_SPAN,
        );
        let lane_scroll_offset = self.timeline_viewport.lane_scroll_offset;
        let lane_height_px = self.timeline_viewport.lane_height_px;
        self.timeline_viewport = viewport_state_from_range(next_range, latest_ns, full_range);
        self.timeline_viewport.lane_scroll_offset = lane_scroll_offset;
        self.timeline_viewport.lane_height_px = lane_height_px;
    }

    fn apply_timeline_pan_fraction(&mut self, pan_delta_fraction: f32) {
        let Some(full_range) = self.timeline_full_range() else {
            return;
        };
        let Some(current_range) = self.timeline_render_range() else {
            return;
        };
        let span_ns = current_range.span_nanos();
        if span_ns == 0 {
            return;
        }

        let delta_ns = (pan_delta_fraction as f64 * span_ns as f64).round() as i64;
        self.timeline_viewport.pan_offset_ns = self
            .timeline_viewport
            .pan_offset_ns
            .saturating_add(delta_ns);

        let Some(latest_ns) = self.global_timeline.last().copied() else {
            return;
        };
        let clamped_range =
            derive_timeline_render_range(full_range, self.timeline_viewport, self.follow_live);
        let lane_scroll_offset = self.timeline_viewport.lane_scroll_offset;
        let lane_height_px = self.timeline_viewport.lane_height_px;
        self.timeline_viewport = viewport_state_from_range(clamped_range, latest_ns, full_range);
        self.timeline_viewport.lane_scroll_offset = lane_scroll_offset;
        self.timeline_viewport.lane_height_px = lane_height_px;
    }

    fn apply_timeline_lane_scroll(&mut self, lane_delta: f32, total_lanes: usize) {
        let visible_lanes = timeline_visible_lane_count(self.timeline_viewport.lane_height_px);
        if total_lanes == 0 || total_lanes <= visible_lanes {
            self.timeline_viewport.lane_scroll_offset = 0.0;
            return;
        }
        let lane_height = self.timeline_viewport.lane_height_px.max(12.0);
        let max_offset = total_lanes.saturating_sub(visible_lanes) as f32;
        let scaled_delta = lane_delta / lane_height;
        self.timeline_viewport.lane_scroll_offset =
            (self.timeline_viewport.lane_scroll_offset + scaled_delta).clamp(0.0, max_offset);
    }

    fn lane_label_for_key(&self, key: &TimelineLaneKey) -> String {
        let default_namespace = self.default_namespace_input.trim();
        if default_namespace.is_empty() || default_namespace == key.namespace {
            key.path_expression.clone()
        } else {
            format!("{} @ {}", key.path_expression, key.namespace)
        }
    }

    fn bind_stream_lane(&mut self, stream_id: StreamId, binding: SourceBindingInfo) {
        let key = TimelineLaneKey {
            namespace: binding.namespace,
            path_expression: binding.path_expression,
        };

        if let Some(previous_key) = self.stream_lane_bindings.get(&stream_id).cloned() {
            if previous_key != key {
                self.decrement_lane_binding(&previous_key);
            }
        }
        self.stream_lane_bindings.insert(stream_id, key.clone());

        let lane = self
            .timeline_lanes
            .entry(key.clone())
            .or_insert_with(|| TimelineLaneState {
                key,
                sample_timestamps: VecDeque::new(),
                last_seen_ns: 0,
                active_bindings: 0,
            });
        lane.active_bindings = lane.active_bindings.saturating_add(1);
        self.evict_inactive_lanes_if_needed();
    }

    fn decrement_lane_binding(&mut self, lane_key: &TimelineLaneKey) {
        let mut should_remove = false;
        if let Some(lane) = self.timeline_lanes.get_mut(lane_key) {
            lane.active_bindings = lane.active_bindings.saturating_sub(1);
            should_remove = lane.active_bindings == 0 && lane.sample_timestamps.is_empty();
        }
        if should_remove {
            self.timeline_lanes.remove(lane_key);
        }
    }

    fn unbind_stream_lane(&mut self, stream_id: StreamId) {
        if let Some(key) = self.stream_lane_bindings.remove(&stream_id) {
            self.decrement_lane_binding(&key);
        }
    }

    fn append_lane_samples(&mut self, stream_id: StreamId, records: &[DisplayedRecord]) {
        let Some(lane_key) = self.stream_lane_bindings.get(&stream_id).cloned() else {
            return;
        };
        let lane = self
            .timeline_lanes
            .entry(lane_key.clone())
            .or_insert_with(|| TimelineLaneState {
                key: lane_key,
                sample_timestamps: VecDeque::new(),
                last_seen_ns: 0,
                active_bindings: 0,
            });

        for record in records {
            if lane.sample_timestamps.back().copied() != Some(record.timestamp_nanos) {
                lane.sample_timestamps.push_back(record.timestamp_nanos);
            }
            lane.last_seen_ns = lane.last_seen_ns.max(record.timestamp_nanos);
        }
        while lane.sample_timestamps.len() > self.config.max_samples_per_lane {
            let _ = lane.sample_timestamps.pop_front();
        }
        self.evict_inactive_lanes_if_needed();
    }

    fn evict_inactive_lanes_if_needed(&mut self) {
        if self.timeline_lanes.len() <= self.config.max_retained_lanes {
            return;
        }

        let mut candidates = self
            .timeline_lanes
            .iter()
            .filter(|(_, lane)| lane.active_bindings == 0)
            .map(|(key, lane)| (key.clone(), lane.last_seen_ns))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(_, last_seen_ns)| *last_seen_ns);

        let mut over = self
            .timeline_lanes
            .len()
            .saturating_sub(self.config.max_retained_lanes);
        for (key, _) in candidates {
            if over == 0 {
                break;
            }
            self.timeline_lanes.remove(&key);
            over = over.saturating_sub(1);
        }
    }

    fn timeline_lane_rows(
        &self,
        viewport_range: TimelineRenderRange,
        lane_window_start: usize,
        lane_window_count: usize,
        pixel_width: f32,
    ) -> (Vec<LaneRenderRow>, usize) {
        let mut lanes = self.timeline_lanes.values().collect::<Vec<_>>();
        lanes.sort_by(|left, right| {
            left.key
                .path_expression
                .cmp(&right.key.path_expression)
                .then_with(|| left.key.namespace.cmp(&right.key.namespace))
        });

        let total = lanes.len();
        if total == 0 || lane_window_count == 0 {
            return (Vec::new(), total);
        }
        let start = lane_window_start.min(total.saturating_sub(1));
        let end = (start + lane_window_count).min(total);
        let slot_count = ((pixel_width.max(80.0) / 8.0).round() as usize).clamp(32, 1024);

        let mut rows = Vec::with_capacity(end.saturating_sub(start));
        for lane in &lanes[start..end] {
            let clustered = cluster_lane_samples(
                &lane.sample_timestamps,
                viewport_range.start_ns,
                viewport_range.end_ns,
                slot_count,
            );
            let points = merge_lane_points_by_pixel_distance(
                clustered,
                viewport_range,
                pixel_width.max(64.0),
                5.0,
            );
            rows.push(LaneRenderRow {
                key: lane.key.clone(),
                label: self.lane_label_for_key(&lane.key),
                points,
                color_index: lane_color_index(&lane.key),
                active_bindings: lane.active_bindings,
            });
        }

        (rows, total)
    }

    fn maybe_emit_scrub_command(&mut self) {
        let Some(anchor_nanos) = self.pending_scrub_anchor else {
            return;
        };

        let now = Instant::now();
        if should_emit_scrub_command(self.last_scrub_emitted, now, self.config.scrub_debounce) {
            for stream_id in self.stream_states.keys().copied().collect::<Vec<_>>() {
                self.send_command(WorkerCommand::SetScrubAnchor {
                    stream_id,
                    anchor_nanos,
                });
            }
            self.last_scrub_emitted = now;
            self.pending_scrub_anchor = None;
        }
    }

    fn create_text_stream_panel(&mut self) {
        self.open_text_panel(
            NamespaceSelection::FollowDefault,
            self.config.source_expression.clone(),
        );
    }

    fn has_panel_kind(&self, kind: PanelKind) -> bool {
        self.dock_state
            .iter_all_tabs()
            .any(|(_, tab)| tab.kind() == kind)
    }

    fn open_panel_kind(&mut self, kind: PanelKind) {
        if kind.is_singleton() && self.has_panel_kind(kind) {
            return;
        }
        match kind {
            PanelKind::Text => self.create_text_stream_panel(),
            PanelKind::Parameters => self.create_parameter_panel(),
            PanelKind::Discovery => self.dock_state.push_to_focused_leaf(ViewerTab::Discovery),
            PanelKind::Timeline => self.dock_state.push_to_focused_leaf(ViewerTab::Timeline),
        }
    }

    fn open_text_panel(
        &mut self,
        namespace_selection: NamespaceSelection,
        path_expression: String,
    ) {
        let stream_id = self.next_stream_id;
        self.next_stream_id = self.next_stream_id.saturating_add(1);

        let mut panel = TextPanelTab::new(stream_id, path_expression);
        panel.namespace_selection = namespace_selection;

        self.dock_state
            .push_to_focused_leaf(ViewerTab::Text(panel.clone()));
        self.stream_states.insert(
            stream_id,
            StreamRuntimeState {
                source_label: "unbound".to_string(),
                ..StreamRuntimeState::default()
            },
        );
    }

    fn create_parameter_panel(&mut self) {
        let panel_id = self.next_parameter_panel_id;
        self.next_parameter_panel_id = self.next_parameter_panel_id.saturating_add(1);
        self.dock_state
            .push_to_focused_leaf(ViewerTab::Parameters(ParameterPanelTab::new(panel_id)));
    }

    fn set_default_namespace(&mut self) {
        let namespace = self.default_namespace_input.trim().to_string();
        self.default_namespace_input = namespace;
        self.update_discovery_namespace();
    }

    fn parameter_node_candidates(&self, panel: &ParameterPanelTab) -> Vec<String> {
        let Some(namespace) = panel.effective_namespace(&self.default_namespace_input) else {
            return Vec::new();
        };
        let namespace_filter = namespace.as_str();
        let path_filter = panel.path_input.trim();
        let mut candidates = BTreeSet::new();
        for parameter in &self.discovered_parameters {
            if parameter.namespace != namespace_filter {
                continue;
            }
            if !path_filter.is_empty() && parameter.path_expression != path_filter {
                continue;
            }
            let node = parameter.node.trim();
            if !node.is_empty() {
                candidates.insert(node.to_string());
            }
        }
        candidates.into_iter().collect()
    }

    fn parameter_path_candidates(&self, panel: &ParameterPanelTab) -> Vec<String> {
        let Some(namespace) = panel.effective_namespace(&self.default_namespace_input) else {
            return Vec::new();
        };
        let namespace_filter = namespace.as_str();
        let node_filter = panel.node_input.trim();
        let mut candidates = BTreeSet::new();
        for parameter in &self.discovered_parameters {
            if parameter.namespace != namespace_filter {
                continue;
            }
            if !node_filter.is_empty() && parameter.node != node_filter {
                continue;
            }
            let path = parameter.path_expression.trim();
            if !path.is_empty() {
                candidates.insert(path.to_string());
            }
        }
        candidates.into_iter().collect()
    }

    fn parameter_reference_from_inputs(
        &self,
        panel: &ParameterPanelTab,
    ) -> Result<ParameterReference, String> {
        let Some(namespace) = panel.effective_namespace(&self.default_namespace_input) else {
            return Err("Set a default namespace first.".to_string());
        };
        let namespace_filter = namespace.as_str();

        let path_expression = panel.path_input.trim();
        if path_expression.is_empty() {
            return Err("Enter a parameter path.".to_string());
        }

        let mut node = panel.node_input.trim().to_string();
        if node.is_empty() {
            if let Some(private_node) = private_node_from_expression(path_expression) {
                node = private_node.to_string();
            } else {
                let mut nodes = self
                    .discovered_parameters
                    .iter()
                    .filter(|parameter| {
                        parameter.namespace == namespace_filter
                            && parameter.path_expression.trim() == path_expression
                    })
                    .map(|parameter| parameter.node.trim())
                    .filter(|node| !node.is_empty())
                    .collect::<BTreeSet<_>>();
                if nodes.len() == 1 {
                    node = nodes.pop_first().unwrap_or_default().to_string();
                } else if nodes.len() > 1 {
                    return Err("Parameter exists on multiple nodes. Pick a node.".to_string());
                }
            }
        }

        if node.is_empty() {
            return Err("Enter a node (or use ~node/path).".to_string());
        }

        Ok(ParameterReference {
            namespace,
            node,
            path_expression: path_expression.to_string(),
        })
    }

    fn source_path_candidates(&self, stream: &TextPanelTab) -> Vec<String> {
        let effective_namespace = stream.effective_namespace(&self.default_namespace_input);

        let mut candidates = BTreeSet::new();
        for publisher in &self.discovered_publishers {
            if let Some(namespace) = effective_namespace.as_deref() {
                if publisher.namespace != namespace {
                    continue;
                }
            }
            let path = publisher.path_expression.trim();
            if !path.is_empty() {
                candidates.insert(path.to_string());
            }
        }
        candidates.into_iter().collect()
    }

    fn namespace_candidates(&self) -> Vec<String> {
        self.discovered_sessions
            .iter()
            .map(|session| session.namespace.trim())
            .filter(|namespace| !namespace.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn initiate_shutdown(&mut self) {
        if self.shutdown_started {
            return;
        }
        self.shutdown_started = true;
        info!("shutting down viewer app");

        self.send_command(WorkerCommand::Shutdown);
        self.cancellation_token.cancel();

        if let Some(worker_task) = self.worker_task.take() {
            let _ = self.runtime.block_on(async {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut dock_state = std::mem::replace(
                &mut self.dock_state,
                DockState::new(vec![ViewerTab::Text(TextPanelTab::new(
                    0,
                    self.config.source_expression.clone(),
                ))]),
            );
            let text_panel_count = dock_state
                .iter_all_tabs()
                .filter(|(_, tab)| matches!(tab, ViewerTab::Text(_)))
                .count();
            let mut tab_viewer = ViewerTabHost {
                app: self,
                text_panel_count,
            };
            DockArea::new(&mut dock_state)
                .show_close_buttons(true)
                .secondary_button_on_modifier(false)
                .show_inside(ui, &mut tab_viewer);
            self.dock_state = dock_state;
        });

        self.reconcile_text_panels();
        self.maybe_emit_scrub_command();
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        match serde_json::to_string(&self.dock_state) {
            Ok(json) => storage.set_string(STORAGE_KEY_DOCK_STATE, json),
            Err(error) => {
                warn!(?error, "failed to serialize dock state");
                return;
            }
        }

        let ui_state = PersistedUiState {
            ingest_enabled: self.ingest_enabled,
            follow_live: self.follow_live,
            next_stream_id: self.next_stream_id,
            next_parameter_panel_id: self.next_parameter_panel_id,
            default_namespace: self.default_namespace_input.clone(),
        };

        match serde_json::to_string(&ui_state) {
            Ok(json) => storage.set_string(STORAGE_KEY_UI_STATE, json),
            Err(error) => warn!(?error, "failed to serialize ui state"),
        }
    }
}

impl Drop for ViewerApp {
    fn drop(&mut self) {
        self.initiate_shutdown();
    }
}

fn format_timestamp(nanos: u64) -> String {
    let secs = nanos / 1_000_000_000;
    let subsec_nanos = (nanos % 1_000_000_000) as u32;
    let Ok(secs_i64) = i64::try_from(secs) else {
        return format!("{nanos} ns");
    };
    let Some(utc) = DateTime::<Utc>::from_timestamp(secs_i64, subsec_nanos) else {
        return format!("{nanos} ns");
    };
    utc.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S%.3f")
        .to_string()
}

fn nearest_timestamp_index(timeline: &[u64], target_ns: u64) -> Option<usize> {
    if timeline.is_empty() {
        return None;
    }
    Some(match timeline.binary_search(&target_ns) {
        Ok(index) => index,
        Err(0) => 0,
        Err(index) if index >= timeline.len() => timeline.len().saturating_sub(1),
        Err(index) => {
            let prev = index.saturating_sub(1);
            let prev_delta = target_ns.saturating_sub(timeline[prev]);
            let next_delta = timeline[index].saturating_sub(target_ns);
            if prev_delta <= next_delta {
                prev
            } else {
                index
            }
        }
    })
}

fn derive_timeline_render_range(
    full_range: TimelineRenderRange,
    viewport: TimelineViewportState,
    follow_live: bool,
) -> TimelineRenderRange {
    let full_span = full_range.span();
    let Some(span) = viewport
        .span
        .map(|span| clamp_timeline_span(span, full_span))
    else {
        return full_range;
    };
    let span_ns = duration_to_nanos(span);

    if follow_live {
        let start_ns = full_range.end_ns.saturating_sub(span_ns);
        return TimelineRenderRange {
            start_ns,
            end_ns: full_range.end_ns,
        };
    }

    let latest_i128 = i128::from(full_range.end_ns);
    let span_i128 = i128::from(span_ns);
    let candidate_end = latest_i128 + i128::from(viewport.pan_offset_ns);
    let candidate_start = candidate_end - span_i128;
    clamp_timeline_range(full_range, candidate_start, span_ns)
}

fn clamp_timeline_span(requested_span: Duration, full_span: Duration) -> Duration {
    if full_span.is_zero() {
        return Duration::ZERO;
    }
    requested_span
        .max(MIN_TIMELINE_SPAN.min(full_span))
        .min(full_span)
}

fn clamp_timeline_range(
    full_range: TimelineRenderRange,
    start_candidate_ns: i128,
    span_ns: u64,
) -> TimelineRenderRange {
    let full_span_ns = full_range.span_nanos();
    if span_ns >= full_span_ns {
        return full_range;
    }

    let min_start = i128::from(full_range.start_ns);
    let max_start = i128::from(full_range.end_ns.saturating_sub(span_ns));
    let start_ns = start_candidate_ns.clamp(min_start, max_start);
    let start_ns = u64::try_from(start_ns).unwrap_or(full_range.start_ns);
    TimelineRenderRange {
        start_ns,
        end_ns: start_ns.saturating_add(span_ns),
    }
}

fn zoom_range_around_focus(
    full_range: TimelineRenderRange,
    current_range: TimelineRenderRange,
    focus_ns: u64,
    zoom_factor: f32,
    min_span: Duration,
) -> TimelineRenderRange {
    let current_span_ns = current_range.span_nanos();
    let full_span_ns = full_range.span_nanos();
    if current_span_ns == 0 || full_span_ns == 0 {
        return full_range;
    }

    let clamped_factor = zoom_factor.clamp(0.1, 10.0) as f64;
    let desired_span = ((current_span_ns as f64) * clamped_factor).round() as u64;
    let min_span_ns = duration_to_nanos(min_span);
    let target_span = desired_span
        .max(min_span_ns.min(full_span_ns))
        .min(full_span_ns);
    if target_span == full_span_ns {
        return full_range;
    }

    let focus_ns = focus_ns.clamp(current_range.start_ns, current_range.end_ns);
    let relative = (focus_ns.saturating_sub(current_range.start_ns) as f64
        / current_span_ns.max(1) as f64)
        .clamp(0.0, 1.0);
    let start_candidate = i128::from(focus_ns) - ((target_span as f64 * relative).round() as i128);
    clamp_timeline_range(full_range, start_candidate, target_span)
}

fn viewport_state_from_range(
    range: TimelineRenderRange,
    latest_ns: u64,
    full_range: TimelineRenderRange,
) -> TimelineViewportState {
    if range == full_range {
        return TimelineViewportState::default();
    }
    let span = range.span();
    let pan_offset_i128 = i128::from(range.end_ns) - i128::from(latest_ns);
    let pan_offset_ns = pan_offset_i128.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    TimelineViewportState {
        span: Some(span),
        pan_offset_ns,
        lane_scroll_offset: 0.0,
        lane_height_px: DEFAULT_TIMELINE_LANE_HEIGHT_PX,
    }
}

fn timeline_visible_lane_count(lane_height_px: f32) -> usize {
    timeline_lane_window_capacity(lane_height_px)
}

fn lane_color_index(key: &TimelineLaneKey) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish() as usize
}

fn cluster_lane_samples(
    samples: &VecDeque<u64>,
    start_ns: u64,
    end_ns: u64,
    slot_count: usize,
) -> Vec<LaneRenderPoint> {
    if samples.is_empty() || slot_count == 0 || start_ns > end_ns {
        return Vec::new();
    }

    let visible = collect_visible_lane_samples(samples, start_ns, end_ns);
    if visible.is_empty() {
        return Vec::new();
    }

    if start_ns == end_ns {
        return vec![LaneRenderPoint {
            timestamp_ns: start_ns,
            count: u32::try_from(visible.len()).unwrap_or(u32::MAX),
        }];
    }

    if visible.len() <= slot_count.saturating_mul(2) {
        return visible
            .into_iter()
            .map(|timestamp_ns| LaneRenderPoint {
                timestamp_ns,
                count: 1,
            })
            .collect();
    }

    let span_ns = end_ns.saturating_sub(start_ns).max(1);
    let mut first_per_slot = vec![None::<u64>; slot_count];
    let mut last_per_slot = vec![None::<u64>; slot_count];
    let mut count_per_slot = vec![0_u32; slot_count];

    for timestamp_ns in visible {
        let relative = timestamp_ns.saturating_sub(start_ns);
        let slot_index = ((relative as u128).saturating_mul(slot_count.saturating_sub(1) as u128)
            / span_ns as u128) as usize;
        let slot = slot_index.min(slot_count.saturating_sub(1));
        if first_per_slot[slot].is_none() {
            first_per_slot[slot] = Some(timestamp_ns);
        }
        last_per_slot[slot] = Some(timestamp_ns);
        count_per_slot[slot] = count_per_slot[slot].saturating_add(1);
    }

    let mut points = Vec::new();
    for slot in 0..slot_count {
        let Some(first) = first_per_slot[slot] else {
            continue;
        };
        let last = last_per_slot[slot].unwrap_or(first);
        let count = count_per_slot[slot].max(1);
        let center = first.saturating_add(last.saturating_sub(first) / 2);
        points.push(LaneRenderPoint {
            timestamp_ns: center,
            count,
        });
    }
    points
}

fn merge_lane_points_by_pixel_distance(
    points: Vec<LaneRenderPoint>,
    viewport_range: TimelineRenderRange,
    pixel_width: f32,
    min_distance_px: f32,
) -> Vec<LaneRenderPoint> {
    if points.len() <= 1 {
        return points;
    }
    let span_ns = viewport_range
        .end_ns
        .saturating_sub(viewport_range.start_ns)
        .max(1);
    let mut merged = Vec::new();
    let mut cluster = vec![points[0].clone()];

    for point in points.into_iter().skip(1) {
        let previous = cluster.last().expect("cluster has at least one entry");
        let delta_ns = point.timestamp_ns.abs_diff(previous.timestamp_ns);
        let distance_px = (delta_ns as f64 / span_ns as f64) * pixel_width as f64;
        if distance_px < min_distance_px as f64 {
            cluster.push(point);
            continue;
        }
        merged.push(select_cluster_representative(cluster.as_slice()));
        cluster.clear();
        cluster.push(point);
    }

    if !cluster.is_empty() {
        merged.push(select_cluster_representative(cluster.as_slice()));
    }
    merged
}

fn select_cluster_representative(cluster: &[LaneRenderPoint]) -> LaneRenderPoint {
    cluster
        .iter()
        .cloned()
        .max_by(|left, right| {
            left.count
                .cmp(&right.count)
                .then_with(|| right.timestamp_ns.cmp(&left.timestamp_ns))
        })
        .expect("merge clusters are always non-empty")
}

fn collect_visible_lane_samples(samples: &VecDeque<u64>, start_ns: u64, end_ns: u64) -> Vec<u64> {
    if samples.is_empty() {
        return Vec::new();
    }

    let (first, second) = samples.as_slices();
    let mut visible = Vec::new();
    push_visible_slice(first, start_ns, end_ns, &mut visible);
    push_visible_slice(second, start_ns, end_ns, &mut visible);
    visible
}

fn push_visible_slice(slice: &[u64], start_ns: u64, end_ns: u64, out: &mut Vec<u64>) {
    if slice.is_empty() {
        return;
    }
    let lower = slice.partition_point(|timestamp| *timestamp < start_ns);
    let upper = slice.partition_point(|timestamp| *timestamp <= end_ns);
    if lower < upper {
        out.extend_from_slice(&slice[lower..upper]);
    }
}

pub(crate) fn is_manual_timeline_navigation(
    selected_timestamp_ns: Option<u64>,
    pan_delta_fraction: Option<f32>,
    zoom_factor: Option<f32>,
    lane_scroll_delta: Option<f32>,
) -> bool {
    selected_timestamp_ns.is_some()
        || pan_delta_fraction.is_some()
        || zoom_factor.is_some()
        || lane_scroll_delta.is_some()
}

fn trim_timeline_to_capacity(
    timeline: &mut Vec<u64>,
    timeline_index: &mut Option<usize>,
    max_points: usize,
) {
    if timeline.len() <= max_points {
        return;
    }

    let trim_count = timeline.len().saturating_sub(max_points);
    timeline.drain(0..trim_count);
    *timeline_index = timeline_index.map(|index| index.saturating_sub(trim_count));
    if timeline.is_empty() {
        *timeline_index = None;
    }
}

fn default_namespace_value() -> String {
    String::new()
}

fn default_next_parameter_panel_id() -> u64 {
    1
}

fn duration_to_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
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

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, time::Duration};

    use super::{
        clamp_timeline_span, cluster_lane_samples, derive_timeline_render_range,
        is_manual_timeline_navigation, merge_lane_points_by_pixel_distance,
        timeline_visible_lane_count, trim_timeline_to_capacity, zoom_range_around_focus,
        LaneRenderPoint, TimelineRenderRange, TimelineViewportState,
    };

    #[test]
    fn timeline_trim_keeps_most_recent_points() {
        let mut timeline = vec![1_u64, 2, 3, 4, 5];
        let mut index = Some(4_usize);

        trim_timeline_to_capacity(&mut timeline, &mut index, 3);

        assert_eq!(timeline, vec![3_u64, 4, 5]);
        assert_eq!(index, Some(2));
    }

    #[test]
    fn timeline_trim_clamps_index_to_start_when_anchor_evicted() {
        let mut timeline = vec![10_u64, 20, 30, 40];
        let mut index = Some(0_usize);

        trim_timeline_to_capacity(&mut timeline, &mut index, 2);

        assert_eq!(timeline, vec![30_u64, 40]);
        assert_eq!(index, Some(0));
    }

    #[test]
    fn timeline_range_follow_live_uses_latest_end() {
        let full = TimelineRenderRange {
            start_ns: 1_000_000_000,
            end_ns: 11_000_000_000,
        };
        let range = derive_timeline_render_range(
            full,
            TimelineViewportState {
                span: Some(Duration::from_secs(4)),
                pan_offset_ns: -2_000,
                ..TimelineViewportState::default()
            },
            true,
        );

        assert_eq!(range.end_ns, 11_000_000_000);
        assert_eq!(range.start_ns, 7_000_000_000);
    }

    #[test]
    fn timeline_range_manual_pan_is_clamped() {
        let full = TimelineRenderRange {
            start_ns: 1_000_000_000,
            end_ns: 11_000_000_000,
        };
        let range = derive_timeline_render_range(
            full,
            TimelineViewportState {
                span: Some(Duration::from_secs(4)),
                pan_offset_ns: -20_000_000_000,
                ..TimelineViewportState::default()
            },
            false,
        );
        assert_eq!(
            range,
            TimelineRenderRange {
                start_ns: 1_000_000_000,
                end_ns: 5_000_000_000
            }
        );
    }

    #[test]
    fn clamp_span_enforces_minimum_bound() {
        let clamped = clamp_timeline_span(Duration::from_nanos(1_000), Duration::from_secs(1));
        assert!(clamped >= Duration::from_millis(50));
    }

    #[test]
    fn lane_sample_clustering_produces_monotonic_points() {
        let samples = (0..10_000_u64)
            .map(|index| index.saturating_mul(1_000))
            .collect::<VecDeque<_>>();
        let points = cluster_lane_samples(&samples, 100_000, 6_000_000, 200);
        assert!(!points.is_empty());
        assert!(points
            .windows(2)
            .all(|window| { window[0].timestamp_ns <= window[1].timestamp_ns }));
        assert!(points.iter().all(|point| point.count >= 1));
    }

    #[test]
    fn lane_window_capacity_stays_positive() {
        assert!(timeline_visible_lane_count(16.0) >= 1);
        assert!(timeline_visible_lane_count(48.0) >= 1);
    }

    #[test]
    fn merge_keeps_highest_density_representative() {
        let points = vec![
            LaneRenderPoint {
                timestamp_ns: 1_000,
                count: 1,
            },
            LaneRenderPoint {
                timestamp_ns: 1_002,
                count: 8,
            },
            LaneRenderPoint {
                timestamp_ns: 3_500,
                count: 2,
            },
        ];
        let merged = merge_lane_points_by_pixel_distance(
            points,
            TimelineRenderRange {
                start_ns: 0,
                end_ns: 10_000,
            },
            400.0,
            4.0,
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].timestamp_ns, 1_002);
    }

    #[test]
    fn manual_navigation_flags_disable_follow_semantics() {
        assert!(is_manual_timeline_navigation(Some(100), None, None, None));
        assert!(is_manual_timeline_navigation(None, Some(0.2), None, None));
        assert!(is_manual_timeline_navigation(None, None, Some(0.8), None));
        assert!(is_manual_timeline_navigation(None, None, None, Some(12.0)));
        assert!(!is_manual_timeline_navigation(None, None, None, None));
    }

    #[test]
    fn zoom_range_keeps_focus_reasonably_stable() {
        let full = TimelineRenderRange {
            start_ns: 0,
            end_ns: 10_000,
        };
        let current = TimelineRenderRange {
            start_ns: 2_000,
            end_ns: 8_000,
        };
        let focus_ns = 5_000;
        let next = zoom_range_around_focus(full, current, focus_ns, 0.5, Duration::from_nanos(50));
        let prev_relative = (focus_ns - current.start_ns) as f64
            / (current.end_ns - current.start_ns).max(1) as f64;
        let next_relative =
            (focus_ns - next.start_ns) as f64 / (next.end_ns - next.start_ns).max(1) as f64;
        assert!((prev_relative - next_relative).abs() < 0.2);
    }
}

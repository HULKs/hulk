use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use egui_dock::DockState;
use hulkz_stream::{BackendStats, SourceStats};
use serde::{Deserialize, Serialize};
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_util::sync::CancellationToken;

use crate::protocol::{
    DiscoveredParameter, DiscoveredPublisher, DiscoveredSession, DisplayedRecord, StreamId,
    ViewerConfig, WorkerCommand, WorkerEventEnvelope,
};

use super::workspace_panel::WorkspacePanel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PersistedUiState {
    #[serde(default = "default_true")]
    pub(super) ingest_enabled: bool,
    #[serde(default = "default_true")]
    pub(super) follow_live: bool,
    #[serde(default)]
    pub(super) next_stream_id: StreamId,
    #[serde(default)]
    pub(super) next_parameter_panel_id: u64,
    #[serde(default)]
    pub(super) default_namespace: String,
    #[serde(default = "default_true")]
    pub(super) show_discovery: bool,
    #[serde(default = "default_true")]
    pub(super) show_timeline: bool,
}

#[derive(Debug, Default)]
pub(super) struct StreamRuntimeState {
    pub(super) generation: u64,
    pub(super) source_label: String,
    pub(super) current_record: Option<DisplayedRecord>,
    pub(super) source_stats: Option<SourceStats>,
    pub(super) history_loading: bool,
    pub(super) history_total_records: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TimelineViewportState {
    pub(crate) span: Option<Duration>,
    pub(crate) manual_end_ns: Option<u64>,
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
pub(super) struct TimelineLaneState {
    pub(super) key: TimelineLaneKey,
    pub(super) sample_timestamps: VecDeque<u64>,
    pub(super) last_seen_ns: u64,
    pub(super) active_bindings: usize,
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

#[derive(Debug, Clone, Copy)]
pub(super) struct ShellState {
    pub(super) show_discovery: bool,
    pub(super) show_timeline: bool,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            show_discovery: true,
            show_timeline: true,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct DiscoveryState {
    pub(super) publishers: Vec<DiscoveredPublisher>,
    pub(super) parameters: Vec<DiscoveredParameter>,
    pub(super) sessions: Vec<DiscoveredSession>,
}

#[derive(Debug)]
pub(super) struct TimelineState {
    pub(super) global_timeline: Vec<u64>,
    pub(super) global_timeline_index: Option<usize>,
    pub(super) timeline_hover_preview: Option<u64>,
    pub(super) timeline_viewport: TimelineViewportState,
    pub(super) stream_lane_bindings: BTreeMap<StreamId, TimelineLaneKey>,
    pub(super) timeline_lanes: BTreeMap<TimelineLaneKey, TimelineLaneState>,
    pub(super) lane_order_cache: Vec<TimelineLaneKey>,
    pub(super) lane_order_dirty: bool,
    pub(super) pending_scrub_anchor: Option<u64>,
    pub(super) last_scrub_emitted: Instant,
}

#[derive(Debug)]
pub(super) struct WorkspaceState {
    pub(super) dock_state: DockState<WorkspacePanel>,
    pub(super) stream_states: BTreeMap<StreamId, StreamRuntimeState>,
    pub(super) binding_cache: BTreeMap<StreamId, Option<crate::protocol::SourceBindingRequest>>,
    pub(super) next_stream_id: StreamId,
    pub(super) next_parameter_panel_id: u64,
}

pub(super) struct RuntimeState {
    pub(super) runtime: Runtime,
    pub(super) worker_task: Option<tokio::task::JoinHandle<()>>,
    pub(super) cancellation_token: CancellationToken,
    pub(super) command_tx: mpsc::Sender<WorkerCommand>,
    pub(super) pending_commands: VecDeque<WorkerCommand>,
    pub(super) worker_wake_armed: Arc<AtomicBool>,
    pub(super) event_rx: mpsc::Receiver<WorkerEventEnvelope>,
    pub(super) shutdown_started: bool,
}

#[derive(Debug)]
pub(super) struct UiState {
    pub(super) ingest_enabled: bool,
    pub(super) follow_live: bool,
    pub(super) default_namespace: String,
    pub(super) default_namespace_input: String,
    pub(super) ready: bool,
    pub(super) last_error: Option<String>,
    pub(super) backend_stats: Option<BackendStats>,
    pub(super) frame_last_ms: f32,
    pub(super) frame_ema_ms: f32,
    pub(super) frame_processed_events: usize,
    pub(super) frame_processed_event_bytes: usize,
}

pub(super) const MIN_TIMELINE_SPAN: Duration = Duration::from_millis(50);
pub(super) const DEFAULT_TIMELINE_LANE_HEIGHT_PX: f32 = 22.0;

pub struct ViewerApp {
    pub(super) config: ViewerConfig,
    pub(super) shell: ShellState,
    pub(super) discovery: DiscoveryState,
    pub(super) timeline: TimelineState,
    pub(super) workspace: WorkspaceState,
    pub(super) runtime: RuntimeState,
    pub(super) ui: UiState,
}

#[derive(Debug, Clone, Default)]
pub struct ViewerStartupOverrides {
    pub namespace: Option<String>,
    pub source_expression: Option<String>,
    pub storage_path: Option<PathBuf>,
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::PersistedUiState;

    #[test]
    fn persisted_ui_defaults_allow_missing_fields() {
        let state: PersistedUiState =
            serde_json::from_str("{}").expect("persisted ui defaults should deserialize");

        assert!(state.ingest_enabled);
        assert!(state.follow_live);
        assert_eq!(state.next_stream_id, 0);
        assert_eq!(state.next_parameter_panel_id, 0);
        assert_eq!(state.default_namespace, "");
        assert!(state.show_discovery);
        assert!(state.show_timeline);
    }
}

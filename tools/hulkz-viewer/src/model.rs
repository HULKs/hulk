use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use hulkz_stream::{BackendStats, PlaneKind, SourceStats};

pub type StreamId = u64;

#[derive(Debug, Clone)]
pub struct ViewerConfig {
    pub namespace: String,
    pub source_expression: String,
    pub storage_path: Option<PathBuf>,
    pub poll_interval: Duration,
    pub discovery_reconcile_interval: Duration,
    pub scrub_window_radius: Duration,
    pub scrub_prefetch_radius: Duration,
    pub scrub_debounce: Duration,
    pub max_timeline_points: usize,
    pub max_samples_per_lane: usize,
    pub max_retained_lanes: usize,
    pub live_event_batch_max: usize,
    pub live_event_batch_delay: Duration,
    pub worker_command_channel_capacity: usize,
    pub worker_event_channel_capacity: usize,
    pub worker_internal_event_channel_capacity: usize,
    pub discovery_event_channel_capacity: usize,
    pub max_events_per_frame: usize,
    pub max_event_bytes_per_frame: usize,
    pub max_event_ingest_time_per_frame: Duration,
    pub repaint_delay_on_activity: Duration,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            namespace: "demo".to_string(),
            source_expression: "odometry".to_string(),
            storage_path: None,
            poll_interval: Duration::from_millis(100),
            discovery_reconcile_interval: Duration::from_secs(30),
            scrub_window_radius: Duration::from_secs(5),
            scrub_prefetch_radius: Duration::from_secs(10),
            scrub_debounce: Duration::from_millis(200),
            max_timeline_points: 200_000,
            max_samples_per_lane: 50_000,
            max_retained_lanes: 512,
            live_event_batch_max: 32,
            live_event_batch_delay: Duration::from_millis(40),
            worker_command_channel_capacity: 256,
            worker_event_channel_capacity: 512,
            worker_internal_event_channel_capacity: 512,
            discovery_event_channel_capacity: 512,
            max_events_per_frame: 256,
            max_event_bytes_per_frame: 1_500_000,
            max_event_ingest_time_per_frame: Duration::from_millis(6),
            repaint_delay_on_activity: Duration::from_millis(10),
        }
    }
}

#[derive(Clone)]
pub struct WorkerWakeNotifier {
    notify_fn: Arc<dyn Fn() + Send + Sync>,
}

impl WorkerWakeNotifier {
    pub fn new<F>(notify_fn: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            notify_fn: Arc::new(notify_fn),
        }
    }

    pub fn notify(&self) {
        (self.notify_fn)();
    }
}

#[derive(Debug, Clone)]
pub struct DisplayedRecord {
    pub timestamp_nanos: u64,
    pub json_pretty: Option<String>,
    pub raw_fallback: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveredPublisher {
    pub namespace: String,
    pub node: String,
    pub path_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveredSession {
    pub namespace: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveredParameter {
    pub namespace: String,
    pub node: String,
    pub path_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParameterReference {
    pub namespace: String,
    pub node: String,
    pub path_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBindingRequest {
    pub namespace: String,
    pub plane: PlaneKind,
    pub path_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceBindingInfo {
    pub namespace: String,
    pub path_expression: String,
}

#[derive(Debug, Clone)]
pub enum WorkerCommand {
    SetIngestEnabled(bool),
    SetDiscoveryNamespace(String),
    BindStream {
        stream_id: StreamId,
        request: SourceBindingRequest,
    },
    RemoveStream {
        stream_id: StreamId,
    },
    ReadParameter(ParameterReference),
    SetParameter {
        target: ParameterReference,
        value_json: String,
    },
    SetScrubAnchor {
        stream_id: StreamId,
        anchor_nanos: u64,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    StreamHistoryBegin {
        stream_id: StreamId,
        generation: u64,
    },
    StreamRecordsChunk {
        stream_id: StreamId,
        generation: u64,
        records: Vec<DisplayedRecord>,
        source: RecordChunkSource,
    },
    StreamHistoryEnd {
        stream_id: StreamId,
        generation: u64,
        total_records: usize,
    },
    AnchorRecord {
        stream_id: StreamId,
        anchor_nanos: u64,
        record: Option<DisplayedRecord>,
    },
    SourceBound {
        stream_id: StreamId,
        generation: u64,
        label: String,
        binding: SourceBindingInfo,
    },
    DiscoveryPatch {
        op: DiscoveryOp,
    },
    DiscoverySnapshot {
        publishers: Vec<DiscoveredPublisher>,
        parameters: Vec<DiscoveredParameter>,
        sessions: Vec<DiscoveredSession>,
    },
    ParameterValueLoaded {
        target: ParameterReference,
        value_pretty: String,
    },
    ParameterWriteResult {
        target: ParameterReference,
        success: bool,
        message: String,
    },
    StreamStats {
        stream_id: StreamId,
        source: Box<SourceStats>,
    },
    BackendStats {
        backend: Box<BackendStats>,
    },
    Error(String),
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordChunkSource {
    History,
    Live,
}

#[derive(Debug, Clone)]
pub enum DiscoveryOp {
    PublisherUpsert(DiscoveredPublisher),
    PublisherRemove(DiscoveredPublisher),
    ParameterUpsert(DiscoveredParameter),
    ParameterRemove(DiscoveredParameter),
    SessionUpsert(DiscoveredSession),
    SessionRemove(DiscoveredSession),
    ResetNamespace(String),
}

#[derive(Debug, Clone)]
pub struct WorkerEventEnvelope {
    pub event: WorkerEvent,
    pub approx_bytes: usize,
}

impl WorkerEventEnvelope {
    pub fn new(event: WorkerEvent) -> Self {
        let approx_bytes = approximate_event_size_bytes(&event);
        Self {
            event,
            approx_bytes,
        }
    }
}

fn approximate_event_size_bytes(event: &WorkerEvent) -> usize {
    match event {
        WorkerEvent::StreamHistoryBegin { .. } => 64,
        WorkerEvent::StreamRecordsChunk { records, .. } => records
            .iter()
            .map(|record| {
                64 + record
                    .json_pretty
                    .as_ref()
                    .map(|value| value.len())
                    .unwrap_or(0)
                    + record
                        .raw_fallback
                        .as_ref()
                        .map(|value| value.len())
                        .unwrap_or(0)
            })
            .sum::<usize>()
            .saturating_add(32),
        WorkerEvent::StreamHistoryEnd { .. } => 64,
        WorkerEvent::AnchorRecord { record, .. } => {
            96 + record
                .as_ref()
                .and_then(|record| {
                    record
                        .json_pretty
                        .as_ref()
                        .map(|text| text.len())
                        .or_else(|| record.raw_fallback.as_ref().map(|text| text.len()))
                })
                .unwrap_or(0)
        }
        WorkerEvent::SourceBound { label, .. } => 96 + label.len(),
        WorkerEvent::DiscoveryPatch { op } => match op {
            DiscoveryOp::PublisherUpsert(item) | DiscoveryOp::PublisherRemove(item) => {
                64 + item.namespace.len() + item.node.len() + item.path_expression.len()
            }
            DiscoveryOp::ParameterUpsert(item) | DiscoveryOp::ParameterRemove(item) => {
                64 + item.namespace.len() + item.node.len() + item.path_expression.len()
            }
            DiscoveryOp::SessionUpsert(item) | DiscoveryOp::SessionRemove(item) => {
                64 + item.namespace.len() + item.id.len()
            }
            DiscoveryOp::ResetNamespace(namespace) => 64 + namespace.len(),
        },
        WorkerEvent::DiscoverySnapshot {
            publishers,
            parameters,
            sessions,
        } => 96 + publishers.len() * 96 + parameters.len() * 96 + sessions.len() * 64,
        WorkerEvent::ParameterValueLoaded { value_pretty, .. } => 64 + value_pretty.len(),
        WorkerEvent::ParameterWriteResult { message, .. } => 64 + message.len(),
        WorkerEvent::StreamStats { .. } => 128,
        WorkerEvent::BackendStats { .. } => 128,
        WorkerEvent::Error(message) => 64 + message.len(),
        WorkerEvent::Ready => 16,
    }
}

pub fn should_emit_scrub_command(last_emitted: Instant, now: Instant, debounce: Duration) -> bool {
    now.saturating_duration_since(last_emitted) >= debounce
}

#[cfg(test)]
mod tests {
    use super::should_emit_scrub_command;
    use std::time::{Duration, Instant};

    #[test]
    fn scrub_debounce_blocks_rapid_updates() {
        let now = Instant::now();
        let debounce = Duration::from_millis(200);

        assert!(!should_emit_scrub_command(
            now,
            now + Duration::from_millis(100),
            debounce
        ));
        assert!(should_emit_scrub_command(
            now,
            now + Duration::from_millis(220),
            debounce
        ));
    }
}

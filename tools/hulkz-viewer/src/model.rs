use std::{
    path::PathBuf,
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
        }
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
    RecordsAppended {
        stream_id: StreamId,
        records: Vec<DisplayedRecord>,
    },
    AnchorRecord {
        stream_id: StreamId,
        anchor_nanos: u64,
        record: Option<DisplayedRecord>,
    },
    SourceBound {
        stream_id: StreamId,
        label: String,
        binding: SourceBindingInfo,
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

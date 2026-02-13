use std::sync::Arc;

use hulkz_stream::{BackendStats, SourceStats};

use crate::discovery_types::{DiscoveredParameter, DiscoveredPublisher, DiscoveredSession};

use super::{ParameterReference, SourceBindingInfo, StreamId};

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

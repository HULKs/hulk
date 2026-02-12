use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use hulkz::{Scope, ScopedPath, Timestamp};
use zenoh::bytes::Encoding;

/// Backend access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpenMode {
    /// Open existing storage for historical queries only.
    ReadOnly,
    /// Enable ingest and durable appends.
    ReadWrite,
}

/// Logical hulkz data plane represented by a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaneKind {
    Data,
    View,
    ParamReadUpdates,
    /// External topic that cannot be mapped back to a known hulkz plane.
    ExternalRaw,
}

/// Namespace binding strategy used to resolve a source key expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamespaceBinding {
    /// Re-resolve against the current backend target namespace.
    FollowTarget,
    /// Always use the provided namespace.
    Pinned(String),
}

/// Durable source identity aligned with hulkz scope/path semantics.
#[derive(Debug, Clone)]
pub struct SourceSpec {
    pub plane: PlaneKind,
    pub path: ScopedPath,
    /// Required for private scope reads when no implicit node identity is available.
    pub node_override: Option<String>,
    pub namespace_binding: NamespaceBinding,
}

impl PartialEq for SourceSpec {
    fn eq(&self, other: &Self) -> bool {
        self.plane == other.plane
            && self.path.scope() == other.path.scope()
            && self.path.path() == other.path.path()
            && self.node_override == other.node_override
            && self.namespace_binding == other.namespace_binding
    }
}

impl Eq for SourceSpec {}

impl Hash for SourceSpec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.plane.hash(state);
        self.path.scope().hash(state);
        self.path.path().hash(state);
        self.node_override.hash(state);
        self.namespace_binding.hash(state);
    }
}

impl SourceSpec {
    /// Convenience access to the scoped path kind.
    pub fn scope(&self) -> Scope {
        self.path.scope()
    }
}

/// One raw sample plus indexing metadata as exposed by query APIs.
#[derive(Debug, Clone)]
pub struct StreamRecord {
    pub source: SourceSpec,
    /// Namespace used at ingest time after binding resolution.
    pub effective_namespace: Option<String>,
    pub timestamp: Timestamp,
    pub encoding: Encoding,
    pub payload: Arc<[u8]>,
}

/// Per-source observed bounds and fault status.
#[derive(Debug, Clone, Default)]
pub struct SourceStats {
    /// Oldest durable timestamp currently indexed.
    pub durable_oldest: Option<Timestamp>,
    /// Newest durable timestamp currently indexed.
    pub durable_latest: Option<Timestamp>,
    /// Number of durable records currently indexed.
    pub durable_len: u64,
    /// Newest timestamp seen by ingest worker (may be ahead of durable frontier).
    pub ingest_frontier: Option<Timestamp>,
    /// Newest timestamp known to be durably committed.
    pub durable_frontier: Option<Timestamp>,
    /// Last non-fatal ingest/storage/query error for this source.
    pub last_error: Option<String>,
}

/// Global cache usage and effectiveness counters.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub bytes_used: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
}

/// Backend-wide runtime stats snapshot.
#[derive(Debug, Clone, Default)]
pub struct BackendStats {
    pub active_sources: usize,
    pub active_subscribers: usize,
    pub cache: CacheStats,
    /// Approximate number of records currently waiting in durable writer queue.
    pub writer_queue_depth: usize,
    /// Maximum observed writer queue depth since backend start.
    pub writer_queue_high_watermark: usize,
    /// Number of times ingest encountered a full writer queue and had to wait.
    pub writer_backpressure_events: u64,
}

/// One aggregate timeline bucket.
#[derive(Debug, Clone)]
pub struct TimelineBucket {
    pub bucket_start: Timestamp,
    pub bucket_end: Timestamp,
    pub message_count: u64,
    pub min_ts: Option<Timestamp>,
    pub max_ts: Option<Timestamp>,
}

/// Timeline query result including ingest-vs-durable frontiers.
#[derive(Debug, Clone, Default)]
pub struct TimelineSummary {
    pub buckets: Vec<TimelineBucket>,
    pub ingest_frontier: Option<Timestamp>,
    pub durable_frontier: Option<Timestamp>,
}

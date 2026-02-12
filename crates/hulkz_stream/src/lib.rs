pub mod backend;
pub mod cache;
pub mod error;
mod keyspace;
pub mod storage;
pub mod types;

pub use backend::{SourceHandle, StreamBackend, StreamBackendBuilder, StreamDriver};
pub use error::{Error, Result};
pub use types::{
    BackendStats, CacheStats, NamespaceBinding, OpenMode, PlaneKind, SourceSpec, SourceStats,
    StreamRecord, TimelineBucket, TimelineSummary,
};

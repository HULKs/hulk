use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("hulkz error: {0}")]
    Hulkz(#[from] hulkz::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MCAP error: {0}")]
    Mcap(#[from] mcap::McapError),

    #[error("serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("source with private scope requires node override")]
    NodeRequiredForPrivate,

    #[error("invalid source scope for plane")]
    InvalidSource,

    #[error("source not found")]
    SourceNotFound,

    #[error("read-only backend does not support this operation")]
    ReadOnly,

    #[error("backend is closed")]
    BackendClosed,

    #[error("control channel closed")]
    ControlChannelClosed,

    #[error("response channel closed")]
    ResponseChannelClosed,

    #[error("invalid timeline bucket count")]
    InvalidBucketCount,

    #[error("invalid timeline range")]
    InvalidTimelineRange,

    #[error("storage path is invalid: {0}")]
    InvalidStoragePath(PathBuf),

    #[error("unsupported manifest version (expected {expected}, found {found})")]
    UnsupportedManifestVersion { expected: u32, found: u32 },

    #[error("durable index is inconsistent with underlying segment")]
    BadDurableIndex,

    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

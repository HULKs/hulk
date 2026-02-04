//! Error types for hulkz.
//!
//! All fallible operations return [`Result<T>`](crate::Result) which uses [`Error`] as the error
//! type.

use std::path::PathBuf;

use zenoh::bytes::Encoding;

/// The unified error type for hulkz operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    ScopedPath(#[from] ScopedPathError),

    #[error("CDR serialization failed: {0}")]
    CdrSerialize(#[source] cdr::Error),

    #[error("CDR deserialization failed: {0}")]
    CdrDeserialize(#[source] cdr::Error),

    #[error("JSON serialization failed: {0}")]
    JsonSerialize(#[source] serde_json::Error),

    #[error("JSON deserialization failed: {0}")]
    JsonDeserialize(#[source] serde_json::Error),

    #[error("JSON5 parse error: {0}")]
    Json5Parse(#[from] json5::Error),

    #[error("zenoh error: {0}")]
    Zenoh(#[from] zenoh::Error),

    #[error("failed to load config file '{}': {source}", path.display())]
    ConfigFileIo {
        /// The file path that failed to load.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parameter not found: no node is serving '{0}'")]
    ParameterNotFound(String),

    #[error("parameter '{0}' has no configured value and no default")]
    ParameterNoDefault(String),

    #[error("parameter validation failed: {0}")]
    ParameterValidation(String),

    #[error("parameter rejected: {}", .0.join("; "))]
    ParameterRejected(Vec<String>),

    #[error("config parse error: {0}")]
    ConfigParse(String),

    #[error("query has empty payload")]
    EmptyPayload,

    #[error("unsupported encoding: {0}")]
    UnsupportedEncoding(Encoding),

    #[error("private parameters require a node target, use .on_node(\"node_name\")")]
    NodeRequiredForPrivate,

    #[error("subscriber closed")]
    SubscriberClosed,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScopedPathError {
    #[error("path cannot be empty")]
    Empty,
    #[error("invalid path: {0}")]
    Invalid(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

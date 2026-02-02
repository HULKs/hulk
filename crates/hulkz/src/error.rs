//! Error types for hulkz.
//!
//! All fallible operations return [`Result<T>`](crate::Result) which uses
//! [`Error`] as the error type.

use zenoh::bytes::Encoding;

/// The unified error type for hulkz operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Key(#[from] KeyError),

    #[error("{0}")]
    ScopedPath(#[from] ScopedPathError),

    #[error("CDR serialization: {0}")]
    CdrSerialize(#[source] cdr::Error),

    #[error("CDR deserialization: {0}")]
    CdrDeserialize(#[source] cdr::Error),

    #[error("JSON serialization: {0}")]
    JsonSerialize(#[source] serde_json::Error),

    #[error("JSON deserialization: {0}")]
    JsonDeserialize(#[source] serde_json::Error),

    #[error("JSON5 parse: {0}")]
    Json5Parse(#[from] json5::Error),

    #[error("zenoh: {0}")]
    Zenoh(#[from] zenoh::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("parameter not found: {0}")]
    ParameterNotFound(String),

    #[error("parameter has no value and no default: {0}")]
    ParameterNoDefault(String),

    #[error("parameter validation failed: {0}")]
    ParameterValidation(String),

    #[error("parameter rejected: {}", .0.join("; "))]
    ParameterRejected(Vec<String>),

    #[error("config parse error: {0}")]
    ConfigParse(String),

    #[error("empty payload in query")]
    EmptyPayload,

    #[error("unsupported encoding: {0}")]
    UnsupportedEncoding(Encoding),
}

/// Key expression construction errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum KeyError {
    #[error("local and private scopes require a namespace")]
    MissingNamespace,
    #[error("private scope requires a node name")]
    MissingNode,
}

/// Scoped path parsing errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ScopedPathError {
    #[error("path cannot be empty")]
    Empty,
    #[error("invalid path: {0}")]
    Invalid(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

use std::path::PathBuf;

use super::{LayerPath, ParameterKey};

/// Errors produced by local and remote parameter operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ParameterError {
    /// Parameter file does not exist.
    #[error("parameter file not found: {}", path.display())]
    FileNotFound { path: PathBuf },

    /// Parameter file could not be read.
    #[error("failed to read parameter file {}: {source}", path.display())]
    FileReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Parameter file could not be parsed.
    #[error("failed to parse parameter file {}: {source}", path.display())]
    ParseError {
        path: PathBuf,
        #[source]
        source: json5::Error,
    },

    /// Parameter layers could not be merged.
    #[error("merge error: {message}")]
    MergeError { message: String },

    /// Typed parameter deserialization failed.
    #[error("typed parameter deserialization failed: {source}")]
    DeserializationError {
        #[source]
        source: serde_json::Error,
    },

    /// Typed parameter serialization failed.
    #[error("typed parameter serialization failed: {source}")]
    SerializationError {
        #[source]
        source: serde_json::Error,
    },

    /// Parameters could not be serialized for persistence.
    #[error("failed to serialize parameters for {}: {source}", path.display())]
    PersistenceSerializationError {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    /// Parameter validation failed.
    #[error("parameter validation failed: {message}")]
    ValidationError { message: String },

    /// Parameter revision did not match the expected value.
    #[error("revision mismatch: expected {expected}, actual {actual}")]
    RevisionMismatch { expected: u64, actual: u64 },

    /// Parameters could not be persisted.
    #[error("failed to persist {}: {source}", path.display())]
    PersistenceError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Parameter path was invalid.
    #[error("invalid path '{path}': {reason}")]
    PathError { path: String, reason: String },

    /// Parameter layer list was empty.
    #[error("parameter layer list must not be empty")]
    EmptyLayerList,

    /// Parameter key was invalid.
    #[error("invalid parameter key '{key}'")]
    InvalidParameterKey { key: ParameterKey },

    /// Target layer is not active for the node.
    #[error("target layer is not active for this node: {layer}")]
    LayerNotActive { layer: LayerPath },

    /// Parameters are already bound for the node.
    #[error("parameters already bound for node {node_fqn}")]
    AlreadyBound { node_fqn: String },

    /// Remote parameter service returned an error.
    #[error("remote parameter error: {message}")]
    RemoteError { message: String },

    /// Parameter operation failed through an underlying subsystem.
    #[error("parameter operation failed while {operation}: {source}")]
    Operation {
        operation: String,
        #[source]
        source: crate::error::BoxError,
    },

    /// Remote parameter payload could not be parsed.
    #[error("failed to parse remote parameter payload: {source}")]
    RemotePayloadParseError {
        #[source]
        source: serde_json::Error,
    },
}

pub type Result<T> = std::result::Result<T, ParameterError>;

impl From<serde_json::Error> for ParameterError {
    fn from(source: serde_json::Error) -> Self {
        Self::DeserializationError { source }
    }
}

impl ParameterError {
    pub(crate) fn operation(
        operation: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Operation {
            operation: operation.into(),
            source: Box::new(source),
        }
    }
}

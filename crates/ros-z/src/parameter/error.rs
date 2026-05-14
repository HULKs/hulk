use std::path::PathBuf;

use super::{LayerPath, ParameterKey};

#[derive(Debug, thiserror::Error)]
pub enum ParameterError {
    #[error("parameter file not found: {}", path.display())]
    FileNotFound { path: PathBuf },

    #[error("failed to read parameter file {}: {source}", path.display())]
    FileReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse parameter file {}: {source}", path.display())]
    ParseError {
        path: PathBuf,
        #[source]
        source: json5::Error,
    },

    #[error("merge error: {message}")]
    MergeError { message: String },

    #[error("typed parameter deserialization failed: {source}")]
    DeserializationError {
        #[source]
        source: serde_json::Error,
    },

    #[error("typed parameter serialization failed: {source}")]
    SerializationError {
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to serialize parameters for {}: {source}", path.display())]
    PersistenceSerializationError {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("parameter validation failed: {message}")]
    ValidationError { message: String },

    #[error("revision mismatch: expected {expected}, actual {actual}")]
    RevisionMismatch { expected: u64, actual: u64 },

    #[error("failed to persist {}: {source}", path.display())]
    PersistenceError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid path '{path}': {reason}")]
    PathError { path: String, reason: String },

    #[error("parameter layer list must not be empty")]
    EmptyLayerList,

    #[error("invalid parameter key '{key}'")]
    InvalidParameterKey { key: ParameterKey },

    #[error("target layer is not active for this node: {layer}")]
    LayerNotActive { layer: LayerPath },

    #[error("parameters already bound for node {node_fqn}")]
    AlreadyBound { node_fqn: String },

    #[error("remote parameter error: {message}")]
    RemoteError { message: String },

    #[error("parameter operation failed while {operation}: {source}")]
    Operation {
        operation: String,
        #[source]
        source: crate::error::BoxError,
    },

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ParameterError;

    #[test]
    fn remote_payload_parse_error_uses_truthful_wording_and_preserves_source() {
        let source = serde_json::from_str::<serde_json::Value>("{ not json }").unwrap_err();
        let error = ParameterError::RemotePayloadParseError { source };

        assert!(
            error
                .to_string()
                .contains("failed to parse remote parameter payload")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn persistence_serialization_error_uses_path_and_preserves_source() {
        let source = serde_json::from_str::<serde_json::Value>("{ not json }").unwrap_err();
        let error = ParameterError::PersistenceSerializationError {
            path: PathBuf::from("/tmp/params.json5"),
            source,
        };

        assert!(
            error
                .to_string()
                .contains("failed to serialize parameters for /tmp/params.json5")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn operation_error_preserves_source() {
        let source = std::io::Error::new(std::io::ErrorKind::Other, "join failed");
        let error = ParameterError::operation("calling remote parameter service", source);

        assert!(
            error
                .to_string()
                .contains("parameter operation failed while calling remote parameter service")
        );
        assert!(std::error::Error::source(&error).is_some());
    }
}

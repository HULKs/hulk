use std::{fmt, path::PathBuf};

use super::{LayerPath, ParameterKey};

#[derive(Debug, Clone)]
pub enum ParameterError {
    FileNotFound { path: PathBuf },
    FileReadError { path: PathBuf, message: String },
    ParseError { path: PathBuf, message: String },
    MergeError { message: String },
    DeserializationError { message: String },
    ValidationError { message: String },
    RevisionMismatch { expected: u64, actual: u64 },
    PersistenceError { path: PathBuf, message: String },
    PathError { path: String, reason: String },
    EmptyLayerList,
    InvalidParameterKey { key: ParameterKey },
    LayerNotActive { layer: LayerPath },
    AlreadyBound { node_fqn: String },
    RemoteError { message: String },
}

pub type Result<T> = std::result::Result<T, ParameterError>;

impl fmt::Display for ParameterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound { path } => {
                write!(f, "parameter file not found: {}", path.display())
            }
            Self::FileReadError { path, message } => {
                write!(
                    f,
                    "failed to read parameter file {}: {message}",
                    path.display()
                )
            }
            Self::ParseError { path, message } => {
                write!(
                    f,
                    "failed to parse parameter file {}: {message}",
                    path.display()
                )
            }
            Self::MergeError { message } => write!(f, "merge error: {message}"),
            Self::DeserializationError { message } => {
                write!(f, "typed parameter deserialization failed: {message}")
            }
            Self::ValidationError { message } => {
                write!(f, "parameter validation failed: {message}")
            }
            Self::RevisionMismatch { expected, actual } => {
                write!(f, "revision mismatch: expected {expected}, actual {actual}")
            }
            Self::PersistenceError { path, message } => {
                write!(f, "failed to persist {}: {message}", path.display())
            }
            Self::PathError { path, reason } => write!(f, "invalid path '{path}': {reason}"),
            Self::EmptyLayerList => write!(f, "parameter layer list must not be empty"),
            Self::InvalidParameterKey { key } => write!(f, "invalid parameter key '{key}'"),
            Self::LayerNotActive { layer } => {
                write!(f, "target layer is not active for this node: {layer}")
            }
            Self::AlreadyBound { node_fqn } => {
                write!(f, "parameters already bound for node {node_fqn}")
            }
            Self::RemoteError { message } => write!(f, "remote parameter error: {message}"),
        }
    }
}

impl std::error::Error for ParameterError {}

impl From<serde_json::Error> for ParameterError {
    fn from(value: serde_json::Error) -> Self {
        Self::DeserializationError {
            message: value.to_string(),
        }
    }
}

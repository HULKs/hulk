use std::{error::Error as StdError, fmt, time::Duration};

use ros_z_protocol::ProtocolError;

use crate::topic_name::TopicNameError;

pub type Result<T> = std::result::Result<T, Error>;

pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Top-level error type returned by fallible `ros-z` APIs.
///
/// Variants group failures by subsystem. Match variants for coarse handling and
/// inspect nested source errors for detailed recovery.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Native key-expression protocol failure.
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    /// Zenoh runtime operation failed.
    #[error("failed to {operation}")]
    Zenoh {
        operation: &'static str,
        #[source]
        source: zenoh::Error,
    },

    /// ROS name qualification failed.
    #[error("failed to qualify {kind} name '{name}'")]
    Name {
        kind: NameKind,
        name: String,
        #[source]
        source: TopicNameError,
    },

    /// Configuration override processing failed.
    #[error(transparent)]
    Config(Box<ConfigError>),

    /// Message payload, schema, or attachment handling failed.
    #[error(transparent)]
    Wire(Box<WireError>),

    /// Service call failed before a successful response was received.
    #[error(transparent)]
    ServiceCall(#[from] ServiceCallError),

    /// Service server was used in a mode that does not support the requested operation.
    #[error("service server cannot {operation}: {reason}")]
    ServiceServerState {
        operation: &'static str,
        reason: &'static str,
    },

    /// Shared-memory setup or buffer conversion failed.
    #[error(transparent)]
    Shm(#[from] ShmError),

    /// Dynamic message, schema, or discovery operation failed.
    #[error(transparent)]
    Dynamic(Box<crate::dynamic::DynamicError>),

    /// Local or remote parameter operation failed.
    #[error(transparent)]
    Parameter(Box<crate::parameter::ParameterError>),
}

impl From<WireError> for Error {
    fn from(source: WireError) -> Self {
        Self::Wire(Box::new(source))
    }
}

impl From<ConfigError> for Error {
    fn from(source: ConfigError) -> Self {
        Self::Config(Box::new(source))
    }
}

impl From<crate::dynamic::DynamicError> for Error {
    fn from(source: crate::dynamic::DynamicError) -> Self {
        Self::Dynamic(Box::new(source))
    }
}

impl From<crate::parameter::ParameterError> for Error {
    fn from(source: crate::parameter::ParameterError) -> Self {
        Self::Parameter(Box::new(source))
    }
}

/// Kind of ROS name being qualified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    /// Topic name.
    Topic,
    /// Service name.
    Service,
    /// Namespace component.
    Namespace,
    /// Node name.
    Node,
}

impl fmt::Display for NameKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Topic => f.write_str("topic"),
            Self::Service => f.write_str("service"),
            Self::Namespace => f.write_str("namespace"),
            Self::Node => f.write_str("node"),
        }
    }
}

/// Errors produced while loading or applying ros-z configuration overrides.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Config override value could not be serialized.
    #[error("failed to serialize config override '{key}'")]
    SerializeOverride {
        key: String,
        #[source]
        source: serde_json::Error,
    },

    /// Config override value could not be rendered as JSON.
    #[error("failed to render config override '{key}' as JSON")]
    RenderOverride {
        key: String,
        #[source]
        source: serde_json::Error,
    },

    /// Environment override value could not be parsed.
    #[error("failed to parse ZENOH_CONFIG_OVERRIDE value for '{key}' ({value})")]
    ParseEnvOverride {
        key: String,
        value: String,
        #[source]
        source: json5::Error,
    },

    /// Environment override did not use the expected `key=value` form.
    #[error("invalid ZENOH_CONFIG_OVERRIDE pair '{pair}'; expected 'key=value'")]
    InvalidEnvOverride { pair: String },
}

/// Errors produced while encoding, decoding, or validating wire data.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    /// Message encoding failed.
    #[error("failed to encode {type_name}")]
    Encode {
        type_name: String,
        #[source]
        source: BoxError,
    },

    /// Message decoding failed.
    #[error("failed to decode payload as {type_name}")]
    Decode {
        type_name: String,
        #[source]
        source: BoxError,
    },

    /// Static schema construction failed.
    #[error("failed to build {endpoint_kind} schema for topic '{topic}'")]
    Schema {
        endpoint_kind: &'static str,
        topic: String,
        #[source]
        source: ros_z_schema::SchemaError,
    },

    /// Dynamic schema construction failed.
    #[error("failed to build {endpoint_kind} schema for topic '{topic}'")]
    DynamicSchema {
        endpoint_kind: &'static str,
        topic: String,
        #[source]
        source: crate::dynamic::DynamicError,
    },

    /// A dynamic payload was used without its schema.
    #[error("dynamic schema is required for topic '{topic}'")]
    MissingDynamicSchema { topic: String },

    /// Publication id attachment was missing.
    #[error("publication id attachment is required for topic '{topic}'")]
    MissingPublicationId { topic: String },

    /// Sample attachment metadata was missing.
    #[error("received ros-z sample without attachment metadata")]
    MissingSampleAttachment,

    /// Sample attachment metadata could not be decoded.
    #[error("failed to decode ros-z attachment metadata")]
    SampleAttachmentDecode {
        #[source]
        source: zenoh::Error,
    },

    /// Service request attachment metadata was missing.
    #[error("received ros-z service request without attachment metadata")]
    MissingServiceRequestAttachment,

    /// Service request attachment metadata could not be decoded.
    #[error("failed to decode ros-z service request attachment metadata")]
    ServiceRequestAttachmentDecode {
        #[source]
        source: zenoh::Error,
    },

    /// Service response attachment metadata was missing.
    #[error("received ros-z service response without attachment metadata")]
    MissingServiceResponseAttachment,

    /// Service response attachment metadata could not be decoded.
    #[error("failed to decode ros-z service response attachment metadata")]
    ServiceResponseAttachmentDecode {
        #[source]
        source: zenoh::Error,
    },

    /// Dynamic payload schema did not match the advertised schema.
    #[error("schema mismatch: dynamic payload schema does not match advertised schema")]
    DynamicSchemaMismatch,
}

/// Errors produced while waiting for or receiving service-call responses.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ServiceCallError {
    /// Service call exceeded its timeout.
    #[error("service call to '{service}' timed out after {timeout:?}")]
    Timeout { service: String, timeout: Duration },

    /// Service call ended without any response.
    #[error("service call to '{service}' ended before any response was received")]
    NoResponse { service: String },

    /// Service call received an error reply.
    #[error("service call to '{service}' received an error reply")]
    Reply {
        service: String,
        #[source]
        source: zenoh::Error,
    },
}

/// Errors produced while using Zenoh shared-memory transport support.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ShmError {
    /// Shared-memory provider creation failed.
    #[error("failed to create shared-memory provider with size {size} bytes")]
    Provider {
        size: usize,
        #[source]
        source: zenoh::Error,
    },

    /// Shared-memory buffer allocation failed.
    #[error("failed to allocate shared-memory buffer with capacity {capacity} bytes")]
    Allocation {
        capacity: usize,
        #[source]
        source: zenoh::Error,
    },

    /// Shared-memory buffer could not be converted into a Zenoh buffer.
    #[error("failed to convert shared-memory buffer into ZBuf")]
    IntoZbuf {
        #[source]
        source: zenoh::Error,
    },
}

impl Error {
    pub(crate) fn zenoh(operation: &'static str, source: zenoh::Error) -> Self {
        Self::Zenoh { operation, source }
    }

    pub(crate) fn service_server_state(operation: &'static str, reason: &'static str) -> Self {
        Self::ServiceServerState { operation, reason }
    }

    pub(crate) fn topic_name(name: impl Into<String>, source: TopicNameError) -> Self {
        Self::Name {
            kind: NameKind::Topic,
            name: name.into(),
            source,
        }
    }

    pub(crate) fn service_name(name: impl Into<String>, source: TopicNameError) -> Self {
        Self::Name {
            kind: NameKind::Service,
            name: name.into(),
            source,
        }
    }

    pub(crate) fn namespace(name: impl Into<String>, source: TopicNameError) -> Self {
        Self::Name {
            kind: NameKind::Namespace,
            name: name.into(),
            source,
        }
    }

    pub(crate) fn node_name(name: impl Into<String>, source: TopicNameError) -> Self {
        Self::Name {
            kind: NameKind::Node,
            name: name.into(),
            source,
        }
    }

    pub(crate) fn encode<E>(type_name: impl Into<String>, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        WireError::Encode {
            type_name: type_name.into(),
            source: Box::new(source),
        }
        .into()
    }

    pub(crate) fn decode<E>(type_name: impl Into<String>, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        WireError::Decode {
            type_name: type_name.into(),
            source: Box::new(source),
        }
        .into()
    }

    pub(crate) fn schema(
        endpoint_kind: &'static str,
        topic: impl Into<String>,
        source: ros_z_schema::SchemaError,
    ) -> Self {
        WireError::Schema {
            endpoint_kind,
            topic: topic.into(),
            source,
        }
        .into()
    }
}

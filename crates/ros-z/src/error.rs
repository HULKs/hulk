use std::{error::Error as StdError, fmt, time::Duration};

use ros_z_protocol::ProtocolError;

use crate::topic_name::TopicNameError;

pub type Result<T> = std::result::Result<T, Error>;

pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error("failed to {operation}")]
    Zenoh {
        operation: &'static str,
        #[source]
        source: zenoh::Error,
    },

    #[error("failed to qualify {kind} name '{name}'")]
    Name {
        kind: NameKind,
        name: String,
        #[source]
        source: TopicNameError,
    },

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Wire(#[from] WireError),

    #[error(transparent)]
    ServiceCall(#[from] ServiceCallError),

    #[error(transparent)]
    Shm(#[from] ShmError),

    #[error(transparent)]
    Dynamic(#[from] crate::dynamic::DynamicError),

    #[error(transparent)]
    Parameter(#[from] crate::parameter::ParameterError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    Topic,
    Service,
    Namespace,
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

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to serialize config override '{key}'")]
    SerializeOverride {
        key: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to render config override '{key}' as JSON")]
    RenderOverride {
        key: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to parse ZENOH_CONFIG_OVERRIDE value for '{key}' ({value})")]
    ParseEnvOverride {
        key: String,
        value: String,
        #[source]
        source: json5::Error,
    },

    #[error("invalid ZENOH_CONFIG_OVERRIDE pair '{pair}'; expected 'key=value'")]
    InvalidEnvOverride { pair: String },
}

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("failed to encode {type_name}")]
    Encode {
        type_name: String,
        #[source]
        source: BoxError,
    },

    #[error("failed to decode payload as {type_name}")]
    Decode {
        type_name: String,
        #[source]
        source: BoxError,
    },

    #[error("failed to build {endpoint_kind} schema for topic '{topic}'")]
    Schema {
        endpoint_kind: &'static str,
        topic: String,
        #[source]
        source: ros_z_schema::SchemaError,
    },

    #[error("failed to build {endpoint_kind} schema for topic '{topic}': {source}")]
    DynamicSchema {
        endpoint_kind: &'static str,
        topic: String,
        #[source]
        source: crate::dynamic::DynamicError,
    },

    #[error("dynamic schema is required for topic '{topic}'")]
    MissingDynamicSchema { topic: String },

    #[error("publication id attachment is required for topic '{topic}'")]
    MissingPublicationId { topic: String },

    #[error("received ros-z sample without attachment metadata")]
    MissingSampleAttachment,

    #[error("failed to decode ros-z attachment metadata")]
    SampleAttachmentDecode {
        #[source]
        source: zenoh::Error,
    },

    #[error("received ros-z service request without attachment metadata")]
    MissingServiceRequestAttachment,

    #[error("failed to decode ros-z service request attachment metadata")]
    ServiceRequestAttachmentDecode {
        #[source]
        source: zenoh::Error,
    },

    #[error("received ros-z service response without attachment metadata")]
    MissingServiceResponseAttachment,

    #[error("failed to decode ros-z service response attachment metadata")]
    ServiceResponseAttachmentDecode {
        #[source]
        source: zenoh::Error,
    },

    #[error("schema mismatch: dynamic payload schema does not match advertised schema")]
    DynamicSchemaMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceCallError {
    #[error("service call to '{service}' timed out after {timeout:?}")]
    Timeout { service: String, timeout: Duration },

    #[error("service call to '{service}' ended before any response was received")]
    NoResponse { service: String },

    #[error("service call to '{service}' received an error reply")]
    Reply {
        service: String,
        #[source]
        source: zenoh::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ShmError {
    #[error("failed to create shared-memory provider with size {size} bytes")]
    Provider {
        size: usize,
        #[source]
        source: zenoh::Error,
    },

    #[error("failed to allocate shared-memory buffer with capacity {capacity} bytes")]
    Allocation {
        capacity: usize,
        #[source]
        source: zenoh::Error,
    },

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

    pub(crate) fn encode<E>(type_name: impl Into<String>, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Wire(WireError::Encode {
            type_name: type_name.into(),
            source: Box::new(source),
        })
    }

    pub(crate) fn decode<E>(type_name: impl Into<String>, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Wire(WireError::Decode {
            type_name: type_name.into(),
            source: Box::new(source),
        })
    }

    pub(crate) fn schema(
        endpoint_kind: &'static str,
        topic: impl Into<String>,
        source: ros_z_schema::SchemaError,
    ) -> Self {
        Self::Wire(WireError::Schema {
            endpoint_kind,
            topic: topic.into(),
            source,
        })
    }
}

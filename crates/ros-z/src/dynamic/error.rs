//! Error types for dynamic message handling.

/// Errors that can occur during dynamic message, schema, and discovery operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum DynamicError {
    /// Type name was empty or otherwise invalid.
    #[error("invalid type name '{0}': expected a non-empty string")]
    InvalidTypeName(String),

    /// Field was not found in the message schema.
    #[error("field '{0}' not found in message schema")]
    FieldNotFound(String),

    /// Field access path was empty.
    #[error("empty path provided for field access")]
    EmptyPath,

    /// Nested field access was attempted on a non-message type.
    #[error("field '{0}' is not a message type, cannot access nested fields")]
    NotAMessage(String),

    /// Field access or conversion found a different type than expected.
    #[error("type mismatch at '{path}': expected type '{expected}'")]
    TypeMismatch { path: String, expected: String },

    /// Field index was out of bounds.
    #[error("field index {0} is out of bounds")]
    IndexOutOfBounds(usize),

    /// Non-CDR serialization error.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Non-CDR deserialization error.
    #[error("deserialization error: {0}")]
    DeserializationError(String),

    /// Schema validation error.
    #[error("schema validation failed while {operation}: {source}")]
    Schema {
        operation: &'static str,
        #[source]
        source: ros_z_schema::SchemaError,
    },

    /// Name qualification error.
    #[error("failed to qualify schema service name while {operation}: {source}")]
    Name {
        operation: &'static str,
        #[source]
        source: crate::topic_name::TopicNameError,
    },

    /// Runtime error from ros-z infrastructure used by dynamic operations.
    #[error("failed to {operation}: {source}")]
    Runtime {
        operation: &'static str,
        #[source]
        source: crate::error::BoxError,
    },

    /// CDR serialization error.
    #[error("CDR serialization error: {source}")]
    Serialization {
        #[source]
        source: ros_z_cdr::Error,
    },

    /// CDR deserialization error.
    #[error("CDR deserialization error: {source}")]
    Deserialization {
        #[source]
        source: ros_z_cdr::Error,
    },

    /// Schema package could not be loaded.
    #[error("schema loading error for package '{package}': {source}")]
    SchemaLoadError {
        package: String,
        #[source]
        source: crate::error::BoxError,
    },

    /// Schema registry lock was poisoned.
    #[error("schema registry lock was poisoned")]
    RegistryLockPoisoned,

    /// Schema was not found in the registry.
    #[error("schema '{0}' not found in registry")]
    SchemaNotFound(String),

    /// Schema service call failed.
    #[error("schema service failed for node '{node}' on service '{service}': {source}")]
    SchemaService {
        node: String,
        service: String,
        #[source]
        source: crate::error::ServiceCallError,
    },

    /// Automatic topic-based schema discovery requires publisher node identity.
    #[error(
        "automatic schema discovery for topic '{topic}' requires publisher node identity, which is unavailable from this backend/discovery format"
    )]
    MissingNodeIdentity { topic: String },

    /// Default value was invalid for the field type.
    #[error("invalid default value for field '{field}': {reason}")]
    InvalidDefaultValue { field: String, reason: String },

    /// Bounded string or sequence exceeded its maximum size.
    #[error("bounded type exceeded maximum size: max={max}, actual={actual}")]
    BoundExceeded { max: usize, actual: usize },
}

impl DynamicError {
    pub(crate) fn schema(operation: &'static str, source: ros_z_schema::SchemaError) -> Self {
        Self::Schema { operation, source }
    }

    pub(crate) fn name(operation: &'static str, source: crate::topic_name::TopicNameError) -> Self {
        Self::Name { operation, source }
    }

    pub(crate) fn runtime(
        operation: &'static str,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Runtime {
            operation,
            source: Box::new(source),
        }
    }

    pub(crate) fn schema_service(
        node: impl Into<String>,
        service: impl Into<String>,
        source: crate::error::ServiceCallError,
    ) -> Self {
        Self::SchemaService {
            node: node.into(),
            service: service.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicError;

    #[test]
    fn string_only_errors_use_generic_serialization_wording() {
        let serialization = DynamicError::SerializationError("topic qualification failed".into());
        let deserialization = DynamicError::DeserializationError("schema shape mismatch".into());

        assert_eq!(
            serialization.to_string(),
            "serialization error: topic qualification failed"
        );
        assert_eq!(
            deserialization.to_string(),
            "deserialization error: schema shape mismatch"
        );
    }

    #[test]
    fn cdr_errors_include_source_in_display_and_error_source() {
        let serialization = DynamicError::Serialization {
            source: ros_z_cdr::Error::UnexpectedEof,
        };
        let deserialization = DynamicError::Deserialization {
            source: ros_z_cdr::Error::UnexpectedEof,
        };

        assert_eq!(
            serialization.to_string(),
            "CDR serialization error: unexpected end of input"
        );
        assert!(std::error::Error::source(&serialization).is_some());
        assert_eq!(
            deserialization.to_string(),
            "CDR deserialization error: unexpected end of input"
        );
        assert!(std::error::Error::source(&deserialization).is_some());
    }

    #[test]
    fn schema_error_preserves_source() {
        let error = DynamicError::schema(
            "registering schema",
            ros_z_schema::SchemaError::BuilderFailed,
        );

        assert!(
            error
                .to_string()
                .contains("schema validation failed while registering schema")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn schema_service_error_preserves_source() {
        let error = DynamicError::schema_service(
            "/vision/object_detection",
            "/vision/object_detection/get_schema",
            crate::error::ServiceCallError::NoResponse {
                service: "/vision/object_detection/get_schema".to_string(),
            },
        );

        let message = error.to_string();
        assert!(message.contains("schema service failed"));
        assert!(message.contains("/vision/object_detection"));
        assert!(std::error::Error::source(&error).is_some());
    }
}

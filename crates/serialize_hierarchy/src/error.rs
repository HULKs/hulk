#[derive(Debug, thiserror::Error)]
pub enum Error<E>
where
    E: std::error::Error,
{
    #[error("failed to serialize")]
    SerializationFailed(E),
    #[error("failed to deserialize")]
    DeserializationFailed(E),
    #[error("type {type_name} does not support serialization for path {path:?}")]
    TypeDoesNotSupportSerialization {
        type_name: &'static str,
        path: String,
    },
    #[error("type {type_name} does not support deserialization for path {path:?}")]
    TypeDoesNotSupportDeserialization {
        type_name: &'static str,
        path: String,
    },
    #[error("unexpected path segment {segment}")]
    UnexpectedPathSegment { segment: String },
}

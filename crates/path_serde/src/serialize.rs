use serde::Serializer;

#[derive(Debug, thiserror::Error)]
pub enum Error<E>
where
    E: std::error::Error,
{
    #[error("failed to serialize")]
    SerializationFailed(#[source] E),
    #[error("type {type_name} does not support serialization for path {path:?}")]
    NotSupported {
        type_name: &'static str,
        path: String,
    },
    #[error("unexpected path {path}")]
    UnexpectedPath { path: String },
}

pub trait PathSerialize {
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer;
}

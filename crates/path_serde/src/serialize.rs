use serde::Serializer;

#[derive(Debug, thiserror::Error)]
pub enum Error<E>
where
    E: std::error::Error,
{
    #[error("failed to serialize")]
    SerializationFailed(#[source] E),
    #[error("path `{path}` does not exist")]
    PathDoesNotExist { path: String },
}

pub trait PathSerialize {
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer;
}

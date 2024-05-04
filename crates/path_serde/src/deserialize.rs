use serde::Deserializer;

#[derive(Debug, thiserror::Error)]
pub enum Error<E>
where
    E: std::error::Error,
{
    #[error("failed to deserialize")]
    DeserializationFailed(#[source] E),
    #[error("path `{path}` does not exist")]
    PathDoesNotExist { path: String },
}

pub trait PathDeserialize {
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>;
}

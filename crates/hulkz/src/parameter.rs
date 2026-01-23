use serde::Deserialize;
use thiserror::Error;
use tokio::fs::read_to_string;

#[derive(Error, Debug)]
pub enum ParameterError {
    #[error("failed to open file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to deserialize TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T, E = ParameterError> = std::result::Result<T, E>;

pub struct Parameters<T> {
    value: T,
}

impl<T> Parameters<T>
where
    for<'de> T: Deserialize<'de>,
{
    pub async fn load() -> Result<Self> {
        let content = read_to_string("parameters.toml").await?;
        let value = toml::from_str(&content)?;

        Ok(Self { value })
    }

    pub async fn get(&self) -> &T {
        &self.value
    }
}

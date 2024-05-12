use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

mod keys;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    Parsing(#[from] toml::de::Error),
}

#[derive(Debug, Deserialize)]
pub struct Configuration {
    pub keys: keys::Keybinds,
}

impl Configuration {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let config_file = std::fs::read_to_string(path)?;

        let configuration: Configuration = toml::from_str(&config_file)?;

        Ok(configuration)
    }
}

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

pub mod keybind_plugin;
pub mod keys;

const DEFAULT_CONFIG: &str = include_str!("../default.toml");

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    Parsing(#[from] toml::de::Error),
}

fn config_path() -> PathBuf {
    let mut result = dirs::config_dir().unwrap();
    result.extend(["hulks", "twix.toml"]);

    result
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Deserialize)]
pub struct Configuration {
    pub keys: keys::Keybinds,
}

impl Configuration {
    pub fn load() -> Result<Self, Error> {
        Self::load_from_file(config_path())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        match std::fs::read_to_string(&path) {
            Ok(config_file) => {
                let mut configuration = Self::default();
                let user_configuration: Configuration = toml::from_str(&config_file)?;

                configuration.merge(user_configuration);

                Ok(configuration)
            }
            Err(error) => {
                log::info!(
                    "Could not load config file at {}: {error}",
                    path.as_ref().display()
                );

                Ok(Self::default())
            }
        }
    }

    pub fn merge(&mut self, other: Self) {
        let Self { keys } = other;

        self.keys.merge(keys);
    }
}

impl Default for Configuration {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{Configuration, DEFAULT_CONFIG};

    #[test]
    fn parse_default_config() {
        toml::from_str::<Configuration>(DEFAULT_CONFIG).expect("failed to parse default.toml");
    }

    #[test]
    fn merge_configs() {
        let mut config_1: Configuration = toml::from_str(
            r#"
                [keys]
                C-a = "focus_left"
                C-S-a = "reconnect"
            "#,
        )
        .unwrap();

        let config_2: Configuration = toml::from_str(
            r#"
                [keys]
                C-b = "focus_left"
                C-A = "focus_right"
            "#,
        )
        .unwrap();

        config_1.merge(config_2);

        assert_eq!(
            config_1,
            toml::from_str(
                r#"
                [keys]
                C-a = "focus_left"
                C-A = "focus_right"
                C-b = "focus_left"
            "#
            )
            .unwrap()
        );
    }
}

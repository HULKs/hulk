pub mod keybind_plugin;
pub mod keys;

use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml::{map::Entry, Value};

const DEFAULT_CONFIG: &str = include_str!("../config_default.toml");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
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
    pub naos: NaoConfig,
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Deserialize)]
pub struct NaoConfig {
    pub lowest: u8,
    pub highest: u8,
}

impl Configuration {
    pub fn load() -> Result<Self, Error> {
        Self::load_from_file(config_path())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        match std::fs::read_to_string(&path) {
            Ok(config_file) => {
                let mut configuration: Value = Self::default();
                let user_configuration: Value = toml::from_str(&config_file)?;

                configuration.update(user_configuration);

                Ok(configuration.try_into()?)
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

    fn default<T: for<'de> Deserialize<'de>>() -> T {
        toml::from_str(DEFAULT_CONFIG).unwrap()
    }
}

trait Update {
    fn update(&mut self, other: Self);
}

impl Update for Value {
    fn update(&mut self, other: Self) {
        match (self, other) {
            (Value::Table(self_table), Value::Table(other_table)) => {
                for (key, value) in other_table {
                    match self_table.entry(key) {
                        Entry::Vacant(entry) => {
                            entry.insert(value);
                        }
                        Entry::Occupied(entry) => {
                            entry.into_mut().update(value);
                        }
                    }
                }
            }
            (other_self, other) => {
                *other_self = other;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::configuration::Update;

    use super::{Configuration, DEFAULT_CONFIG};

    #[test]
    fn parse_default_config() {
        toml::from_str::<Configuration>(DEFAULT_CONFIG).expect("failed to parse default.toml");
    }

    #[test]
    fn merge_configs() {
        let mut config_1: toml::Value = toml::from_str(
            r#"
                [keys]
                C-a = "focus_left"
                C-S-a = "reconnect"

                [naos]
                lowest = 1
                highest = 2
            "#,
        )
        .unwrap();

        let config_2: toml::Value = toml::from_str(
            r#"
                [keys]
                C-b = "focus_left"
                C-A = "focus_right"

                [naos]
                lowest = 3
                highest = 4
            "#,
        )
        .unwrap();

        config_1.update(config_2);

        assert_eq!(
            config_1,
            toml::from_str(
                r#"
                    [keys]
                    C-a = "focus_left"
                    C-S-a = "reconnect"
                    C-A = "focus_right"
                    C-b = "focus_left"

                    [naos]
                    lowest = 3
                    highest = 4
                "#
            )
            .unwrap()
        );
    }

    #[test]
    fn merge_partial_config() {
        let mut default_config: toml::Value = toml::from_str(
            r#"
                [keys]
                C-a = "focus_left"
                C-S-a = "reconnect"

                [naos]
                lowest = 1
                highest = 2
            "#,
        )
        .unwrap();

        let user_config: toml::Value = toml::from_str(
            r#"
                [keys]
                C-a = "focus_right"
            "#,
        )
        .unwrap();

        default_config.update(user_config);

        assert_eq!(
            default_config,
            toml::from_str(
                r#"
                    [keys]
                    C-a = "focus_right"
                    C-S-a = "reconnect"

                    [naos]
                    lowest = 1
                    highest = 2
                "#,
            )
            .unwrap()
        );
    }
}

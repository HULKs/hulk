pub mod keybind_plugin;
pub mod keys;
pub mod merge;

use std::path::{Path, PathBuf};

use merge::Merge;
use serde::Deserialize;

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

#[derive(Debug, Deserialize)]
pub struct RawNaoConfig {
    pub lowest: Option<u8>,
    pub highest: Option<u8>,
}

impl Merge<RawNaoConfig> for NaoConfig {
    fn merge(&mut self, other: RawNaoConfig) {
        self.lowest.merge(other.lowest);
        self.highest.merge(other.highest);
    }
}

impl Merge<NaoConfig> for NaoConfig {
    fn merge(&mut self, other: NaoConfig) {
        *self = other;
    }
}

#[derive(Debug, Deserialize)]
pub struct RawConfiguration {
    pub keys: Option<keys::Keybinds>,
    pub naos: Option<RawNaoConfig>,
}

impl Configuration {
    pub fn load() -> Result<Self, Error> {
        Self::load_from_file(config_path())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        match std::fs::read_to_string(&path) {
            Ok(config_file) => {
                let mut configuration = Self::default();
                let user_configuration: RawConfiguration = toml::from_str(&config_file)?;

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
}

impl Merge<RawConfiguration> for Configuration {
    fn merge(&mut self, other: RawConfiguration) {
        self.keys.merge(other.keys);
        self.naos.merge(other.naos);
    }
}

impl Merge<Configuration> for Configuration {
    fn merge(&mut self, other: Configuration) {
        self.keys.merge(other.keys);
        self.naos.merge(other.naos);
    }
}

impl Default for Configuration {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::configuration::merge::Merge;

    use super::{Configuration, RawConfiguration, DEFAULT_CONFIG};

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

                [naos]
                lowest = 1
                highest = 2
            "#,
        )
        .unwrap();

        let config_2: Configuration = toml::from_str(
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

        config_1.merge(config_2);

        assert_eq!(
            config_1,
            toml::from_str(
                r#"
                    [keys]
                    C-a = "focus_left"
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
        let mut default_config: Configuration = toml::from_str(
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

        let user_config: RawConfiguration = toml::from_str(
            r#"
                [keys]
                C-a = "focus_right"
            "#,
        )
        .unwrap();

        default_config.merge(user_config);

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

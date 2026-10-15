pub mod keybind_plugin;
pub mod keys;

use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml::{Value, map::Entry};

const DEFAULT_CONFIG: &str = r#"
[keys]
C-t = "open_split"
C-T = "open_tab"

C-o = "focus_namespace"
C-p = "focus_panel"
C-f = "focus_topic"

C-h = "focus_left"
C-j = "focus_below"
C-k = "focus_above"
C-l = "focus_right"

C-Up = "focus_above"
C-Down = "focus_below"
C-Left = "focus_left"
C-Right = "focus_right"

C-w = "close_tab"
C-d = "duplicate_tab"

C-S-Backspace = "close_all"

Space = "timeline_toggle_playback"
ArrowLeft = "timeline_step_backward"
ArrowRight = "timeline_step_forward"
S-ArrowLeft = "timeline_fine_step_backward"
S-ArrowRight = "timeline_fine_step_forward"
Comma = "timeline_nudge_backward"
Period = "timeline_nudge_forward"
Home = "timeline_start"
End = "timeline_end"
f = "timeline_frame_all"
"#;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    Parsing(#[from] toml::de::Error),
}

fn config_path() -> PathBuf {
    let mut result = dirs::config_dir().unwrap();
    result.extend(["hulks", "twix-ros-z.toml"]);

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
                let mut configuration: Value = toml::from_str(DEFAULT_CONFIG).unwrap();
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
}

impl Default for Configuration {
    fn default() -> Self {
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
    use super::{
        Configuration, DEFAULT_CONFIG, Update, config_path,
        keys::{KeybindAction, parse_shortcut},
    };

    #[test]
    fn parse_default_config() {
        toml::from_str::<Configuration>(DEFAULT_CONFIG).expect("failed to parse default.toml");
    }

    #[test]
    fn config_path_uses_ros_z_specific_file() {
        assert_eq!(
            config_path().file_name().and_then(|name| name.to_str()),
            Some("twix-ros-z.toml")
        );
    }

    #[test]
    fn default_config_contains_only_current_ros_z_actions() {
        let config: toml::Value = toml::from_str(DEFAULT_CONFIG).unwrap();
        let keys = config
            .get("keys")
            .and_then(toml::Value::as_table)
            .expect("default config should contain keys");

        assert_eq!(
            keys.get("C-o").and_then(toml::Value::as_str),
            Some("focus_namespace")
        );
        assert_eq!(
            keys.get("C-f").and_then(toml::Value::as_str),
            Some("focus_topic")
        );
        assert!(
            !keys
                .values()
                .any(|action| action.as_str() == Some("reconnect"))
        );
        assert!(config.get("robots").is_none());
    }

    #[test]
    fn merge_configs() {
        let mut config_1: toml::Value = toml::from_str(
            r#"
                [keys]
                C-a = "focus_left"
                C-S-a = "focus_namespace"
            "#,
        )
        .unwrap();

        let config_2: toml::Value = toml::from_str(
            r#"
                [keys]
                C-b = "focus_left"
                C-A = "focus_right"
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
                    C-S-a = "focus_namespace"
                    C-A = "focus_right"
                    C-b = "focus_left"
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
                C-S-a = "focus_namespace"
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
                    C-S-a = "focus_namespace"
                "#,
            )
            .unwrap()
        );
    }

    #[test]
    fn timeline_bindings_can_be_overridden_without_losing_other_defaults() {
        let mut config: toml::Value = toml::from_str(DEFAULT_CONFIG).unwrap();
        config.update(
            toml::from_str(
                r#"
            [keys]
            Space = "no_op"
            p = "timeline_toggle_playback"
        "#,
            )
            .unwrap(),
        );
        let config: Configuration = config.try_into().unwrap();
        for (key, action) in [
            ("Space", KeybindAction::NoOp),
            ("p", KeybindAction::TimelineTogglePlayback),
            ("ArrowRight", KeybindAction::TimelineStepForward),
        ] {
            assert!(config.keys.iter().any(|(trigger, configured)| *trigger
                == parse_shortcut(key).unwrap()
                && *configured == action));
        }
    }
}

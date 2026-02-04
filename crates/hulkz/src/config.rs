//! Layered configuration loading for parameters.
//!
//! Configuration files use JSON5 format with the following structure:
//!
//! ```json5
//! {
//!   // Global scope - accessible fleet-wide
//!   "/": {
//!     fleet_id: "alpha",
//!   },
//!   
//!   // Local scope - robot-wide (flat keys)
//!   max_speed: 1.5,
//!   battery_threshold: 0.2,
//!   
//!   // Private scope - nested under ~nodename
//!   "~navigation": {
//!     debug_level: 2,
//!   },
//! }
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::error::{Error, Result};

/// Environment variable for specifying parameter file paths (colon-separated).
pub const PARAMETERS_ENV: &str = "HULKZ_PARAMETERS";

/// Default parameter file name.
pub const DEFAULT_PARAMETERS_FILE: &str = "parameters.json5";

/// Loaded and merged configuration from multiple sources.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Global parameters (from "/" key)
    global: HashMap<String, Value>,
    /// Local parameters (flat keys, no prefix)
    local: HashMap<String, Value>,
    /// Private parameters per node (from "~nodename" keys)
    private: HashMap<String, HashMap<String, Value>>,
}

impl Config {
    /// Creates an empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads configuration from the files specified in HULKZ_PARAMETERS environment variable
    /// (colon-separated), falling back to the default file (parameters.json5) if it exists and no
    /// env var is set
    ///
    /// Files are layered in order, with later files overriding earlier ones.
    pub async fn load_default() -> Result<Self> {
        let mut config = Config::new();

        if let Ok(env_paths) = std::env::var(PARAMETERS_ENV) {
            // Load from environment variable (colon-separated paths)
            for path in env_paths.split(':').filter(|p| !p.is_empty()) {
                config.load_file(path).await?;
            }
        } else {
            // Try default file if it exists
            let default_path = Path::new(DEFAULT_PARAMETERS_FILE);
            if default_path.exists() {
                config.load_file(default_path).await?;
            }
        }

        Ok(config)
    }

    /// Loads and merges a configuration file.
    ///
    /// Later loads override earlier values for the same keys.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConfigFileIo`] if the file cannot be read, with the file path included in
    /// the error message.
    pub async fn load_file(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let content =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|source| Error::ConfigFileIo {
                    path: path.to_path_buf(),
                    source,
                })?;
        self.merge_json5(&content)?;
        Ok(())
    }

    /// Merges JSON5 content into this configuration.
    fn merge_json5(&mut self, content: &str) -> Result<()> {
        let root: Value = json5::from_str(content)?;
        let obj = root
            .as_object()
            .ok_or_else(|| Error::ConfigParse("root must be an object".into()))?;

        for (key, value) in obj {
            if key == "/" {
                // Global parameters
                let global_obj = value
                    .as_object()
                    .ok_or_else(|| Error::ConfigParse("\"/\" must be an object".into()))?;
                for (k, v) in global_obj {
                    self.global.insert(k.clone(), v.clone());
                }
            } else if let Some(node_name) = key.strip_prefix('~') {
                // Private parameters for a node
                let private_obj = value.as_object().ok_or_else(|| {
                    Error::ConfigParse(format!("\"~{node_name}\" must be an object"))
                })?;
                let node_params = self.private.entry(node_name.to_string()).or_default();
                for (k, v) in private_obj {
                    node_params.insert(k.clone(), v.clone());
                }
            } else {
                // Local parameters (flat)
                self.local.insert(key.clone(), value.clone());
            }
        }

        Ok(())
    }

    /// Gets a global parameter value.
    pub fn get_global(&self, name: &str) -> Option<&Value> {
        self.global.get(name)
    }

    /// Gets a local parameter value.
    pub fn get_local(&self, name: &str) -> Option<&Value> {
        self.local.get(name)
    }

    /// Gets a private parameter value for a specific node.
    pub fn get_private(&self, node: &str, name: &str) -> Option<&Value> {
        self.private.get(node).and_then(|m| m.get(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_global_params() {
        let mut config = Config::new();
        config
            .merge_json5(
                r#"{
                "/": {
                    fleet_id: "alpha",
                    emergency: false,
                }
            }"#,
            )
            .unwrap();

        assert_eq!(
            config.get_global("fleet_id"),
            Some(&Value::String("alpha".into()))
        );
        assert_eq!(config.get_global("emergency"), Some(&Value::Bool(false)));
    }

    #[test]
    fn parse_local_params() {
        let mut config = Config::new();
        config
            .merge_json5(
                r#"{
                max_speed: 1.5,
                enabled: true,
            }"#,
            )
            .unwrap();

        assert_eq!(
            config.get_local("max_speed"),
            Some(&Value::Number(serde_json::Number::from_f64(1.5).unwrap()))
        );
        assert_eq!(config.get_local("enabled"), Some(&Value::Bool(true)));
    }

    #[test]
    fn parse_private_params() {
        let mut config = Config::new();
        config
            .merge_json5(
                r#"{
                "~navigation": {
                    debug_level: 2,
                },
                "~vision": {
                    exposure: 100,
                },
            }"#,
            )
            .unwrap();

        assert_eq!(
            config.get_private("navigation", "debug_level"),
            Some(&Value::Number(2.into()))
        );
        assert_eq!(
            config.get_private("vision", "exposure"),
            Some(&Value::Number(100.into()))
        );
        assert_eq!(config.get_private("navigation", "exposure"), None);
    }

    #[test]
    fn layered_override() {
        let mut config = Config::new();

        // Base config
        config
            .merge_json5(
                r#"{
                max_speed: 1.0,
                timeout: 30,
            }"#,
            )
            .unwrap();

        // Override layer
        config
            .merge_json5(
                r#"{
                max_speed: 2.0,
            }"#,
            )
            .unwrap();

        // max_speed overridden, timeout preserved
        assert_eq!(
            config.get_local("max_speed"),
            Some(&Value::Number(serde_json::Number::from_f64(2.0).unwrap()))
        );
        assert_eq!(config.get_local("timeout"), Some(&Value::Number(30.into())));
    }

    #[test]
    fn mixed_scopes() {
        let mut config = Config::new();
        config
            .merge_json5(
                r#"{
                "/": { fleet_id: "beta" },
                max_speed: 1.5,
                "~nav": { lookahead: 0.5 },
            }"#,
            )
            .unwrap();

        assert_eq!(
            config.get_global("fleet_id"),
            Some(&Value::String("beta".into()))
        );
        assert_eq!(
            config.get_local("max_speed"),
            Some(&Value::Number(serde_json::Number::from_f64(1.5).unwrap()))
        );
        assert_eq!(
            config.get_private("nav", "lookahead"),
            Some(&Value::Number(serde_json::Number::from_f64(0.5).unwrap()))
        );
    }
}

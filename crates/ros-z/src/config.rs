//! ros-z Zenoh configuration builders
//!
//! Generates ros-z Zenoh configs programmatically.
//! All configurations are stored as compile-time constants using LazyLock
//! for zero-cost abstraction after first access.
//!
//! # Architecture
//! - Common overrides: 10 settings shared between router and session
//! - Router-specific: 5 settings unique to router mode
//! - Session-specific: 6 settings unique to peer mode
//!
//! # Example
//! ```no_run
//! # use ros_z::config::{RouterConfigBuilder, router_config, session_config};
//! # #[tokio::main]
//! # async fn main() -> zenoh::Result<()> {
//! // Create router config
//! let router_cfg = router_config()?;
//! let router = zenoh::open(router_cfg).await?;
//!
//! // Create session config
//! let session_cfg = session_config()?;
//! let session = zenoh::open(session_cfg).await?;
//!
//! // Customize router port
//! let custom_router = RouterConfigBuilder::new()
//!     .with_listen_port(7448)
//!     .build_config()?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

/// A single configuration override with key, value, and documentation
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigOverride {
    /// Configuration key path (e.g., "transport/link/tx/lease")
    pub key: &'static str,
    /// JSON value to set
    pub value: Value,
    /// Human-readable explanation of why this override exists
    pub reason: &'static str,
}

/// Common overrides shared between router and session configs (10 settings)
fn common_overrides() -> &'static [ConfigOverride] {
    static COMMON: LazyLock<Vec<ConfigOverride>> = LazyLock::new(|| {
        vec![
            ConfigOverride {
                key: "scouting/multicast/enabled",
                value: serde_json::json!(false),
                reason: "Disable multicast discovery - use TCP gossip instead",
            },
            ConfigOverride {
                key: "scouting/gossip/target",
                value: serde_json::json!({"router": ["router", "peer"], "peer": ["router"]}),
                reason: "Peers send gossip only to router (not to other peers) to minimize traffic at launch",
            },
            ConfigOverride {
                key: "timestamping/enabled",
                value: serde_json::json!({"router": true, "peer": true, "client": true}),
                reason: "Enable timestamping for peers and clients (required for transient_local durability)",
            },
            ConfigOverride {
                key: "transport/unicast/open_timeout",
                value: serde_json::json!(60000),
                reason: "Increased from 10s to 60s to avoid timeout when opening links with many nodes",
            },
            ConfigOverride {
                key: "transport/unicast/accept_timeout",
                value: serde_json::json!(60000),
                reason: "Increased from 10s to 60s to avoid timeout when accepting links with many nodes",
            },
            ConfigOverride {
                key: "transport/unicast/accept_pending",
                value: serde_json::json!(10000),
                reason: "Increased from 100 to 10000 to handle many simultaneous connection handshakes",
            },
            ConfigOverride {
                key: "transport/unicast/max_sessions",
                value: serde_json::json!(10000),
                reason: "Increased from 1000 to 10000 to support large number of concurrent sessions",
            },
            ConfigOverride {
                key: "transport/link/tx/lease",
                value: serde_json::json!(60000),
                reason: "Increased from 10s to 60s to avoid lease expiration at launch with many nodes",
            },
            ConfigOverride {
                key: "transport/link/tx/keep_alive",
                value: serde_json::json!(2),
                reason: "Decreased from 4 to 2 for loopback where packet loss is minimal",
            },
            ConfigOverride {
                key: "transport/shared_memory/enabled",
                value: serde_json::json!(false),
                reason: "Disabled by default until fully tested in production deployments",
            },
        ]
    });

    &COMMON
}

/// Router-specific overrides (5 settings)
fn router_specific_overrides() -> &'static [ConfigOverride] {
    static ROUTER_SPECIFIC: LazyLock<Vec<ConfigOverride>> = LazyLock::new(|| {
        vec![
            ConfigOverride {
                key: "mode",
                value: serde_json::json!("router"),
                reason: "Router mode required for ros-z discovery and routing",
            },
            ConfigOverride {
                key: "listen/endpoints",
                value: serde_json::json!(["tcp/[::]:7447"]),
                reason: "Default ros-z router port 7447, IPv6 wildcard for all interfaces",
            },
            ConfigOverride {
                key: "connect/endpoints",
                value: serde_json::json!([]),
                reason: "Router does not connect to other endpoints (empty list)",
            },
            ConfigOverride {
                key: "routing/router/peers_failover_brokering",
                value: serde_json::json!(false),
                reason: "Changed from true to false - unnecessary when peers connect directly, reduces overhead",
            },
            ConfigOverride {
                key: "transport/link/tx/queue/congestion_control/block/wait_before_close",
                value: serde_json::json!(5000000),
                reason: "Keep at 5s (vs 60s for session) - router routes to WiFi, lower value prevents long blocks",
            },
        ]
    });

    &ROUTER_SPECIFIC
}

/// Session-specific overrides (6 settings)
fn session_specific_overrides() -> &'static [ConfigOverride] {
    static SESSION_SPECIFIC: LazyLock<Vec<ConfigOverride>> = LazyLock::new(|| {
        vec![
            ConfigOverride {
                key: "mode",
                value: serde_json::json!("peer"),
                reason: "Peer mode for ros-z nodes - connects to router for discovery and routing",
            },
            ConfigOverride {
                key: "connect/endpoints",
                value: serde_json::json!(["tcp/localhost:7447"]),
                reason: "Connect to Zenoh router on localhost at the default ros-z port 7447",
            },
            ConfigOverride {
                key: "listen/endpoints",
                value: serde_json::json!(["tcp/localhost:0"]),
                reason: "Accept connections only from localhost - external traffic routed via router",
            },
            ConfigOverride {
                key: "scouting/gossip/autoconnect_strategy",
                value: serde_json::json!({"peer": {"to_router": "always", "to_peer": "greater-zid"}}),
                reason: "Changed peer-to-peer from 'always' to 'greater-zid' to avoid redundant connections on loopback",
            },
            ConfigOverride {
                key: "queries_default_timeout",
                value: serde_json::json!(60000),
                reason: "Increased from 10s to 60s to handle slow service servers at launch",
            },
            ConfigOverride {
                key: "transport/link/tx/queue/congestion_control/block/wait_before_close",
                value: serde_json::json!(60000000),
                reason: "Increased from 5s to 60s to avoid premature link closure during launch congestion on loopback",
            },
        ]
    });

    &SESSION_SPECIFIC
}

/// Complete router configuration (cached statically)
///
/// Returns a Vec containing all router-specific and common overrides.
/// The Vec is cloned from static storage for modification if needed.
pub fn router_overrides() -> Vec<ConfigOverride> {
    let mut overrides =
        Vec::with_capacity(router_specific_overrides().len() + common_overrides().len());
    overrides.extend_from_slice(router_specific_overrides());
    overrides.extend_from_slice(common_overrides());
    overrides
}

/// Complete session configuration (cached statically)
///
/// Returns a Vec containing all session-specific and common overrides.
/// The Vec is cloned from static storage for modification if needed.
pub fn session_overrides() -> Vec<ConfigOverride> {
    let mut overrides =
        Vec::with_capacity(session_specific_overrides().len() + common_overrides().len());
    overrides.extend_from_slice(session_specific_overrides());
    overrides.extend_from_slice(common_overrides());
    overrides
}
/// Build a Zenoh config from a set of overrides
fn build_config(overrides: &[ConfigOverride]) -> zenoh::Result<zenoh::Config> {
    let mut config = zenoh::Config::default();
    for override_ in overrides {
        let value_str = serde_json::to_string(&override_.value)?;
        config.insert_json5(override_.key, &value_str)?;
    }
    Ok(config)
}

/// Create a router configuration for ros-z defaults
///
/// # Example
/// ```no_run
/// # use ros_z::config::router_config;
/// # #[tokio::main]
/// # async fn main() -> zenoh::Result<()> {
/// let config = router_config()?;
/// let router = zenoh::open(config).await?;
/// # Ok(())
/// # }
/// ```
pub fn router_config() -> zenoh::Result<zenoh::Config> {
    build_config(&router_overrides())
}

/// Create a session configuration for ros-z defaults
///
/// # Example
/// ```no_run
/// # use ros_z::config::session_config;
/// # #[tokio::main]
/// # async fn main() -> zenoh::Result<()> {
/// let config = session_config()?;
/// let session = zenoh::open(config).await?;
/// # Ok(())
/// # }
/// ```
pub fn session_config() -> zenoh::Result<zenoh::Config> {
    build_config(&session_overrides())
}

/// Build-time JSON5 generator with comments
///
/// Generates a JSON5 file with inline comments explaining each override.
/// Converts path notation (e.g., "connect/endpoints") into nested JSON structure.
/// Useful for generating reference configuration files.
///
/// # Example
/// ```no_run
/// # use ros_z::config::{generate_json5, router_overrides};
/// let json5 = generate_json5(&router_overrides(), "Router Config");
/// std::fs::write("router_config.json5", json5)?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn generate_json5(overrides: &[ConfigOverride], name: &str) -> String {
    use serde_json::Value as JsonValue;
    use std::collections::BTreeMap;

    let mut output = format!("// GENERATED: {} - DO NOT EDIT\n", name);
    output.push_str("// This file is auto-generated from ros-z/src/config.rs\n");
    output.push_str("// Edit the source file and rebuild to make changes\n");

    // Build nested structure from path notation
    let mut root = JsonValue::Object(serde_json::Map::new());
    let mut comments: BTreeMap<String, String> = BTreeMap::new();

    for override_ in overrides {
        let path_parts: Vec<&str> = override_.key.split('/').collect();

        // Store comment at the full path
        comments.insert(override_.key.to_string(), override_.reason.to_string());

        // Navigate/create nested structure
        let mut current = &mut root;
        for (i, part) in path_parts.iter().enumerate() {
            if i == path_parts.len() - 1 {
                // Last part - insert the value
                if let JsonValue::Object(map) = current {
                    map.insert(part.to_string(), override_.value.clone());
                }
            } else {
                // Intermediate part - ensure object exists
                if let JsonValue::Object(map) = current {
                    current = map
                        .entry(part.to_string())
                        .or_insert_with(|| JsonValue::Object(serde_json::Map::new()));
                }
            }
        }
    }

    // Generate JSON5 with comments
    output.push_str(&generate_json5_with_comments(&root, &comments, "", 0));
    output
}

/// Helper function to recursively generate JSON5 with inline comments
fn generate_json5_with_comments(
    value: &serde_json::Value,
    comments: &std::collections::BTreeMap<String, String>,
    current_path: &str,
    indent_level: usize,
) -> String {
    use serde_json::Value as JsonValue;

    let indent = "  ".repeat(indent_level);
    let mut output = String::new();

    match value {
        JsonValue::Object(map) => {
            output.push_str("{\n");
            let entries: Vec<_> = map.iter().collect();
            for (i, (key, val)) in entries.iter().enumerate() {
                let new_path = if current_path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}/{}", current_path, key)
                };

                // Add comment if exists for this path
                if let Some(comment) = comments.get(&new_path) {
                    output.push_str(&format!("{}  // {}\n", indent, comment));
                }

                output.push_str(&format!("{}  \"{}\": ", indent, key));

                let nested =
                    generate_json5_with_comments(val, comments, &new_path, indent_level + 1);
                // Trim the nested output if it's a simple value
                let nested_trimmed = if matches!(val, JsonValue::Object(_) | JsonValue::Array(_)) {
                    nested
                } else {
                    nested.trim().to_string()
                };
                output.push_str(&nested_trimmed);

                if i < entries.len() - 1 {
                    output.push_str(",\n");
                } else {
                    output.push('\n');
                }
            }
            output.push_str(&format!("{}}}", indent));
        }
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                output.push_str("[]");
            } else if arr
                .iter()
                .all(|v| !matches!(v, JsonValue::Object(_) | JsonValue::Array(_)))
            {
                // Inline simple arrays
                output.push('[');
                for (i, item) in arr.iter().enumerate() {
                    output.push_str(&serde_json::to_string(item).unwrap());
                    if i < arr.len() - 1 {
                        output.push_str(", ");
                    }
                }
                output.push(']');
            } else {
                // Multi-line for complex arrays
                output.push_str("[\n");
                for (i, item) in arr.iter().enumerate() {
                    output.push_str(&format!("{}  ", indent));
                    output.push_str(
                        generate_json5_with_comments(
                            item,
                            comments,
                            current_path,
                            indent_level + 1,
                        )
                        .trim(),
                    );
                    if i < arr.len() - 1 {
                        output.push(',');
                    }
                    output.push('\n');
                }
                output.push_str(&format!("{}]", indent));
            }
        }
        other => {
            output.push_str(&serde_json::to_string_pretty(other).unwrap());
        }
    }

    output
}

/// Builder for router configuration with customization options
#[derive(Clone)]
pub struct RouterConfigBuilder {
    overrides: Vec<ConfigOverride>,
}

impl RouterConfigBuilder {
    /// Create a new router config builder with default Zenoh settings
    pub fn new() -> Self {
        Self {
            overrides: router_overrides(),
        }
    }

    /// Change the listen port (default: 7447)
    ///
    /// # Example
    /// ```rust
    /// # use ros_z::config::RouterConfigBuilder;
    /// let config = RouterConfigBuilder::new()
    ///     .with_listen_port(7448)
    ///     .build_config()?;
    /// # Ok::<(), zenoh::Error>(())
    /// ```
    pub fn with_listen_port(mut self, port: u16) -> Self {
        if let Some(listen) = self
            .overrides
            .iter_mut()
            .find(|o| o.key == "listen/endpoints")
        {
            listen.value = serde_json::json!([format!("tcp/[::]:{}", port)]);
        }
        self
    }

    /// Set a custom listen endpoint
    ///
    /// # Example
    /// ```rust
    /// # use ros_z::config::RouterConfigBuilder;
    /// let config = RouterConfigBuilder::new()
    ///     .with_listen_endpoint("tcp/0.0.0.0:7447")
    ///     .build_config()?;
    /// # Ok::<(), zenoh::Error>(())
    /// ```
    pub fn with_listen_endpoint(mut self, endpoint: &str) -> Self {
        if let Some(listen) = self
            .overrides
            .iter_mut()
            .find(|o| o.key == "listen/endpoints")
        {
            listen.value = serde_json::json!([endpoint]);
        }
        self
    }

    /// Override a specific config key
    ///
    /// Replaces existing override if key exists, otherwise adds new one.
    ///
    /// # Example
    /// ```rust
    /// # use ros_z::config::RouterConfigBuilder;
    /// let config = RouterConfigBuilder::new()
    ///     .with_override(
    ///         "transport/unicast/max_sessions",
    ///         serde_json::json!(20000),
    ///         "Custom increased sessions"
    ///     )
    ///     .build_config()?;
    /// # Ok::<(), zenoh::Error>(())
    /// ```
    pub fn with_override(mut self, key: &'static str, value: Value, reason: &'static str) -> Self {
        if let Some(existing) = self.overrides.iter_mut().find(|o| o.key == key) {
            existing.value = value;
            existing.reason = reason;
        } else {
            self.overrides.push(ConfigOverride { key, value, reason });
        }
        self
    }

    /// Build the Zenoh config.
    ///
    pub fn build_config(self) -> zenoh::Result<zenoh::Config> {
        build_config(&self.overrides)
    }
}

impl Default for RouterConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for session configuration with customization options
#[derive(Clone)]
pub struct SessionConfigBuilder {
    overrides: Vec<ConfigOverride>,
}

impl SessionConfigBuilder {
    /// Create a new session config builder with default Zenoh settings
    pub fn new() -> Self {
        Self {
            overrides: session_overrides(),
        }
    }

    /// Change the router endpoint to connect to (default: tcp/localhost:7447)
    ///
    /// # Example
    /// ```rust
    /// # use ros_z::config::SessionConfigBuilder;
    /// let config = SessionConfigBuilder::new()
    ///     .with_router_endpoint("tcp/192.168.1.100:7447")
    ///     .build_config()?;
    /// # Ok::<(), zenoh::Error>(())
    /// ```
    pub fn with_router_endpoint(mut self, endpoint: &str) -> Self {
        if let Some(connect) = self
            .overrides
            .iter_mut()
            .find(|o| o.key == "connect/endpoints")
        {
            connect.value = serde_json::json!([endpoint]);
        }
        self
    }

    /// Override a specific config key
    ///
    /// Replaces existing override if key exists, otherwise adds new one.
    ///
    /// # Example
    /// ```rust
    /// # use ros_z::config::SessionConfigBuilder;
    /// let config = SessionConfigBuilder::new()
    ///     .with_override(
    ///         "queries_default_timeout",
    ///         serde_json::json!(120000),
    ///         "Increased timeout for slow network"
    ///     )
    ///     .build_config()?;
    /// # Ok::<(), zenoh::Error>(())
    /// ```
    pub fn with_override(mut self, key: &'static str, value: Value, reason: &'static str) -> Self {
        if let Some(existing) = self.overrides.iter_mut().find(|o| o.key == key) {
            existing.value = value;
            existing.reason = reason;
        } else {
            self.overrides.push(ConfigOverride { key, value, reason });
        }
        self
    }

    /// Build the Zenoh config.
    pub fn build_config(self) -> zenoh::Result<zenoh::Config> {
        build_config(&self.overrides)
    }
}

impl Default for SessionConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_router_config_creates_valid_session() {
        // Use port 0 to let OS assign an available port
        let config = RouterConfigBuilder::new()
            .with_listen_endpoint("tcp/[::]:0")
            .build_config()
            .expect("Failed to build router config");

        let session = zenoh::open(config).await;
        assert!(
            session.is_ok(),
            "Failed to create Zenoh session with router config: {:?}",
            session.err()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_session_config_creates_valid_session() {
        let config = session_config().expect("Failed to build session config");

        let session = zenoh::open(config).await;
        assert!(
            session.is_ok(),
            "Failed to create Zenoh session with peer config: {:?}",
            session.err()
        );
    }

    #[test]
    fn test_all_overrides_produce_valid_config() {
        // Test router overrides
        for override_ in &router_overrides() {
            let mut config = zenoh::Config::default();
            let value_str = serde_json::to_string(&override_.value).unwrap();
            let result = config.insert_json5(override_.key, &value_str);
            assert!(
                result.is_ok(),
                "Router override '{}' is invalid: {:?}",
                override_.key,
                result.err()
            );
        }

        // Test session overrides
        for override_ in &session_overrides() {
            let mut config = zenoh::Config::default();
            let value_str = serde_json::to_string(&override_.value).unwrap();
            let result = config.insert_json5(override_.key, &value_str);
            assert!(
                result.is_ok(),
                "Session override '{}' is invalid: {:?}",
                override_.key,
                result.err()
            );
        }
    }

    #[test]
    fn test_router_builder_custom_port() {
        let config = RouterConfigBuilder::new()
            .with_listen_port(7448)
            .build_config()
            .expect("Failed to build router config");

        assert_eq!(config.mode().unwrap().to_string(), "router");
    }

    #[test]
    fn test_session_builder_custom_endpoint() {
        let config = SessionConfigBuilder::new()
            .with_router_endpoint("tcp/192.168.1.1:7447")
            .build_config()
            .expect("Failed to build session config");

        assert_eq!(config.mode().unwrap().to_string(), "peer");
    }

    #[test]
    fn test_builder_with_custom_override() {
        let config = RouterConfigBuilder::new()
            .with_override(
                "transport/unicast/max_sessions",
                serde_json::json!(20000),
                "Custom increased sessions",
            )
            .build_config()
            .expect("Failed to build");

        assert_eq!(config.mode().unwrap().to_string(), "router");
    }
}

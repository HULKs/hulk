//! Resolution of layered Zenoh session configuration.

use std::{env, fs, path::Path};

use serde_json::{Value, json};
use zenoh::Config;

use crate::{Result, error::ConfigError};

fn base() -> Value {
    json!({
        "timestamping": { "enabled": true },
        "transport": { "shared_memory": { "enabled": false } },
    })
}

pub(crate) fn session_config(overlay: Option<&Path>) -> Result<Config> {
    let file = env::var_os("ZENOH_SESSION_CONFIG_URI");
    let gateway = env::var_os("ROS_Z_GATEWAY");
    let gateway = gateway
        .as_deref()
        .map(|s| s.to_str().ok_or(ConfigError::GatewayNotUnicode))
        .transpose()?;
    resolve(file.as_deref().map(Path::new), gateway, overlay)
}

fn resolve(file: Option<&Path>, gateway: Option<&str>, overlay: Option<&Path>) -> Result<Config> {
    let mut doc = base();
    if let Some(file) = file {
        json_patch::merge(&mut doc, &read_overlay(file)?);
    }
    if let Some(endpoint) = gateway {
        json_patch::merge(
            &mut doc,
            &json!({
                "mode": "client",
                "connect": { "endpoints": [endpoint], "timeout_ms": 3000 },
                "listen": { "endpoints": [] },
                "scouting": { "multicast": { "enabled": false } },
            }),
        );
    }
    if let Some(file) = overlay {
        json_patch::merge(&mut doc, &read_overlay(file)?);
    }
    native_config(doc)
}

fn read_overlay(path: &Path) -> Result<Value> {
    let invalid = |reason| ConfigError::InvalidSessionFile {
        path: path.into(),
        reason,
    };
    let text = fs::read_to_string(path).map_err(|source| ConfigError::ReadSessionFile {
        path: path.into(),
        source,
    })?;
    if text.is_empty() {
        return Err(invalid("expected a configuration object, found an empty file").into());
    }
    // Use the same syntax dispatch as Zenoh's file loader, but do not deserialize
    // a native Config here: its defaults and redacted private fields cannot round-trip.
    let doc: Value = match path.extension().and_then(|s| s.to_str()) {
        Some("json" | "json5") => json5::from_str(&text).map_err(|_| invalid("invalid JSON5"))?,
        Some("yaml" | "yml") => serde_yaml::from_str(&text).map_err(|_| invalid("invalid YAML"))?,
        Some("toml") => toml::from_str(&text).map_err(|_| invalid("invalid TOML"))?,
        _ => {
            return Err(invalid("expected a .json, .json5, .yaml, .yml or .toml extension").into());
        }
    };
    if !doc.is_object() {
        return Err(invalid("expected a configuration object").into());
    }
    Ok(doc)
}

fn native_config(doc: Value) -> Result<Config> {
    if let Some(plugins) = doc.get("plugins") {
        let plugins = plugins.as_object().ok_or(ConfigError::InvalidSession {
            reason: "plugins must be an object",
        })?;
        for plugin in plugins.values() {
            let plugin = plugin.as_object().ok_or(ConfigError::InvalidSession {
                reason: "each plugin configuration must be an object",
            })?;
            if has_plugin_include(plugin) {
                return Err(ConfigError::PluginInclude.into());
            }
        }
    }
    // Native errors may contain the rejected document or a credential-bearing value.
    Config::from_deserializer(&doc).map_err(|_| {
        ConfigError::InvalidSession {
            reason: "check the field names, types and values in the session overlays and ROS_Z_GATEWAY",
        }
        .into()
    })
}

// The native file loader expands includes in plugin objects and nested objects,
// but not within arrays. A plugin named `__config__` is not itself an include.
fn has_plugin_include(object: &serde_json::Map<String, Value>) -> bool {
    object.contains_key("__config__")
        || object
            .values()
            .filter_map(Value::as_object)
            .any(has_plugin_include)
}

pub(crate) fn isolated_config() -> Config {
    let mut doc = base();
    json_patch::merge(
        &mut doc,
        &json!({
            "mode": "peer",
            "listen": { "endpoints": ["tcp/127.0.0.1:0"] },
            "connect": { "endpoints": [] },
            "scouting": { "multicast": { "enabled": false }, "gossip": { "enabled": false } },
        }),
    );
    native_config(doc).expect("valid static isolated session config")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::TcpListener, time::Duration};

    fn value(config: &Config, key: &str) -> Value {
        serde_json::from_str(&config.get_json(key).unwrap()).unwrap()
    }

    #[test]
    fn patches_preserve_siblings_replace_arrays_and_delete_keys_before_validation() {
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("lower.json");
        let upper = dir.path().join("upper.yml");
        // .json uses Zenoh's JSON5 grammar. Neither patch has a valid usrpwd
        // configuration on its own; validation belongs after merging both.
        fs::write(
            &lower,
            r#"{
            region_name: 'robot', mode: 'peer',
            connect: { endpoints: ['tcp/old:1', 'tcp/old:2'], exit_on_failure: false },
            scouting: { gossip: { enabled: true } },
            transport: { auth: { usrpwd: { user: 'operator' } } },
            plugins: { demo: { private: { token: 'fixture' } } },
        }"#,
        )
        .unwrap();
        fs::write(&upper, "region_name: null\nconnect: {timeout_ms: 900}\ntransport: {auth: {usrpwd: {password: example}}}\n").unwrap();
        assert!(resolve(Some(&lower), None, None).is_err());
        let merged = resolve(Some(&lower), Some("tcp/127.0.0.1:7447"), Some(&upper)).unwrap();
        // Native Config serialization omits `private`. The plugin must still see it.
        assert!(merged.plugin("demo").unwrap()["private"]["token"] == "fixture");
        assert_eq!(value(&merged, "mode"), "client");
        assert_eq!(
            value(&merged, "connect/endpoints"),
            json!(["tcp/127.0.0.1:7447"])
        );
        assert_eq!(value(&merged, "connect/timeout_ms"), 900);
        assert_eq!(value(&merged, "connect/exit_on_failure"), false);
        assert_eq!(value(&merged, "scouting/gossip/enabled"), true);
        assert_eq!(
            value(&merged, "region_name"),
            value(&Config::default(), "region_name")
        );
        assert_eq!(value(&merged, "timestamping/enabled"), true);
        assert_eq!(value(&merged, "transport/shared_memory/enabled"), false);
    }

    #[test]
    fn toml_and_yaml_overlays_can_override_the_transport_policy() {
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("lower.toml");
        let upper = dir.path().join("upper.yaml");
        fs::write(&lower, "[transport.shared_memory]\nenabled = true\n").unwrap();
        fs::write(&upper, "timestamping:\n  enabled: false\n").unwrap();
        let config = resolve(Some(&lower), None, Some(&upper)).unwrap();
        assert_eq!(value(&config, "transport/shared_memory/enabled"), true);
        assert_eq!(value(&config, "timestamping/enabled"), false);
    }

    #[test]
    fn reject_only_effective_includes_that_the_native_file_loader_would_expand() {
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("lower.json5");
        let upper = dir.path().join("upper.json5");
        fs::write(
            &lower,
            r#"{plugins: {
            __config__: {list: [{__config__: 'data, not an include'}]},
            example: {nested: {__config__: 'external.json5'}},
        }}"#,
        )
        .unwrap();
        let error = resolve(Some(&lower), None, None).unwrap_err();
        assert!(
            matches!(error, crate::Error::Config(e) if matches!(*e, ConfigError::PluginInclude))
        );
        fs::write(&upper, "{plugins: {example: {nested: {__config__: null}}}}").unwrap();
        resolve(Some(&lower), None, Some(&upper)).unwrap();
    }

    #[test]
    fn broken_files_and_invalid_native_values_fail_without_disclosing_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lower.json5");
        assert!(resolve(Some(&path), Some("tcp/127.0.0.1:7447"), None).is_err());
        for contents in ["{mode: 'secret', :}", "{mode: 'secret'}", "['secret']"] {
            fs::write(&path, contents).unwrap();
            let err = resolve(None, None, Some(&path)).unwrap_err();
            assert!(!format!("{err}: {err:?}").contains("secret"));
        }
        assert!(resolve(None, Some("not-an-endpoint"), None).is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn robot_peer_starts_while_gateway_handshake_is_stalled() {
        let gateway = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/{}", gateway.local_addr().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let upper = dir.path().join("loopback.json5");
        fs::write(
            &upper,
            json!({
                "listen": {"endpoints": ["tcp/127.0.0.1:0"]},
                "connect": {"endpoints": [endpoint]},
                "scouting": {"multicast": {"enabled": false}, "gossip": {"enabled": false}},
            })
            .to_string(),
        )
        .unwrap();
        let robot =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/k1-setup/robot-peer.json5");
        let config = resolve(Some(&robot), None, Some(&upper)).unwrap();
        let session = tokio::time::timeout(Duration::from_secs(5), zenoh::open(config))
            .await
            .expect("peer startup waited for its optional gateway")
            .unwrap();
        drop(gateway);
        session.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn gateway_client_fails_within_startup_deadline_without_fallback() {
        let gateway = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/{}", gateway.local_addr().unwrap());
        let config = resolve(None, Some(&endpoint), None).unwrap();
        let result = tokio::time::timeout(Duration::from_secs(6), zenoh::open(config))
            .await
            .expect("gateway startup exceeded its three-second deadline and test margin");
        assert!(
            result.is_err(),
            "client unexpectedly opened or fell back to peer mode"
        );
    }
}

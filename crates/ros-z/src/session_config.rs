use std::{env, fs, path::Path};

use serde_json::{Value, json};
use zenoh::Config;

use crate::{Result, error::ConfigError};

fn sdk_defaults() -> Value {
    json!({
        "timestamping": { "enabled": true },
        "transport": { "shared_memory": { "enabled": false } },
    })
}

pub(crate) fn resolve(
    native: Option<Config>,
    explicit_file: Option<&Path>,
    overrides: Vec<(String, Value)>,
) -> Result<Config> {
    if let Some(mut config) = native {
        for (key, value) in overrides {
            let invalid = || ConfigError::InvalidOverride {
                key: key.clone(),
                reason: "Zenoh rejected the field or value",
            };
            let value = serde_json::to_string(&value).map_err(|_| invalid())?;
            config.insert_json5(&key, &value).map_err(|_| invalid())?;
        }
        return Ok(config);
    }

    let file = env::var_os("ZENOH_SESSION_CONFIG_URI");
    let gateway = env::var_os("ROS_Z_GATEWAY");
    let gateway = gateway
        .as_deref()
        .map(|value| value.to_str().ok_or(ConfigError::GatewayNotUnicode))
        .transpose()?;
    resolve_overlays(
        file.as_deref().map(Path::new),
        gateway,
        explicit_file,
        overrides,
    )
}

fn resolve_overlays(
    file: Option<&Path>,
    gateway: Option<&str>,
    explicit_file: Option<&Path>,
    overrides: Vec<(String, Value)>,
) -> Result<Config> {
    let mut doc = sdk_defaults();
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
                "scouting": { "multicast": { "enabled": false }, "gossip": { "enabled": false } },
            }),
        );
    }
    if let Some(file) = explicit_file {
        json_patch::merge(&mut doc, &read_overlay(file)?);
    }
    for (key, value) in overrides {
        let key_path = key.strip_prefix('/').unwrap_or(&key);
        let invalid = |reason| ConfigError::InvalidOverride {
            key: key.clone(),
            reason,
        };
        let mut target = Some(&doc);
        for field in key_path.split('/') {
            if field.is_empty() {
                return Err(invalid("expected a slash-separated field path").into());
            }
            if target.is_some_and(Value::is_array) {
                return Err(
                    invalid("cannot descend into an array; replace the whole array").into(),
                );
            }
            target = target.and_then(|value| value.get(field));
        }
        let patch = key_path
            .rsplit('/')
            .fold(value, |value, field| json!({ field: value }));
        json_patch::merge(&mut doc, &patch);
    }
    native_config(&doc)
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
    // Unlike JSON Value, YAML Value preserves non-finite floats until we can reject them.
    let doc: serde_yaml::Value = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json" | "json5") => json5::from_str(&text).map_err(|_| invalid("invalid JSON5"))?,
        Some("yaml" | "yml") => serde_yaml::from_str(&text).map_err(|_| invalid("invalid YAML"))?,
        Some("toml") => {
            let doc: toml::Value = toml::from_str(&text).map_err(|_| invalid("invalid TOML"))?;
            if has_toml_datetime(&doc) {
                return Err(invalid("TOML dates and times must be quoted as strings").into());
            }
            serde_yaml::to_value(doc).map_err(|_| invalid("invalid TOML value"))?
        }
        _ => {
            return Err(invalid("expected a .json, .json5, .yaml, .yml or .toml extension").into());
        }
    };
    if !doc.is_mapping() {
        return Err(invalid("expected a configuration object").into());
    }
    validate_json(&doc, false).map_err(invalid)?;
    serde_json::to_value(doc)
        .map_err(|_| invalid("configuration cannot be represented as JSON").into())
}

fn has_toml_datetime(doc: &toml::Value) -> bool {
    match doc {
        toml::Value::Datetime(_) => true,
        toml::Value::Array(values) => values.iter().any(has_toml_datetime),
        toml::Value::Table(entries) => entries.values().any(has_toml_datetime),
        _ => false,
    }
}

pub(crate) fn serialize_override(
    value: impl serde::Serialize,
) -> std::result::Result<Value, &'static str> {
    let probe = serde_yaml::to_value(&value).map_err(|_| "failed to serialize config value")?;
    validate_json(&probe, true)?;
    serde_json::to_value(value).map_err(|_| "configuration cannot be represented as JSON")
}

fn validate_json(
    doc: &serde_yaml::Value,
    from_serialize: bool,
) -> std::result::Result<(), &'static str> {
    use serde_yaml::Value;
    match doc {
        Value::Number(number) if number.as_f64().is_some_and(|value| !value.is_finite()) => {
            Err("non-finite numbers are not allowed")
        }
        Value::Mapping(entries) => entries.iter().try_for_each(|(key, value)| {
            if !from_serialize && !key.is_string() {
                return Err("configuration object keys must be strings");
            }
            validate_json(value, from_serialize)
        }),
        Value::Sequence(values) => values
            .iter()
            .try_for_each(|value| validate_json(value, from_serialize)),
        Value::Tagged(tagged) if from_serialize => validate_json(&tagged.value, from_serialize),
        Value::Tagged(_) => Err("YAML tags are not supported in session overlays"),
        _ => Ok(()),
    }
}

fn native_config(doc: &Value) -> Result<Config> {
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
    // Native errors can contain the rejected document or credential-bearing values.
    Config::from_deserializer(doc).map_err(|_| {
        ConfigError::InvalidSession {
            reason: "check field names, types and values in session files, ROS_Z_GATEWAY and builder overrides",
        }
        .into()
    })
}

// Zenoh's file loader looks for includes in plugin objects, not inside arrays.
fn has_plugin_include(object: &serde_json::Map<String, Value>) -> bool {
    object.contains_key("__config__")
        || object
            .values()
            .filter_map(Value::as_object)
            .any(has_plugin_include)
}

pub(crate) fn isolated_config() -> Config {
    let mut doc = sdk_defaults();
    json_patch::merge(
        &mut doc,
        &json!({
            "mode": "peer",
            "connect": { "endpoints": [] },
            "listen": { "endpoints": ["tcp/127.0.0.1:0"] },
            "scouting": { "multicast": { "enabled": false }, "gossip": { "enabled": false } },
        }),
    );
    native_config(&doc).expect("valid static loopback config")
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use tokio::net::TcpListener;

    use super::*;

    fn field(config: &Config, key: &str) -> Value {
        serde_json::from_str(&config.get_json(key).unwrap()).unwrap()
    }

    #[test]
    fn overlays_replace_arrays_merge_objects_and_delete_to_native_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("session.json");
        let upper = dir.path().join("explicit.yml");
        fs::write(&lower, r#"{
            mode: 'peer',
            connect: { endpoints: { peer: ['tcp/127.0.0.1:1'], client: ['tcp/127.0.0.1:2'] }, timeout_ms: { client: 40000 } },
            listen: { endpoints: { client: ['tcp/127.0.0.1:0'] } },
            scouting: { gossip: { enabled: true } },
            transport: { auth: { usrpwd: { user: 'operator' } } },
            plugins: { demo: { private: { token: 'fixture' } } },
        }"#).unwrap();
        fs::write(&upper, "connect: { timeout_ms: 900 }\ntimestamping: { enabled: null }\ntransport: { auth: { usrpwd: { password: example } } }\n").unwrap();
        let config = resolve_overlays(
            Some(&lower),
            Some("tcp/127.0.0.1:7447"),
            Some(&upper),
            vec![("connect/exit_on_failure".into(), json!(false))],
        )
        .unwrap();
        assert_eq!(field(&config, "mode"), "client");
        assert_eq!(
            field(&config, "connect/endpoints"),
            json!(["tcp/127.0.0.1:7447"])
        );
        assert_eq!(field(&config, "connect/timeout_ms"), 900);
        assert_eq!(field(&config, "connect/exit_on_failure"), false);
        assert_eq!(field(&config, "listen/endpoints"), json!([]));
        assert_eq!(field(&config, "scouting/multicast/enabled"), false);
        assert_eq!(field(&config, "scouting/gossip/enabled"), false);
        assert_eq!(
            field(&config, "timestamping/enabled"),
            field(&Config::default(), "timestamping/enabled")
        );
        assert_eq!(field(&config, "transport/shared_memory/enabled"), false);
        assert_eq!(
            config.plugin("demo").unwrap()["private"]["token"],
            "fixture"
        );

        let explicit = dir.path().join("explicit.toml");
        fs::write(
            &explicit,
            "mode = 'peer'\n[connect]\nendpoints = ['tcp/127.0.0.1:3']\n",
        )
        .unwrap();
        let config = resolve_overlays(
            None,
            Some("tcp/127.0.0.1:7447"),
            Some(&explicit),
            vec![("connect/endpoints".into(), Value::Null)],
        )
        .unwrap();
        assert_eq!(field(&config, "mode"), "peer");
        assert_eq!(
            field(&config, "connect/endpoints"),
            field(&Config::default(), "connect/endpoints")
        );
    }

    #[test]
    fn plugin_includes_are_rejected_only_where_the_effective_file_loader_would_expand_them() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("plugins.json5");
        fs::write(
            &file,
            r#"{plugins: {
            __config__: {list: [{__config__: 'plugin data'}]},
            demo: {nested: {__config__: 'external.json5'}},
        }}"#,
        )
        .unwrap();
        let error = resolve_overlays(Some(&file), None, None, vec![]).unwrap_err();
        assert!(
            matches!(error, crate::Error::Config(error) if matches!(*error, ConfigError::PluginInclude))
        );
        let config = resolve_overlays(
            Some(&file),
            None,
            None,
            vec![("plugins/demo/nested/__config__".into(), Value::Null)],
        )
        .unwrap();
        assert_eq!(
            config.plugin("__config__").unwrap()["list"][0]["__config__"],
            "plugin data"
        );
    }

    #[test]
    fn non_finite_numbers_never_become_deletions_even_in_nested_values() {
        let dir = tempfile::tempdir().unwrap();
        for (extension, contents) in [
            ("json5", "{plugins: {example: {value: Infinity}}}"),
            ("yaml", "plugins: {example: {value: [[-.inf]]}}"),
            ("toml", "[plugins.example]\nvalue = nan"),
        ] {
            let path = dir.path().join(format!("invalid.{extension}"));
            fs::write(&path, contents).unwrap();
            let error = read_overlay(&path).unwrap_err();
            assert!(error.to_string().contains("non-finite"), "{error}");
        }
    }

    #[test]
    fn builder_paths_can_replace_arrays_but_cannot_descend_into_them() {
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("session.json5");
        fs::write(&lower, "{plugins: {demo: {hosts: ['old']}}}").unwrap();
        let error = resolve_overlays(
            Some(&lower),
            None,
            None,
            vec![("plugins/demo/hosts/0".into(), json!("new"))],
        )
        .unwrap_err();
        assert!(error.to_string().contains("cannot descend into an array"));
        let config = resolve_overlays(
            Some(&lower),
            None,
            None,
            vec![("plugins/demo/hosts".into(), json!(["new"]))],
        )
        .unwrap();
        assert_eq!(config.plugin("demo").unwrap()["hosts"], json!(["new"]));
    }

    #[test]
    fn toml_datetime_cannot_merge_as_an_object() {
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("session.json5");
        let upper = dir.path().join("explicit.toml");
        fs::write(&lower, "{plugins: {demo: {start: {retained: true}}}}").unwrap();
        fs::write(&upper, "[plugins.demo]\nstart = 2026-11-25").unwrap();
        let error = resolve_overlays(Some(&lower), None, Some(&upper), vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("TOML dates and times must be quoted")
        );

        fs::write(&upper, "[plugins.demo]\nstart = '2026-11-25'").unwrap();
        let config = resolve_overlays(Some(&lower), None, Some(&upper), vec![]).unwrap();
        assert_eq!(config.plugin("demo").unwrap()["start"], "2026-11-25");
    }

    #[test]
    fn native_base_is_not_round_tripped_or_given_sdk_defaults() {
        let native =
            Config::from_json5("{plugins: {demo: {private: {token: 'fixture'}}}}").unwrap();
        let config = resolve(
            Some(native),
            Some(Path::new("missing.json5")),
            vec![("mode".into(), json!("client"))],
        )
        .unwrap();
        assert_eq!(
            config.plugin("demo").unwrap()["private"]["token"],
            "fixture"
        );
        assert_eq!(field(&config, "mode"), "client");
        assert_eq!(
            field(&config, "timestamping/enabled"),
            field(&Config::default(), "timestamping/enabled")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn gateway_connect_deadline_includes_a_stalled_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("tcp/{}", listener.local_addr().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("session.json5");
        fs::write(
            &lower,
            "{transport: {unicast: {open_timeout: 60000}}, connect: {timeout_ms: {client: 0}}}",
        )
        .unwrap();
        let config = resolve_overlays(Some(&lower), Some(&endpoint), None, vec![]).unwrap();
        assert_eq!(field(&config, "connect/timeout_ms"), 3000);
        assert_eq!(field(&config, "transport/unicast/open_timeout"), 60000);
        let start = Instant::now();
        let opening = tokio::spawn(async move { zenoh::open(config).await });
        let (_silent_peer, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
            .await
            .expect("gateway was not dialed")
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(6), opening)
            .await
            .expect("stalled handshake missed the connection deadline")
            .unwrap();
        assert!(result.is_err(), "client unexpectedly opened");
        assert!(start.elapsed() >= Duration::from_secs(3));
    }
}

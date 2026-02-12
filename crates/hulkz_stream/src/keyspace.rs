use std::{
    collections::BTreeMap,
    num::NonZeroU128,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hulkz::{Scope, ScopedPath, Timestamp};
use zenoh::{bytes::Encoding, time::TimestampId};

use crate::types::{NamespaceBinding, PlaneKind, SourceSpec, StreamRecord};

pub(crate) const META_PREFIX: &str = "hulkz_stream";
pub(crate) const STREAM_FORMAT_VERSION: &str = "1";
pub(crate) const FALLBACK_TIMESTAMP_ID_U128: u128 = 1;

/// Returns a deterministic dedup key for logical source identity.
pub(crate) fn source_key(spec: &SourceSpec) -> String {
    let binding = match &spec.namespace_binding {
        NamespaceBinding::FollowTarget => "follow-target".to_string(),
        NamespaceBinding::Pinned(ns) => format!("pinned:{ns}"),
    };

    format!(
        "{:?}|{}|{}|{}|{}",
        spec.plane,
        spec.path.scope().as_str(),
        spec.path.path(),
        spec.node_override.as_deref().unwrap_or(""),
        binding,
    )
}

/// Resolves the effective namespace for a source against current target namespace.
pub(crate) fn effective_namespace(spec: &SourceSpec, target_namespace: &str) -> Option<String> {
    match &spec.namespace_binding {
        NamespaceBinding::Pinned(ns) => Some(ns.clone()),
        NamespaceBinding::FollowTarget => Some(target_namespace.to_string()),
    }
}

/// Builds a hulkz key expression for a source and resolved namespace.
pub(crate) fn key_expr_for_record(spec: &SourceSpec, effective_namespace: Option<&str>) -> String {
    let namespace = effective_namespace.unwrap_or("default");
    let node = spec.node_override.as_deref().unwrap_or("*");

    match spec.plane {
        PlaneKind::Data => match spec.path.scope() {
            Scope::Global => format!("hulkz/data/global/{}", spec.path.path()),
            Scope::Local => format!("hulkz/data/local/{namespace}/{}", spec.path.path()),
            Scope::Private => {
                format!("hulkz/data/private/{namespace}/{node}/{}", spec.path.path())
            }
        },
        PlaneKind::View => match spec.path.scope() {
            Scope::Global => format!("hulkz/view/global/{}", spec.path.path()),
            Scope::Local => format!("hulkz/view/local/{namespace}/{}", spec.path.path()),
            Scope::Private => {
                format!("hulkz/view/private/{namespace}/{node}/{}", spec.path.path())
            }
        },
        PlaneKind::ParamReadUpdates => match spec.path.scope() {
            Scope::Global => format!("hulkz/param/read/global/{}", spec.path.path()),
            Scope::Local => format!("hulkz/param/read/local/{namespace}/{}", spec.path.path()),
            Scope::Private => {
                format!(
                    "hulkz/param/read/private/{namespace}/{node}/{}",
                    spec.path.path()
                )
            }
        },
        PlaneKind::ExternalRaw => spec.path.path().to_string(),
    }
}

pub(crate) fn encoding_to_mcap(encoding: &Encoding) -> String {
    encoding.to_string()
}

pub(crate) fn encoding_from_mcap(encoding: &str) -> Encoding {
    Encoding::from(encoding)
}

pub(crate) fn to_nanos(timestamp: &Timestamp) -> u64 {
    timestamp.get_time().as_nanos()
}

pub(crate) fn timestamp_id_to_string(timestamp: &Timestamp) -> String {
    timestamp.get_id().to_string()
}

pub(crate) fn timestamp_id_from_metadata(metadata: &BTreeMap<String, String>) -> Option<String> {
    metadata
        .get(&format!("{META_PREFIX}.timestamp_id"))
        .and_then(|value| (!value.is_empty()).then_some(value.clone()))
}

pub(crate) fn parse_timestamp_id(value: &str) -> Option<TimestampId> {
    TimestampId::from_str(value).ok()
}

pub(crate) fn fallback_timestamp_id() -> TimestampId {
    NonZeroU128::new(FALLBACK_TIMESTAMP_ID_U128)
        .expect("fallback ID is non-zero")
        .into()
}

/// Reconstructs a hulkz timestamp from nanos assuming local clock-domain alignment.
pub(crate) fn from_nanos(nanos: u64) -> Timestamp {
    let system_time = UNIX_EPOCH + Duration::from_nanos(nanos);
    from_system_time(system_time)
}

/// Creates a timestamp using a fallback deterministic clock id.
pub(crate) fn from_system_time(system_time: SystemTime) -> Timestamp {
    let nanos = system_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    Timestamp::new(zenoh::time::NTP64::from(nanos), fallback_timestamp_id())
}

/// Reconstructs a timestamp from nanos and optional serialized timestamp id metadata.
pub(crate) fn from_nanos_with_id(nanos: u64, timestamp_id: Option<&str>) -> Timestamp {
    let id = timestamp_id
        .and_then(parse_timestamp_id)
        .unwrap_or_else(fallback_timestamp_id);
    Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

pub(crate) fn metadata_for_record(record: &StreamRecord) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        format!("{META_PREFIX}.version"),
        STREAM_FORMAT_VERSION.to_string(),
    );
    metadata.insert(
        format!("{META_PREFIX}.plane"),
        match record.source.plane {
            PlaneKind::Data => "data",
            PlaneKind::View => "view",
            PlaneKind::ParamReadUpdates => "param-read-updates",
            PlaneKind::ExternalRaw => "external-raw",
        }
        .to_string(),
    );
    metadata.insert(
        format!("{META_PREFIX}.scope"),
        record.source.path.scope().as_str().to_string(),
    );
    metadata.insert(
        format!("{META_PREFIX}.path"),
        record.source.path.path().to_string(),
    );
    metadata.insert(
        format!("{META_PREFIX}.node_override"),
        record.source.node_override.clone().unwrap_or_default(),
    );
    metadata.insert(
        format!("{META_PREFIX}.namespace_binding"),
        match &record.source.namespace_binding {
            NamespaceBinding::FollowTarget => "follow-target".to_string(),
            NamespaceBinding::Pinned(ns) => format!("pinned:{ns}"),
        },
    );
    metadata.insert(
        format!("{META_PREFIX}.effective_namespace"),
        record.effective_namespace.clone().unwrap_or_default(),
    );
    metadata.insert(
        format!("{META_PREFIX}.timestamp_id"),
        timestamp_id_to_string(&record.timestamp),
    );
    metadata
}

pub(crate) fn source_from_topic_and_metadata(
    topic: &str,
    metadata: &BTreeMap<String, String>,
) -> (SourceSpec, Option<String>) {
    if let Some(plane) = metadata.get(&format!("{META_PREFIX}.plane")) {
        let scope = match metadata
            .get(&format!("{META_PREFIX}.scope"))
            .map(|s| s.as_str())
        {
            Some("global") => Scope::Global,
            Some("private") => Scope::Private,
            _ => Scope::Local,
        };
        let path = metadata
            .get(&format!("{META_PREFIX}.path"))
            .cloned()
            .unwrap_or_else(|| topic.to_string());
        let node_override = metadata
            .get(&format!("{META_PREFIX}.node_override"))
            .and_then(|v| (!v.is_empty()).then_some(v.clone()));
        let namespace_binding = match metadata
            .get(&format!("{META_PREFIX}.namespace_binding"))
            .map(String::as_str)
        {
            Some(binding) if binding.starts_with("pinned:") => {
                NamespaceBinding::Pinned(binding.trim_start_matches("pinned:").to_string())
            }
            _ => NamespaceBinding::FollowTarget,
        };
        let effective_namespace = metadata
            .get(&format!("{META_PREFIX}.effective_namespace"))
            .and_then(|v| (!v.is_empty()).then_some(v.clone()));

        let plane = match plane.as_str() {
            "data" => PlaneKind::Data,
            "view" => PlaneKind::View,
            "param-read-updates" => PlaneKind::ParamReadUpdates,
            _ => PlaneKind::ExternalRaw,
        };

        return (
            SourceSpec {
                plane,
                path: ScopedPath::new(scope, path),
                node_override,
                namespace_binding,
            },
            effective_namespace,
        );
    }

    parse_topic_best_effort(topic)
}

fn parse_topic_best_effort(topic: &str) -> (SourceSpec, Option<String>) {
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() >= 4 && parts[0] == "hulkz" && (parts[1] == "data" || parts[1] == "view") {
        let plane = if parts[1] == "data" {
            PlaneKind::Data
        } else {
            PlaneKind::View
        };
        match parts[2] {
            "global" => {
                let path = parts[3..].join("/");
                return (
                    SourceSpec {
                        plane,
                        path: ScopedPath::new(Scope::Global, path),
                        node_override: None,
                        namespace_binding: NamespaceBinding::FollowTarget,
                    },
                    None,
                );
            }
            "local" if parts.len() >= 5 => {
                let namespace = parts[3].to_string();
                let path = parts[4..].join("/");
                return (
                    SourceSpec {
                        plane,
                        path: ScopedPath::new(Scope::Local, path),
                        node_override: None,
                        namespace_binding: NamespaceBinding::Pinned(namespace.clone()),
                    },
                    Some(namespace),
                );
            }
            "private" if parts.len() >= 6 => {
                let namespace = parts[3].to_string();
                let node = parts[4].to_string();
                let path = parts[5..].join("/");
                return (
                    SourceSpec {
                        plane,
                        path: ScopedPath::new(Scope::Private, path),
                        node_override: Some(node),
                        namespace_binding: NamespaceBinding::Pinned(namespace.clone()),
                    },
                    Some(namespace),
                );
            }
            _ => {}
        }
    }

    if parts.len() >= 5
        && parts[0] == "hulkz"
        && parts[1] == "param"
        && parts[2] == "read"
        && (parts[3] == "global" || parts[3] == "local" || parts[3] == "private")
    {
        match parts[3] {
            "global" => {
                let path = parts[4..].join("/");
                return (
                    SourceSpec {
                        plane: PlaneKind::ParamReadUpdates,
                        path: ScopedPath::new(Scope::Global, path),
                        node_override: None,
                        namespace_binding: NamespaceBinding::FollowTarget,
                    },
                    None,
                );
            }
            "local" if parts.len() >= 6 => {
                let namespace = parts[4].to_string();
                let path = parts[5..].join("/");
                return (
                    SourceSpec {
                        plane: PlaneKind::ParamReadUpdates,
                        path: ScopedPath::new(Scope::Local, path),
                        node_override: None,
                        namespace_binding: NamespaceBinding::Pinned(namespace.clone()),
                    },
                    Some(namespace),
                );
            }
            "private" if parts.len() >= 7 => {
                let namespace = parts[4].to_string();
                let node = parts[5].to_string();
                let path = parts[6..].join("/");
                return (
                    SourceSpec {
                        plane: PlaneKind::ParamReadUpdates,
                        path: ScopedPath::new(Scope::Private, path),
                        node_override: Some(node),
                        namespace_binding: NamespaceBinding::Pinned(namespace.clone()),
                    },
                    Some(namespace),
                );
            }
            _ => {}
        }
    }

    (
        SourceSpec {
            plane: PlaneKind::ExternalRaw,
            path: ScopedPath::new(Scope::Global, topic),
            node_override: None,
            namespace_binding: NamespaceBinding::Pinned("__external__".to_string()),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use hulkz::{Scope, ScopedPath};
    use zenoh::bytes::Encoding;

    use crate::types::{NamespaceBinding, PlaneKind, SourceSpec, StreamRecord};

    use super::{
        from_nanos_with_id, metadata_for_record, parse_topic_best_effort, source_key,
        timestamp_id_from_metadata, to_nanos,
    };

    #[test]
    fn source_key_distinguishes_namespace_binding() {
        let follow = SourceSpec {
            plane: PlaneKind::Data,
            path: ScopedPath::new(Scope::Local, "imu"),
            node_override: None,
            namespace_binding: NamespaceBinding::FollowTarget,
        };
        let pinned = SourceSpec {
            plane: PlaneKind::Data,
            path: ScopedPath::new(Scope::Local, "imu"),
            node_override: None,
            namespace_binding: NamespaceBinding::Pinned("robot".to_string()),
        };

        assert_ne!(source_key(&follow), source_key(&pinned));
    }

    #[test]
    fn best_effort_topic_parse_for_local_data() {
        let (spec, effective) = parse_topic_best_effort("hulkz/data/local/nao42/camera/front");

        assert_eq!(spec.plane, PlaneKind::Data);
        assert_eq!(spec.path.scope(), Scope::Local);
        assert_eq!(spec.path.path(), "camera/front");
        assert_eq!(effective.as_deref(), Some("nao42"));
    }

    #[test]
    fn timestamp_fallback_is_explicit_and_stable() {
        let ts = from_nanos_with_id(1_000, None);
        assert_eq!(to_nanos(&ts), 1_000);
        assert_eq!(ts.get_time().to_duration(), Duration::from_nanos(1_000));
    }

    #[test]
    fn timestamp_id_metadata_roundtrip() {
        let source = SourceSpec {
            plane: PlaneKind::Data,
            path: ScopedPath::new(Scope::Local, "imu"),
            node_override: None,
            namespace_binding: NamespaceBinding::Pinned("robot".to_string()),
        };
        let record = StreamRecord {
            source,
            effective_namespace: Some("robot".to_string()),
            timestamp: from_nanos_with_id(42, None),
            encoding: Encoding::APPLICATION_CDR,
            payload: Arc::from([1_u8, 2, 3]),
        };

        let metadata = metadata_for_record(&record);
        let timestamp_id = timestamp_id_from_metadata(&metadata);
        let rebuilt = from_nanos_with_id(42, timestamp_id.as_deref());

        assert_eq!(
            rebuilt.get_id().to_string(),
            record.timestamp.get_id().to_string()
        );
        assert_eq!(to_nanos(&rebuilt), 42);
    }
}

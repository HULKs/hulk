use std::{
    collections::BTreeMap,
    num::NonZeroU128,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hulkz::{Timestamp, TopicExpression};
use zenoh::{bytes::Encoding, time::TimestampId};

use crate::error::{Error, Result};
use crate::types::{NamespaceBinding, PlaneKind, SourceSpec, StreamRecord};

pub(crate) const META_PREFIX: &str = "hulkz_stream";
pub(crate) const STREAM_FORMAT_VERSION: &str = "2";
pub(crate) const FALLBACK_TIMESTAMP_ID_U128: u128 = 1;

/// Returns a deterministic dedup key for logical source identity.
pub(crate) fn source_key(spec: &SourceSpec) -> String {
    let binding = match &spec.namespace_binding {
        NamespaceBinding::FollowTarget => "follow-target".to_string(),
        NamespaceBinding::Pinned(ns) => format!("pinned:{ns}"),
    };

    format!(
        "{:?}|{}|{}|{}",
        spec.plane,
        spec.topic_expression.as_str(),
        spec.default_node.as_deref().unwrap_or(""),
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

/// Resolves canonical topic for a source and namespace.
pub(crate) fn resolved_topic_for_source(
    spec: &SourceSpec,
    effective_namespace: Option<&str>,
) -> Result<String> {
    let namespace = effective_namespace.unwrap_or("default");
    Ok(spec
        .topic_expression
        .resolve(namespace, spec.default_node.as_deref())?)
}

/// Builds a hulkz key expression for a source and resolved namespace.
pub(crate) fn key_expr_for_record(
    spec: &SourceSpec,
    effective_namespace: Option<&str>,
) -> Result<String> {
    let resolved_topic = resolved_topic_for_source(spec, effective_namespace)?;
    let domain_id = current_domain_id();
    Ok(match spec.plane {
        PlaneKind::Data => format!("hulkz/data/{domain_id}/{resolved_topic}"),
        PlaneKind::View => format!("hulkz/view/{domain_id}/{resolved_topic}"),
        PlaneKind::ParamReadUpdates => format!("hulkz/param/read/{domain_id}/{resolved_topic}"),
    })
}

fn current_domain_id() -> u32 {
    std::env::var("ROS_DOMAIN_ID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0)
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
        }
        .to_string(),
    );
    metadata.insert(
        format!("{META_PREFIX}.topic_expression"),
        record.source.topic_expression.as_str().to_string(),
    );
    metadata.insert(
        format!("{META_PREFIX}.default_node"),
        record.source.default_node.clone().unwrap_or_default(),
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
) -> Result<(SourceSpec, Option<String>)> {
    if let Some(plane) = metadata.get(&format!("{META_PREFIX}.plane")) {
        if let Some(version) = metadata.get(&format!("{META_PREFIX}.version")) {
            if version != STREAM_FORMAT_VERSION {
                return Err(Error::UnsupportedStreamMetadataVersion {
                    expected: STREAM_FORMAT_VERSION.to_string(),
                    found: version.clone(),
                });
            }
        }

        let topic_expression = if let Some(expression) =
            metadata.get(&format!("{META_PREFIX}.topic_expression"))
        {
            TopicExpression::parse(expression)?
        } else {
            let (spec, _) = parse_topic_best_effort(topic)?;
            spec.topic_expression
        };

        let default_node = metadata
            .get(&format!("{META_PREFIX}.default_node"))
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
            _ => {
                return Err(Error::UnknownPlaneMapping {
                    topic: topic.to_string(),
                    plane: Some(plane.clone()),
                });
            }
        };

        return Ok((
            SourceSpec {
                plane,
                topic_expression,
                default_node,
                namespace_binding,
            },
            effective_namespace,
        ));
    }

    parse_topic_best_effort(topic)
}

fn parse_topic_best_effort(topic: &str) -> Result<(SourceSpec, Option<String>)> {
    let (plane, rest) = if let Some(rest) = topic.strip_prefix("hulkz/data/") {
        (PlaneKind::Data, rest)
    } else if let Some(rest) = topic.strip_prefix("hulkz/view/") {
        (PlaneKind::View, rest)
    } else if let Some(rest) = topic.strip_prefix("hulkz/param/read/") {
        (PlaneKind::ParamReadUpdates, rest)
    } else {
        return Err(Error::UnknownPlaneMapping {
            topic: topic.to_string(),
            plane: None,
        });
    };

    let (domain_id, resolved_topic) = rest.split_once('/').ok_or_else(|| Error::UnknownPlaneMapping {
        topic: topic.to_string(),
        plane: None,
    })?;
    if domain_id.parse::<u32>().is_err() || resolved_topic.is_empty() {
        return Err(Error::UnknownPlaneMapping {
            topic: topic.to_string(),
            plane: None,
        });
    }

    if resolved_topic.is_empty() {
        return Err(Error::UnknownPlaneMapping {
            topic: topic.to_string(),
            plane: None,
        });
    }

    Ok((
        SourceSpec {
            plane,
            topic_expression: TopicExpression::parse(&format!("/{resolved_topic}"))?,
            default_node: None,
            namespace_binding: NamespaceBinding::FollowTarget,
        },
        None,
    ))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc, time::Duration};

    use hulkz::TopicExpression;
    use zenoh::bytes::Encoding;

    use crate::error::Error;
    use crate::types::{NamespaceBinding, PlaneKind, SourceSpec, StreamRecord};

    use super::{
        current_domain_id, from_nanos_with_id, key_expr_for_record, metadata_for_record,
        parse_topic_best_effort, source_from_topic_and_metadata, source_key,
        timestamp_id_from_metadata, to_nanos,
    };

    #[test]
    fn source_key_distinguishes_namespace_binding() {
        let follow = SourceSpec {
            plane: PlaneKind::Data,
            topic_expression: TopicExpression::parse("imu").unwrap(),
            default_node: None,
            namespace_binding: NamespaceBinding::FollowTarget,
        };
        let pinned = SourceSpec {
            plane: PlaneKind::Data,
            topic_expression: TopicExpression::parse("imu").unwrap(),
            default_node: None,
            namespace_binding: NamespaceBinding::Pinned("robot".to_string()),
        };

        assert_ne!(source_key(&follow), source_key(&pinned));
    }

    #[test]
    fn best_effort_topic_parse_for_data() {
        let (spec, effective) =
            parse_topic_best_effort("hulkz/data/0/nao42/camera/front").unwrap();

        assert_eq!(spec.plane, PlaneKind::Data);
        assert_eq!(spec.topic_expression.as_str(), "/nao42/camera/front");
        assert_eq!(effective, None);
    }

    #[test]
    fn metadata_parse_errors_for_unknown_plane() {
        let mut metadata = BTreeMap::new();
        metadata.insert(format!("{}.version", super::META_PREFIX), "2".to_string());
        metadata.insert(
            format!("{}.plane", super::META_PREFIX),
            "external-raw".to_string(),
        );

        let error = source_from_topic_and_metadata("hulkz/data/0/imu", &metadata).unwrap_err();

        assert!(matches!(
            error,
            Error::UnknownPlaneMapping {
                ref topic,
                plane: Some(ref plane),
            } if topic == "hulkz/data/0/imu" && plane == "external-raw"
        ));
    }

    #[test]
    fn best_effort_topic_parse_errors_for_unknown_topic() {
        let error = parse_topic_best_effort("external/topic/raw").unwrap_err();

        assert!(matches!(
            error,
            Error::UnknownPlaneMapping {
                ref topic,
                plane: None,
            } if topic == "external/topic/raw"
        ));
    }

    #[test]
    fn key_expr_for_record_uses_resolved_topic() {
        let spec = SourceSpec {
            plane: PlaneKind::Data,
            topic_expression: TopicExpression::parse("camera/front").unwrap(),
            default_node: None,
            namespace_binding: NamespaceBinding::Pinned("robot".to_string()),
        };

        let key = key_expr_for_record(&spec, Some("robot")).unwrap();
        assert_eq!(
            key,
            format!("hulkz/data/{}/robot/camera/front", current_domain_id())
        );
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
            topic_expression: TopicExpression::parse("imu").unwrap(),
            default_node: None,
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

use std::sync::Arc;

use ros_z::{TypeInfo, dynamic::DynamicPayload, time::Time};

/// Metadata fixed for all samples received through one debug subscription.
#[derive(Debug)]
#[non_exhaustive]
pub struct SampleMetadata {
    /// Topic reference originally requested by the caller.
    pub topic_reference: crate::TopicReference,
    /// Absolute topic name used for the underlying subscription.
    pub resolved_topic: String,
    /// Type metadata discovered or declared for this subscription.
    pub type_info: TypeInfo,
}

/// A retained subscription sample with transport metadata and shared subscription metadata.
#[derive(Debug)]
#[non_exhaustive]
pub struct SampleRecord<V> {
    /// Decoded typed value or dynamic payload.
    pub value: V,
    /// Source timestamp reported by the publisher.
    pub source_time: Time,
    /// Zenoh transport timestamp, when present on the received sample.
    pub transport_time: Option<Time>,
    /// Stable publication identity derived from source id and sequence number.
    pub publication_id: ros_z::pubsub::PublicationId,
    /// Metadata shared by all samples received through the same subscription.
    pub metadata: Arc<SampleMetadata>,
}

/// A retained dynamic subscription sample projected to JSON with original metadata preserved.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JsonSampleRecord {
    /// Source timestamp reported by the publisher.
    pub source_time: ros_z::time::Time,
    /// Zenoh transport timestamp, when present on the received sample.
    pub transport_time: Option<ros_z::time::Time>,
    /// Stable publication identity derived from source id and sequence number.
    pub publication_id: ros_z::pubsub::PublicationId,
    /// Metadata shared by all samples received through the same subscription.
    pub metadata: Arc<SampleMetadata>,
    /// Dynamic payload rendered with the active JSON render policy.
    pub value: serde_json::Value,
}

pub(crate) fn dynamic_record_json_value(
    record: &SampleRecord<DynamicPayload>,
    policy: crate::JsonRenderPolicy,
) -> serde_json::Value {
    crate::dynamic_payload_to_json(&record.value, policy)
}

pub(crate) fn dynamic_record_to_json_sample(
    record: Arc<SampleRecord<DynamicPayload>>,
    policy: crate::JsonRenderPolicy,
) -> JsonSampleRecord {
    JsonSampleRecord {
        source_time: record.source_time,
        transport_time: record.transport_time,
        publication_id: record.publication_id,
        metadata: Arc::clone(&record.metadata),
        value: dynamic_record_json_value(record.as_ref(), policy),
    }
}

use std::sync::Arc;

use ros_z::{TypeInfo, time::Time};

/// Metadata fixed for all samples received through one debug subscription.
#[derive(Debug)]
#[non_exhaustive]
pub struct SampleMetadata {
    /// Topic selector originally requested by the caller.
    pub requested_topic: crate::TopicSelector,
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

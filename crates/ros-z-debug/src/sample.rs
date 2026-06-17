use std::{sync::Arc, time::Duration};

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
    /// Wallclock timestamp when the debug subscription received and decoded this sample.
    pub receive_time: Time,
    /// Stable publication identity derived from source id and sequence number.
    pub publication_id: ros_z::pubsub::PublicationId,
    /// Metadata shared by all samples received through the same subscription.
    pub metadata: Arc<SampleMetadata>,
}

impl<V> SampleRecord<V> {
    /// Duration from the publisher source timestamp to `now`.
    pub fn source_latency_at(&self, now: Time) -> Duration {
        now.duration_since(self.source_time)
    }

    /// Duration from the Zenoh transport timestamp to `now`, when the sample carried one.
    pub fn transport_latency_at(&self, now: Time) -> Option<Duration> {
        self.transport_time
            .map(|transport_time| now.duration_since(transport_time))
    }

    /// Duration from local debug-subscription receive time to `now`.
    pub fn receive_latency_at(&self, now: Time) -> Duration {
        now.duration_since(self.receive_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn publication_id() -> ros_z::pubsub::PublicationId {
        ros_z::pubsub::Received {
            message: (),
            transport_time: None,
            source_time: Time::zero(),
            sequence_number: 1,
            source_global_id: [1; 16].into(),
        }
        .publication_id()
    }

    fn metadata() -> Arc<SampleMetadata> {
        Arc::new(SampleMetadata {
            requested_topic: crate::TopicSelector::new("debug").unwrap(),
            resolved_topic: "/debug".to_string(),
            type_info: ros_z::TypeInfo::new("test_msgs::Debug", ros_z::SchemaHash::zero()),
        })
    }

    #[test]
    fn latency_helpers_measure_source_transport_and_receive_age() {
        let record = SampleRecord {
            value: 1,
            source_time: Time::from_nanos(1_000),
            transport_time: Some(Time::from_nanos(2_000)),
            receive_time: Time::from_nanos(3_000),
            publication_id: publication_id(),
            metadata: metadata(),
        };

        let now = Time::from_nanos(5_000);

        assert_eq!(
            record.source_latency_at(now),
            std::time::Duration::from_nanos(4_000)
        );
        assert_eq!(
            record.transport_latency_at(now),
            Some(std::time::Duration::from_nanos(3_000))
        );
        assert_eq!(
            record.receive_latency_at(now),
            std::time::Duration::from_nanos(2_000)
        );
    }
}

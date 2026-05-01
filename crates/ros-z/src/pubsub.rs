use std::time::Duration;

mod metadata;
mod publisher;
mod raw;
mod replay;
mod subscriber;

pub use metadata::{PublicationId, Received};
pub use publisher::{PreparedPublication, Publisher, PublisherBuilder};
pub use raw::{RawSubscriber, RawSubscriberBuilder};
pub use subscriber::{Subscriber, SubscriberBuilder};

pub(crate) const DEFAULT_TRANSIENT_LOCAL_REPLAY_TIMEOUT: Duration = Duration::from_secs(1);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::qos::QosProfile;
    use ros_z_protocol::qos::{QosDurability, QosHistory};

    #[test]
    fn transient_local_subscriber_queue_capacity_matches_qos_depth() {
        let qos = ros_z_protocol::qos::QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(3),
            ..Default::default()
        };

        assert_eq!(subscriber::subscriber_queue_capacity(&qos), 3);
    }

    // -----------------------------------------------------------------------
    // Topic name qualification (leading '/' is added when missing)
    // -----------------------------------------------------------------------

    #[test]
    fn test_qualify_absolute_topic_unchanged() {
        let result = crate::topic_name::qualify_topic_name("/chatter", "/", "node").unwrap();
        assert_eq!(result, "/chatter");
    }

    #[test]
    fn test_qualify_relative_topic_adds_leading_slash() {
        let result = crate::topic_name::qualify_topic_name("chatter", "/", "node").unwrap();
        assert_eq!(result, "/chatter");
    }

    #[test]
    fn test_qualify_topic_with_namespace() {
        let result = crate::topic_name::qualify_topic_name("chatter", "/ns", "node").unwrap();
        assert_eq!(result, "/ns/chatter");
    }

    #[test]
    fn test_qualify_topic_nested_ns() {
        let result = crate::topic_name::qualify_topic_name("/ns/sub/topic", "/", "node").unwrap();
        assert_eq!(result, "/ns/sub/topic");
    }

    // -----------------------------------------------------------------------
    // QoS override is stored in builder entity.qos
    // QoS defaults: Reliable, Volatile, KeepLast(10)
    // -----------------------------------------------------------------------

    #[test]
    fn test_qos_reliability_encoding() {
        // Reliable is the default, BestEffort maps to protocol value
        let best_effort = QosProfile {
            reliability: crate::qos::QosReliability::BestEffort,
            ..Default::default()
        };
        let proto = best_effort.to_protocol_qos();
        assert_eq!(
            proto.reliability,
            ros_z_protocol::qos::QosReliability::BestEffort
        );
    }

    #[test]
    fn test_qos_durability_encoding() {
        let transient = QosProfile {
            durability: crate::qos::QosDurability::TransientLocal,
            ..Default::default()
        };
        let proto = transient.to_protocol_qos();
        assert_eq!(
            proto.durability,
            ros_z_protocol::qos::QosDurability::TransientLocal
        );
    }

    #[test]
    fn test_qos_keep_last_depth_preserved_in_protocol() {
        use std::num::NonZeroUsize;
        let qos = QosProfile {
            history: crate::qos::QosHistory::KeepLast(NonZeroUsize::new(5).unwrap()),
            ..Default::default()
        };
        let proto = qos.to_protocol_qos();
        assert_eq!(proto.history, ros_z_protocol::qos::QosHistory::KeepLast(5));
    }

    #[test]
    fn received_deref_and_partial_eq_follow_inner_message() {
        let received = Received {
            message: vec![1_u8, 2, 3],
            transport_time: None,
            source_time: None,
            sequence_number: None,
            source_global_id: None,
        };

        assert_eq!(received.len(), 3);
        assert_eq!(received, vec![1_u8, 2, 3]);
        assert_eq!(received[1], 2);
    }

    #[test]
    fn dynamic_publish_schema_validation_rejects_mismatched_message_schema() {
        let advertised_schema = Arc::new(
            crate::dynamic::MessageSchema::builder("geometry_msgs::Point")
                .field("x", crate::dynamic::FieldType::Float64)
                .field("y", crate::dynamic::FieldType::Float64)
                .field("z", crate::dynamic::FieldType::Float64)
                .build()
                .unwrap(),
        );
        let schema = Arc::new(
            crate::dynamic::MessageSchema::builder("geometry_msgs::Vector3")
                .field("x", crate::dynamic::FieldType::Float64)
                .field("y", crate::dynamic::FieldType::Float64)
                .field("z", crate::dynamic::FieldType::Float64)
                .build()
                .unwrap(),
        );
        let message = crate::dynamic::DynamicMessage::new(&schema);

        let error = publisher::validate_dynamic_publish_schema(Some(&advertised_schema), &message)
            .expect_err("mismatched schemas should fail before publishing");

        assert!(error.to_string().contains("schema mismatch"));
    }
}

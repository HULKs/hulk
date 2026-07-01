use std::{error::Error as _, fmt::Write as _, num::NonZero, sync::Arc, time::Duration};

use ros_z::{Message, dynamic::DynamicPayload, node::Node, qos::QosProfile};
use tokio_util::sync::CancellationToken;

use crate::{
    CachedJsonSubscription, CachedSubscription, CachedSubscriptionStatus,
    CachedSubscriptionStatusSnapshot, JsonRenderPolicy, Result, RetentionPolicy, SampleMetadata,
    SampleRecord, TargetIdentity, TopicReference, cache::CachedSubscriptionState,
};

/// Convenience methods for creating cached debug subscriptions from a `ros-z` node.
pub trait CachedSubscriptionNodeExt {
    /// Start building a cached debug subscription for `topic`.
    fn cached_subscription(&self, topic: impl Into<String>) -> Result<CachedSubscriptionBuilder>;
}

impl CachedSubscriptionNodeExt for Arc<Node> {
    fn cached_subscription(&self, topic: impl Into<String>) -> Result<CachedSubscriptionBuilder> {
        CachedSubscriptionBuilder::new(Arc::clone(self), topic)
    }
}

/// Builder for cached debug subscriptions.
///
/// Relative topic references resolve against the configured target namespace.
/// Private topic references resolve against the configured target namespace and
/// target node name. Dropping the returned subscription handle stops the receive
/// task when no other handles remain.
pub struct CachedSubscriptionBuilder {
    node: Arc<Node>,
    topic: TopicReference,
    target_identity: TargetIdentity,
    retention: RetentionPolicy,
    schema_discovery_timeout: Duration,
}

impl CachedSubscriptionBuilder {
    /// Create a builder for a cached subscription owned by the returned handle.
    pub fn new(node: Arc<Node>, topic: impl Into<String>) -> Result<Self> {
        Ok(Self {
            node,
            topic: TopicReference::new(topic.into())?,
            target_identity: TargetIdentity::new("/").expect("root namespace is valid"),
            retention: RetentionPolicy::LatestOnly,
            schema_discovery_timeout: Duration::from_secs(5),
        })
    }

    /// Configure how many samples the handle retains.
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Configure the target identity used to resolve topic references.
    pub fn target_identity(mut self, identity: TargetIdentity) -> Self {
        self.target_identity = identity;
        self
    }

    /// Configure the target namespace used to resolve relative topic references.
    pub fn target_namespace(mut self, namespace: impl Into<String>) -> Result<Self> {
        self.target_identity.set_namespace(namespace)?;
        Ok(self)
    }

    /// Configure the target node name used to resolve private topic references.
    pub fn target_node_name(mut self, node_name: impl Into<String>) -> Result<Self> {
        self.target_identity.set_node_name(node_name)?;
        Ok(self)
    }

    /// Clear the target node name used to resolve private topic references.
    pub fn clear_target_node_name(mut self) -> Self {
        self.target_identity.clear_node_name();
        self
    }

    /// Configure how long dynamic subscriptions wait while discovering schemas.
    pub fn schema_discovery_timeout(mut self, timeout: Duration) -> Self {
        self.schema_discovery_timeout = timeout;
        self
    }

    /// Topic reference requested by the caller.
    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }

    /// Configured retention policy.
    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }

    /// Build the typed subscription and spawn its receive task.
    pub async fn build_typed<T>(self) -> Result<CachedSubscription<T>>
    where
        T: Message + Send + Sync + 'static,
        T::Codec: Send + Sync,
    {
        let Self {
            node,
            topic,
            target_identity,
            retention,
            schema_discovery_timeout: _,
        } = self;
        let resolved_topic = topic.resolve(&target_identity)?;
        let subscriber = node
            .subscriber::<T>(&resolved_topic)
            .qos(QosProfile {
                durability: ros_z::qos::QosDurability::TransientLocal,
                history: ros_z::qos::QosHistory::KeepLast(NonZero::new(1).unwrap()),
                ..Default::default()
            })
            .build()
            .await?;
        let type_info = subscriber.entity().type_info.clone();
        let metadata = Arc::new(SampleMetadata {
            topic_reference: topic,
            resolved_topic: resolved_topic.clone(),
            type_info: type_info.clone(),
        });
        let state = CachedSubscriptionState::spawn(
            CachedSubscriptionStatusSnapshot::with_metadata(
                CachedSubscriptionStatus::WaitingForFirstSample,
                resolved_topic,
                type_info,
            ),
            retention,
            move |state, cancellation| {
                receive_typed_loop(subscriber, state, cancellation, metadata)
            },
        );

        Ok(state.handle())
    }

    /// Build a dynamic payload subscription and spawn its receive task.
    ///
    /// Schema discovery requires at least one currently visible publisher on the
    /// resolved topic, consistent type metadata, and a reachable schema service.
    pub async fn build_dynamic(self) -> Result<CachedSubscription<DynamicPayload>> {
        self.build_dynamic_payload().await
    }

    /// Build a dynamic JSON subscription with `policy`.
    ///
    /// Schema discovery has the same requirements as [`Self::build_dynamic`].
    pub async fn build_json(self, policy: JsonRenderPolicy) -> Result<CachedJsonSubscription> {
        let dynamic = self.build_dynamic_payload().await?;
        Ok(CachedJsonSubscription::new(dynamic, policy))
    }

    async fn build_dynamic_payload(self) -> Result<CachedSubscription<DynamicPayload>> {
        let Self {
            node,
            topic,
            target_identity,
            retention,
            schema_discovery_timeout,
        } = self;
        let resolved_topic = topic.resolve(&target_identity)?;
        let subscriber = node
            .dynamic_subscriber_auto(&resolved_topic, schema_discovery_timeout)
            .build()
            .await?;
        let type_info = subscriber.entity().type_info.clone();
        let metadata = Arc::new(SampleMetadata {
            topic_reference: topic,
            resolved_topic: resolved_topic.clone(),
            type_info: type_info.clone(),
        });
        let state = CachedSubscriptionState::spawn(
            CachedSubscriptionStatusSnapshot::with_metadata(
                CachedSubscriptionStatus::WaitingForFirstSample,
                resolved_topic,
                type_info,
            ),
            retention,
            move |state, cancellation| {
                receive_dynamic_loop(subscriber, state, cancellation, metadata)
            },
        );

        Ok(state.handle())
    }
}

async fn receive_typed_loop<T>(
    subscriber: ros_z::pubsub::Subscriber<T>,
    state: std::sync::Weak<CachedSubscriptionState<T>>,
    cancellation: CancellationToken,
    metadata: Arc<SampleMetadata>,
) where
    T: Message,
    T::Codec: Send + Sync,
{
    loop {
        let received = tokio::select! {
            result = subscriber.recv_with_metadata() => result,
            _ = cancellation.cancelled() => break,
        };

        let Some(state) = state.upgrade() else {
            break;
        };

        match received {
            Ok(received) => {
                let publication_id = received.publication_id();
                state.store_latest(Arc::new(SampleRecord {
                    value: received.message,
                    source_time: received.source_time,
                    transport_time: received.transport_time,
                    publication_id,
                    metadata: Arc::clone(&metadata),
                }));
            }
            Err(error) => {
                let message = receive_error_diagnostic(&error, &metadata);
                let status = classify_receive_error(&error, message);
                state.set_receive_error(status);
            }
        }
    }
}

async fn receive_dynamic_loop(
    subscriber: ros_z::pubsub::Subscriber<DynamicPayload, ros_z::dynamic::DynamicCdrCodec>,
    state: std::sync::Weak<CachedSubscriptionState<DynamicPayload>>,
    cancellation: CancellationToken,
    metadata: Arc<SampleMetadata>,
) {
    loop {
        let received = tokio::select! {
            result = subscriber.recv_with_metadata() => result,
            _ = cancellation.cancelled() => break,
        };

        let Some(state) = state.upgrade() else {
            break;
        };

        match received {
            Ok(received) => {
                let publication_id = received.publication_id();
                state.store_latest(Arc::new(SampleRecord {
                    value: received.message,
                    source_time: received.source_time,
                    transport_time: received.transport_time,
                    publication_id,
                    metadata: Arc::clone(&metadata),
                }));
            }
            Err(error) => {
                let message = receive_error_diagnostic(&error, &metadata);
                let status = classify_receive_error(&error, message);
                state.set_receive_error(status);
            }
        }
    }
}

fn receive_error_diagnostic(error: &ros_z::Error, metadata: &SampleMetadata) -> String {
    let mut message = format!(
        "{} while receiving '{}' as {}",
        error, metadata.resolved_topic, metadata.type_info.name
    );
    let mut source = error.source();

    while let Some(error) = source {
        let _ = write!(message, ": {error}");
        source = error.source();
    }

    message
}

fn classify_receive_error(error: &ros_z::Error, message: String) -> CachedSubscriptionStatus {
    if matches!(
        error,
        ros_z::Error::Wire(source) if matches!(
            source.as_ref(),
            ros_z::error::WireError::MissingSampleAttachment
                | ros_z::error::WireError::SampleAttachmentDecode { .. }
        )
    ) {
        CachedSubscriptionStatus::protocol_error(message)
    } else {
        CachedSubscriptionStatus::decode_error(message)
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_receive_error, receive_error_diagnostic};
    use crate::{CachedSubscriptionStatus, SampleMetadata, TopicReference};

    fn test_metadata() -> SampleMetadata {
        SampleMetadata {
            topic_reference: TopicReference::new("debug").unwrap(),
            resolved_topic: "/debug".to_string(),
            type_info: ros_z::TypeInfo::new("test_msgs::DebugValue", ros_z::SchemaHash::zero()),
        }
    }

    #[test]
    fn classify_missing_sample_attachment_as_protocol_error() {
        let error = ros_z::Error::from(ros_z::error::WireError::MissingSampleAttachment);

        assert!(matches!(
            classify_receive_error(&error, "missing attachment".to_string()),
            CachedSubscriptionStatus::ProtocolError { ref message } if message == "missing attachment"
        ));
    }

    #[test]
    fn classify_wire_decode_errors_as_decode_errors() {
        let error = ros_z::Error::from(ros_z::error::WireError::Decode {
            type_name: "test_msgs::DebugValue".to_string(),
            source: Box::new(std::io::Error::other("failed to deserialize cdr payload")),
        });

        assert!(matches!(
            classify_receive_error(&error, "decode failed".to_string()),
            CachedSubscriptionStatus::DecodeError { ref message } if message == "decode failed"
        ));
    }

    #[test]
    fn receive_error_diagnostic_includes_source_chain_and_metadata() {
        let error = ros_z::Error::from(ros_z::error::WireError::Decode {
            type_name: "test_msgs::DebugValue".to_string(),
            source: Box::new(std::io::Error::other("failed to deserialize cdr payload")),
        });

        let message = receive_error_diagnostic(&error, &test_metadata());

        assert!(message.contains("/debug"));
        assert!(message.contains("test_msgs::DebugValue"));
        assert!(message.contains("failed to deserialize cdr payload"));
    }
}

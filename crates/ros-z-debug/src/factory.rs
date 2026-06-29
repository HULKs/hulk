use std::{error::Error as _, fmt::Write as _, marker::PhantomData, sync::Arc, time::Duration};

use parking_lot::Mutex;
use ros_z::{Message, dynamic::DynamicPayload, node::Node};
use tokio_util::sync::CancellationToken;

use crate::{
    CachedJsonSubscription, CachedSubscription, CachedSubscriptionStatus,
    CachedSubscriptionStatusSnapshot, JsonRenderPolicy, Result, RetentionPolicy, SampleMetadata,
    SampleRecord, TargetIdentity, TopicReference,
    cache::{CachedSubscriptionState, ManagedCachedSubscription},
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CachedSubscriptionOptions {
    /// Target identity used to resolve topic references.
    ///
    /// This names the robot or target node being inspected, not the debug node
    /// that owns the subscriptions.
    target_identity: TargetIdentity,
    /// Timeout used while querying schema services for dynamic subscriptions.
    schema_discovery_timeout: Duration,
}

impl Default for CachedSubscriptionOptions {
    fn default() -> Self {
        Self {
            target_identity: TargetIdentity::new("/").expect("root namespace is valid"),
            schema_discovery_timeout: Duration::from_secs(5),
        }
    }
}

impl CachedSubscriptionOptions {
    /// Create options with a validated target namespace.
    pub fn with_target_namespace(namespace: impl Into<String>) -> Result<Self> {
        let mut options = Self::default();
        options.set_target_namespace(namespace)?;
        Ok(options)
    }

    /// Identity used to resolve topic references.
    pub fn target_identity(&self) -> &TargetIdentity {
        &self.target_identity
    }

    /// Namespace used to resolve relative topic references.
    pub fn target_namespace(&self) -> &str {
        self.target_identity.namespace()
    }

    /// Node name used to resolve private topic references, when configured.
    pub fn target_node_name(&self) -> Option<&str> {
        self.target_identity.node_name()
    }

    /// Timeout used while querying schema services for dynamic subscriptions.
    pub fn schema_discovery_timeout(&self) -> Duration {
        self.schema_discovery_timeout
    }

    /// Update the target namespace after validating it as a graph namespace.
    pub fn set_target_namespace(&mut self, namespace: impl Into<String>) -> Result<&mut Self> {
        self.target_identity.set_namespace(namespace)?;
        Ok(self)
    }

    /// Update the target node name after validating it as a graph component.
    pub fn set_target_node_name(&mut self, node_name: impl Into<String>) -> Result<&mut Self> {
        self.target_identity.set_node_name(node_name)?;
        Ok(self)
    }

    /// Clear the target node name used to resolve private topic references.
    pub fn clear_target_node_name(&mut self) -> &mut Self {
        self.target_identity.clear_node_name();
        self
    }

    /// Update the timeout used while querying schema services for dynamic subscriptions.
    pub fn set_schema_discovery_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.schema_discovery_timeout = timeout;
        self
    }
}

/// Owns debug subscriptions created from a `ros-z` node.
///
/// Handles returned by this factory retain the latest sample, optional history,
/// status, and live subscription update streams. Dropping the factory closes
/// subscriptions it can still reach; dropping the last handle for a subscription
/// cancels its receive task.
pub struct CachedSubscriptionFactory {
    node: Arc<Node>,
    options: CachedSubscriptionOptions,
    subscriptions: CachedSubscriptionRegistry,
}

impl CachedSubscriptionFactory {
    /// Create a factory for subscriptions owned by `node`.
    pub fn new(node: Arc<Node>, options: CachedSubscriptionOptions) -> Self {
        Self {
            node,
            options,
            subscriptions: CachedSubscriptionRegistry::default(),
        }
    }

    /// Start building a typed debug subscription.
    ///
    /// Relative topics resolve against [`CachedSubscriptionOptions::target_namespace`].
    /// Private topics resolve against [`CachedSubscriptionOptions::target_node_name`].
    /// The default retention policy is [`RetentionPolicy::LatestOnly`].
    pub fn subscribe_typed<T>(
        &self,
        topic: impl Into<String>,
    ) -> Result<CachedTypedSubscriptionBuilder<'_, T>> {
        Ok(CachedTypedSubscriptionBuilder {
            factory: self,
            topic: TopicReference::new(topic.into())?,
            retention: RetentionPolicy::LatestOnly,
            value: PhantomData,
        })
    }

    /// Start building a dynamic debug subscription.
    ///
    /// Relative topics resolve against [`CachedSubscriptionOptions::target_namespace`].
    /// Private topics resolve against [`CachedSubscriptionOptions::target_node_name`].
    /// Dynamic subscriptions discover the schema from currently visible
    /// publishers during `build()` or `build_json()`. Schema service queries use
    /// [`CachedSubscriptionOptions::schema_discovery_timeout`].
    pub fn subscribe_dynamic(
        &self,
        topic: impl Into<String>,
    ) -> Result<CachedDynamicSubscriptionBuilder<'_>> {
        Ok(CachedDynamicSubscriptionBuilder {
            factory: self,
            topic: TopicReference::new(topic.into())?,
            retention: RetentionPolicy::LatestOnly,
        })
    }

    pub(crate) fn node(&self) -> &Arc<Node> {
        &self.node
    }

    /// Namespace used to resolve relative topic references.
    pub fn target_namespace(&self) -> &str {
        self.options.target_namespace()
    }

    /// Node name used to resolve private topic references, when configured.
    pub fn target_node_name(&self) -> Option<&str> {
        self.options.target_node_name()
    }

    pub(crate) fn close(&self) {
        self.subscriptions.close_all();
    }
}

impl Drop for CachedSubscriptionFactory {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Default)]
pub(crate) struct CachedSubscriptionRegistry {
    subscriptions: Mutex<Vec<std::sync::Weak<dyn ManagedCachedSubscription>>>,
}

impl CachedSubscriptionRegistry {
    pub(crate) fn register<V>(&self, state: &Arc<CachedSubscriptionState<V>>)
    where
        V: Send + Sync + 'static,
    {
        let state: Arc<dyn ManagedCachedSubscription> = state.clone();
        let mut subscriptions = self.subscriptions.lock();
        subscriptions.retain(|subscription| subscription.strong_count() > 0);
        subscriptions.push(Arc::downgrade(&state));
    }

    pub(crate) fn close_all(&self) {
        self.subscriptions.lock().retain(|subscription| {
            if let Some(subscription) = subscription.upgrade() {
                subscription.close();
                true
            } else {
                false
            }
        });
    }
}

/// Builder for typed debug subscriptions.
pub struct CachedTypedSubscriptionBuilder<'a, T> {
    pub(crate) factory: &'a CachedSubscriptionFactory,
    pub(crate) topic: TopicReference,
    pub(crate) retention: RetentionPolicy,
    value: PhantomData<T>,
}

impl<T> CachedTypedSubscriptionBuilder<'_, T> {
    /// Configure how many samples the handle retains.
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Factory that will own the subscription.
    pub fn factory(&self) -> &CachedSubscriptionFactory {
        self.factory
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
    pub async fn build(self) -> Result<CachedSubscription<T>>
    where
        T: Message + Send + Sync + 'static,
        T::Codec: Send + Sync,
    {
        let Self {
            factory,
            topic,
            retention,
            value: _,
        } = self;
        let resolved_topic = topic.resolve(factory.options.target_identity())?;
        let subscriber = factory
            .node()
            .subscriber::<T>(&resolved_topic)
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
        let handle = state.handle();
        factory.subscriptions.register(&state);

        Ok(handle)
    }
}

/// Builder for dynamic debug subscriptions.
///
/// Dynamic builders use currently visible publishers to find one consistent
/// schema for the resolved topic. Building fails if no matching publisher/schema
/// is visible, or if schema service queries do not complete within the factory's
/// configured discovery timeout.
pub struct CachedDynamicSubscriptionBuilder<'a> {
    pub(crate) factory: &'a CachedSubscriptionFactory,
    pub(crate) topic: TopicReference,
    pub(crate) retention: RetentionPolicy,
}

impl CachedDynamicSubscriptionBuilder<'_> {
    /// Configure how many samples the handle retains.
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Factory that will own the subscription.
    pub fn factory(&self) -> &CachedSubscriptionFactory {
        self.factory
    }

    /// Topic reference requested by the caller.
    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }

    /// Configured retention policy.
    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }

    /// Build a dynamic payload subscription and spawn its receive task.
    ///
    /// Schema discovery requires at least one currently visible publisher on the
    /// resolved topic, consistent type metadata, and a reachable schema service.
    pub async fn build(self) -> Result<CachedSubscription<DynamicPayload>> {
        self.build_dynamic_payload().await
    }

    /// Build a dynamic JSON subscription with `policy`.
    ///
    /// Schema discovery has the same requirements as [`Self::build`].
    pub async fn build_json(self, policy: JsonRenderPolicy) -> Result<CachedJsonSubscription> {
        let dynamic = self.build_dynamic_payload().await?;
        Ok(CachedJsonSubscription::new(dynamic, policy))
    }

    async fn build_dynamic_payload(self) -> Result<CachedSubscription<DynamicPayload>> {
        let Self {
            factory,
            topic,
            retention,
        } = self;
        let resolved_topic = topic.resolve(factory.options.target_identity())?;
        let subscriber = factory
            .node()
            .dynamic_subscriber_auto(&resolved_topic, factory.options.schema_discovery_timeout())
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
        let handle = state.handle();
        factory.subscriptions.register(&state);

        Ok(handle)
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
    use std::sync::Arc;

    use super::{
        CachedSubscriptionOptions, CachedSubscriptionRegistry, classify_receive_error,
        receive_error_diagnostic,
    };
    use crate::{
        CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot, CachedSubscriptionUpdateClosed,
        Error, RetentionPolicy, SampleMetadata, TopicReference, cache::CachedSubscriptionState,
    };

    struct TestPayload;

    fn test_metadata() -> SampleMetadata {
        SampleMetadata {
            topic_reference: TopicReference::new("debug").unwrap(),
            resolved_topic: "/debug".to_string(),
            type_info: ros_z::TypeInfo::new("test_msgs::DebugValue", ros_z::SchemaHash::zero()),
        }
    }

    #[test]
    fn cached_subscription_options_reject_invalid_target_namespace() {
        let error = CachedSubscriptionOptions::with_target_namespace("alpha%bad").unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTargetNamespace { ref target_namespace, .. }
                if target_namespace == "alpha%bad"
        ));
    }

    #[test]
    fn cached_subscription_options_normalize_valid_target_namespace() {
        let options = CachedSubscriptionOptions::with_target_namespace("alpha/").unwrap();

        assert_eq!(options.target_namespace(), "/alpha");
    }

    #[test]
    fn cached_subscription_options_set_target_node_name() {
        let mut options = CachedSubscriptionOptions::with_target_namespace("/42").unwrap();

        options.set_target_node_name("behavior_node").unwrap();

        assert_eq!(options.target_node_name(), Some("behavior_node"));
    }

    #[test]
    fn cached_subscription_options_clear_target_node_name() {
        let mut options = CachedSubscriptionOptions::with_target_namespace("/42").unwrap();
        options.set_target_node_name("behavior_node").unwrap();

        options.clear_target_node_name();

        assert_eq!(options.target_node_name(), None);
    }

    #[test]
    fn cached_subscription_options_keep_previous_node_name_on_invalid_update() {
        let mut options = CachedSubscriptionOptions::with_target_namespace("/42").unwrap();
        options.set_target_node_name("behavior_node").unwrap();

        let error = options.set_target_node_name("bad%node").unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTargetNodeName { ref target_node_name, .. }
                if target_node_name == "bad%node"
        ));
        assert_eq!(options.target_node_name(), Some("behavior_node"));
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

    #[test]
    fn registry_closes_subscriptions_while_handles_remain_alive() {
        let registry = CachedSubscriptionRegistry::default();
        let state = Arc::new(CachedSubscriptionState::<TestPayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();
        registry.register(&state);
        drop(state);

        registry.close_all();

        assert_eq!(handle.status().status(), &CachedSubscriptionStatus::Closed);
        assert!(matches!(
            updates.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }
}

use std::{error::Error as _, fmt::Write as _, marker::PhantomData, sync::Arc, time::Duration};

use parking_lot::Mutex;
use ros_z::{Message, dynamic::DynamicPayload, node::Node};
use tokio_util::sync::CancellationToken;

use crate::{
    JsonRenderPolicy, JsonSubscriptionHandle, Result, RetentionPolicy, SampleMetadata,
    SampleRecord, SubscriptionHandle, SubscriptionStatus, SubscriptionStatusSnapshot,
    TopicSelector,
    subscription::{ManagedSubscription, SubscriptionState},
    topic::normalize_target_namespace,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ManagerOptions {
    /// Namespace used to resolve relative topic selectors.
    ///
    /// This names the robot or target namespace being inspected, not the
    /// namespace of the debug node that owns the subscriptions.
    target_namespace: String,
}

impl Default for ManagerOptions {
    fn default() -> Self {
        Self {
            target_namespace: "/".to_string(),
        }
    }
}

impl ManagerOptions {
    /// Create options with a validated target namespace.
    pub fn with_target_namespace(target_namespace: impl Into<String>) -> Result<Self> {
        let mut options = Self::default();
        options.set_target_namespace(target_namespace)?;
        Ok(options)
    }

    /// Namespace used to resolve relative topic selectors.
    pub fn target_namespace(&self) -> &str {
        &self.target_namespace
    }

    /// Update the target namespace after validating it as a ROS namespace.
    pub fn set_target_namespace(
        &mut self,
        target_namespace: impl Into<String>,
    ) -> Result<&mut Self> {
        let target_namespace = target_namespace.into();
        self.target_namespace = normalize_target_namespace(&target_namespace)?;
        Ok(self)
    }
}

/// Owns debug subscriptions created from a `ros-z` node.
///
/// Handles returned by this manager retain the latest sample, optional history,
/// status, and queued debug events. Dropping the manager closes subscriptions it
/// can still reach; dropping the last handle for a subscription also cancels its
/// receive task.
pub struct SubscriptionManager {
    node: Arc<Node>,
    options: ManagerOptions,
    subscriptions: SubscriptionRegistry,
}

impl SubscriptionManager {
    /// Create a manager for subscriptions owned by `node`.
    pub fn new(node: Arc<Node>, options: ManagerOptions) -> Self {
        Self {
            node,
            options,
            subscriptions: SubscriptionRegistry::default(),
        }
    }

    /// Start building a typed debug subscription.
    ///
    /// Relative topics resolve against [`ManagerOptions::target_namespace`].
    /// The default retention policy is [`RetentionPolicy::LatestOnly`].
    pub fn subscribe_typed<T>(&self, topic: impl Into<String>) -> TypedSubscriptionBuilder<'_, T> {
        TypedSubscriptionBuilder {
            manager: self,
            topic: topic.into(),
            retention: RetentionPolicy::LatestOnly,
            value: PhantomData,
        }
    }

    /// Start building a dynamic debug subscription.
    ///
    /// Relative topics resolve against [`ManagerOptions::target_namespace`].
    /// Dynamic subscriptions discover the schema from currently visible
    /// publishers during `build()` or `build_json()`. Schema service queries use
    /// a five second timeout.
    pub fn subscribe_dynamic(&self, topic: impl Into<String>) -> DynamicSubscriptionBuilder<'_> {
        DynamicSubscriptionBuilder {
            manager: self,
            topic: topic.into(),
            retention: RetentionPolicy::LatestOnly,
        }
    }

    pub(crate) fn node(&self) -> &Arc<Node> {
        &self.node
    }

    /// Namespace used to resolve relative topic selectors.
    pub fn target_namespace(&self) -> &str {
        self.options.target_namespace()
    }

    pub(crate) fn close(&self) {
        self.subscriptions.close_all();
    }
}

impl Drop for SubscriptionManager {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Default)]
pub(crate) struct SubscriptionRegistry {
    subscriptions: Mutex<Vec<std::sync::Weak<dyn ManagedSubscription>>>,
}

impl SubscriptionRegistry {
    pub(crate) fn register<V>(&self, state: &Arc<SubscriptionState<V>>)
    where
        V: Send + Sync + 'static,
    {
        let state: Arc<dyn ManagedSubscription> = state.clone();
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
pub struct TypedSubscriptionBuilder<'a, T> {
    pub(crate) manager: &'a SubscriptionManager,
    pub(crate) topic: String,
    pub(crate) retention: RetentionPolicy,
    value: PhantomData<T>,
}

impl<T> TypedSubscriptionBuilder<'_, T> {
    /// Configure how many samples the handle retains.
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Manager that will own the subscription.
    pub fn manager(&self) -> &SubscriptionManager {
        self.manager
    }

    /// Topic selector requested by the caller.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Configured retention policy.
    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }

    /// Build the typed subscription and spawn its receive task.
    pub async fn build(self) -> Result<SubscriptionHandle<T>>
    where
        T: Message + Send + Sync + 'static,
        T::Codec: Send + Sync,
    {
        let retention = self.retention;
        let requested_topic = TopicSelector::new(self.topic)?;
        let resolved_topic = requested_topic.resolve(self.manager.target_namespace())?;
        let subscriber = self
            .manager
            .node()
            .subscriber::<T>(&resolved_topic)?
            .build()
            .await?;
        let type_info = subscriber.entity().type_info.clone();
        let metadata = Arc::new(SampleMetadata {
            requested_topic,
            resolved_topic: resolved_topic.clone(),
            type_info: type_info.clone(),
        });
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::with_metadata(
                SubscriptionStatus::WaitingForFirstSample,
                resolved_topic,
                type_info,
            ),
            retention,
        ));
        let handle = state.handle();
        self.manager.subscriptions.register(&state);

        let receive_task = tokio::spawn(receive_typed_loop(
            subscriber,
            Arc::downgrade(&state),
            state.cancellation_token(),
            metadata,
        ));
        state.set_receive_task(receive_task.abort_handle());

        Ok(handle)
    }
}

/// Builder for dynamic debug subscriptions.
///
/// Dynamic builders use currently visible publishers to find one consistent
/// schema for the resolved topic. Building fails if no matching publisher/schema
/// is visible, or if schema service queries do not complete within five seconds.
pub struct DynamicSubscriptionBuilder<'a> {
    pub(crate) manager: &'a SubscriptionManager,
    pub(crate) topic: String,
    pub(crate) retention: RetentionPolicy,
}

impl DynamicSubscriptionBuilder<'_> {
    /// Configure how many samples the handle retains.
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Manager that will own the subscription.
    pub fn manager(&self) -> &SubscriptionManager {
        self.manager
    }

    /// Topic selector requested by the caller.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Configured retention policy.
    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }

    /// Build a dynamic payload subscription and spawn its receive task.
    ///
    /// Schema discovery requires at least one currently visible publisher on the
    /// resolved topic, consistent type metadata, and a reachable schema service.
    pub async fn build(self) -> Result<SubscriptionHandle<DynamicPayload>> {
        self.build_dynamic_payload().await
    }

    /// Build a dynamic JSON subscription with `policy`.
    ///
    /// Schema discovery has the same requirements as [`Self::build`].
    pub async fn build_json(self, policy: JsonRenderPolicy) -> Result<JsonSubscriptionHandle> {
        let dynamic = self.build_dynamic_payload().await?;
        Ok(JsonSubscriptionHandle::new(dynamic, policy))
    }

    async fn build_dynamic_payload(self) -> Result<SubscriptionHandle<DynamicPayload>> {
        let retention = self.retention;
        let requested_topic = TopicSelector::new(self.topic)?;
        let resolved_topic = requested_topic.resolve(self.manager.target_namespace())?;
        let subscriber = self
            .manager
            .node()
            .dynamic_subscriber_auto(&resolved_topic, Duration::from_secs(5))
            .await?
            .build()
            .await?;
        let type_info = subscriber.entity().type_info.clone();
        let metadata = Arc::new(SampleMetadata {
            requested_topic,
            resolved_topic: resolved_topic.clone(),
            type_info: type_info.clone(),
        });
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::with_metadata(
                SubscriptionStatus::WaitingForFirstSample,
                resolved_topic,
                type_info,
            ),
            retention,
        ));
        let handle = state.handle();
        self.manager.subscriptions.register(&state);

        let receive_task = tokio::spawn(receive_dynamic_loop(
            subscriber,
            Arc::downgrade(&state),
            state.cancellation_token(),
            metadata,
        ));
        state.set_receive_task(receive_task.abort_handle());

        Ok(handle)
    }
}

async fn receive_typed_loop<T>(
    subscriber: ros_z::pubsub::Subscriber<T>,
    state: std::sync::Weak<SubscriptionState<T>>,
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
    state: std::sync::Weak<SubscriptionState<DynamicPayload>>,
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

fn classify_receive_error(error: &ros_z::Error, message: String) -> SubscriptionStatus {
    if matches!(
        error,
        ros_z::Error::Wire(source) if matches!(
            source.as_ref(),
            ros_z::error::WireError::MissingSampleAttachment
                | ros_z::error::WireError::SampleAttachmentDecode { .. }
        )
    ) {
        SubscriptionStatus::protocol_error(message)
    } else {
        SubscriptionStatus::decode_error(message)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        ManagerOptions, SubscriptionRegistry, classify_receive_error, receive_error_diagnostic,
    };
    use crate::{
        Error, RetentionPolicy, SampleMetadata, SubscriptionStatus, SubscriptionStatusSnapshot,
        TopicSelector, subscription::SubscriptionState,
    };

    struct TestPayload;

    fn test_metadata() -> SampleMetadata {
        SampleMetadata {
            requested_topic: TopicSelector::new("debug").unwrap(),
            resolved_topic: "/debug".to_string(),
            type_info: ros_z::TypeInfo::new("test_msgs::DebugValue", ros_z::SchemaHash::zero()),
        }
    }

    #[test]
    fn manager_options_reject_invalid_target_namespace() {
        let error = ManagerOptions::with_target_namespace("123invalid").unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTargetNamespace { ref target_namespace, .. }
                if target_namespace == "123invalid"
        ));
    }

    #[test]
    fn manager_options_normalize_valid_target_namespace() {
        let options = ManagerOptions::with_target_namespace("alpha/").unwrap();

        assert_eq!(options.target_namespace(), "/alpha");
    }

    #[test]
    fn classify_missing_sample_attachment_as_protocol_error() {
        let error = ros_z::Error::from(ros_z::error::WireError::MissingSampleAttachment);

        assert!(matches!(
            classify_receive_error(&error, "missing attachment".to_string()),
            SubscriptionStatus::ProtocolError { ref message } if message == "missing attachment"
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
            SubscriptionStatus::DecodeError { ref message } if message == "decode failed"
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
        let registry = SubscriptionRegistry::default();
        let state = Arc::new(SubscriptionState::<TestPayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        registry.register(&state);
        drop(state);

        registry.close_all();

        assert_eq!(handle.status().status(), &SubscriptionStatus::Closed);
        assert!(matches!(
            handle.drain_events().as_slice(),
            [crate::DebugEvent::StatusChanged]
        ));
    }
}

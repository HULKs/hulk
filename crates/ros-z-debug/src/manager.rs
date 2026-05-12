use std::{marker::PhantomData, sync::Arc, time::Duration};

use parking_lot::Mutex;
use ros_z::{Message, dynamic::DynamicPayload, node::Node, pubsub::QueueLossStats};
use tokio::sync::watch;

use crate::{
    JsonRenderPolicy, JsonSubscriptionHandle, Result, RetentionPolicy, SampleRecord,
    SubscriptionHandle, SubscriptionStatus, SubscriptionStatusSnapshot, TopicSelector,
    subscription::{ManagedSubscription, SubscriptionState},
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ManagerOptions {
    pub namespace: String,
}

impl Default for ManagerOptions {
    fn default() -> Self {
        Self {
            namespace: "/".to_string(),
        }
    }
}

pub struct SubscriptionManager {
    node: Arc<Node>,
    options: ManagerOptions,
    subscriptions: SubscriptionRegistry,
}

impl SubscriptionManager {
    pub fn new(node: Arc<Node>, options: ManagerOptions) -> Self {
        Self {
            node,
            options,
            subscriptions: SubscriptionRegistry::default(),
        }
    }

    pub fn subscribe_typed<T>(&self, topic: impl Into<String>) -> TypedSubscriptionBuilder<'_, T> {
        TypedSubscriptionBuilder {
            manager: self,
            topic: topic.into(),
            retention: RetentionPolicy::LatestOnly,
            value: PhantomData,
        }
    }

    pub fn subscribe_dynamic(&self, topic: impl Into<String>) -> DynamicSubscriptionBuilder<'_> {
        DynamicSubscriptionBuilder {
            manager: self,
            topic: topic.into(),
            retention: RetentionPolicy::LatestOnly,
            json: None,
        }
    }

    pub(crate) fn node(&self) -> &Arc<Node> {
        &self.node
    }

    pub fn namespace(&self) -> &str {
        &self.options.namespace
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
        self.subscriptions.lock().push(Arc::downgrade(&state));
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

pub struct TypedSubscriptionBuilder<'a, T> {
    pub(crate) manager: &'a SubscriptionManager,
    pub(crate) topic: String,
    pub(crate) retention: RetentionPolicy,
    value: PhantomData<T>,
}

impl<T> TypedSubscriptionBuilder<'_, T> {
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    pub fn manager(&self) -> &SubscriptionManager {
        self.manager
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }

    pub async fn build(self) -> Result<SubscriptionHandle<T>>
    where
        T: Message + Send + Sync + 'static,
        T::Codec: Send + Sync,
    {
        let retention = self.retention.validate()?;
        let requested_topic = TopicSelector::new(self.topic)?;
        let resolved_topic = requested_topic.resolve(self.manager.namespace())?;
        let subscriber = self
            .manager
            .node()
            .subscriber::<T>(&resolved_topic)
            .build()
            .await?;
        let type_info = subscriber.entity().type_info.clone();
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot {
                status: SubscriptionStatus::WaitingForFirstSample,
                message: None,
                resolved_topic: Some(resolved_topic.clone()),
                type_info: type_info.clone(),
            },
            retention,
        ));
        let handle = state.handle();
        self.manager.subscriptions.register(&state);

        let receive_task = tokio::spawn(receive_typed_loop(
            subscriber,
            Arc::downgrade(&state),
            state.cancellation_receiver(),
            requested_topic,
            resolved_topic,
            type_info,
        ));
        state.set_receive_task(receive_task.abort_handle());

        Ok(handle)
    }
}

pub struct DynamicSubscriptionBuilder<'a> {
    pub(crate) manager: &'a SubscriptionManager,
    pub(crate) topic: String,
    pub(crate) retention: RetentionPolicy,
    pub(crate) json: Option<JsonRenderPolicy>,
}

impl DynamicSubscriptionBuilder<'_> {
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    pub fn json(mut self, policy: JsonRenderPolicy) -> Self {
        self.json = Some(policy);
        self
    }

    pub fn manager(&self) -> &SubscriptionManager {
        self.manager
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }

    pub fn json_policy(&self) -> Option<JsonRenderPolicy> {
        self.json
    }

    pub async fn build(self) -> Result<SubscriptionHandle<DynamicPayload>> {
        self.build_payload().await
    }

    pub async fn build_payload(self) -> Result<SubscriptionHandle<DynamicPayload>> {
        self.build_dynamic_payload().await
    }

    pub async fn build_json(self) -> Result<JsonSubscriptionHandle> {
        let policy = self.json.unwrap_or_default();
        let dynamic = self.build_dynamic_payload().await?;
        Ok(JsonSubscriptionHandle::new(dynamic, policy))
    }

    async fn build_dynamic_payload(self) -> Result<SubscriptionHandle<DynamicPayload>> {
        let retention = self.retention.validate()?;
        let requested_topic = TopicSelector::new(self.topic)?;
        let resolved_topic = requested_topic.resolve(self.manager.namespace())?;
        let subscriber = self
            .manager
            .node()
            .dynamic_subscriber_auto(&resolved_topic, Duration::from_secs(5))
            .await?
            .build()
            .await?;
        let type_info = subscriber.entity().type_info.clone();
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot {
                status: SubscriptionStatus::WaitingForFirstSample,
                message: None,
                resolved_topic: Some(resolved_topic.clone()),
                type_info: type_info.clone(),
            },
            retention,
        ));
        let handle = state.handle();
        self.manager.subscriptions.register(&state);

        let receive_task = tokio::spawn(receive_dynamic_loop(
            subscriber,
            Arc::downgrade(&state),
            state.cancellation_receiver(),
            requested_topic,
            resolved_topic,
            type_info,
        ));
        state.set_receive_task(receive_task.abort_handle());

        Ok(handle)
    }
}

async fn receive_typed_loop<T>(
    subscriber: ros_z::pubsub::Subscriber<T>,
    state: std::sync::Weak<SubscriptionState<T>>,
    mut cancellation: watch::Receiver<()>,
    requested_topic: TopicSelector,
    resolved_topic: String,
    type_info: Option<ros_z::TypeInfo>,
) where
    T: Message,
    T::Codec: Send + Sync,
{
    let mut observed_dropped_samples = 0;

    loop {
        let received = tokio::select! {
            result = subscriber.recv_with_metadata() => result,
            _ = cancellation.changed() => break,
        };

        let Some(state) = state.upgrade() else {
            break;
        };

        report_queue_loss(
            &state,
            &mut observed_dropped_samples,
            subscriber.queue_loss_stats(),
        );

        match received {
            Ok(received) => {
                let publication_id = received.publication_id();
                state.store_latest(Arc::new(SampleRecord {
                    value: received.message,
                    source_time: received.source_time,
                    transport_time: received.transport_time,
                    publication_id: Some(publication_id),
                    source_global_id: Some(received.source_global_id),
                    requested_topic: requested_topic.clone(),
                    resolved_topic: resolved_topic.clone(),
                    namespace_version: 0,
                    type_info: type_info.clone(),
                    schema: None,
                }));
            }
            Err(error) => {
                let message = error.to_string();
                state.set_receive_error(classify_receive_error(&message), message);
            }
        }
    }
}

async fn receive_dynamic_loop(
    subscriber: ros_z::pubsub::Subscriber<DynamicPayload, ros_z::dynamic::DynamicCdrCodec>,
    state: std::sync::Weak<SubscriptionState<DynamicPayload>>,
    mut cancellation: watch::Receiver<()>,
    requested_topic: TopicSelector,
    resolved_topic: String,
    type_info: Option<ros_z::TypeInfo>,
) {
    let mut observed_dropped_samples = 0;

    loop {
        let received = tokio::select! {
            result = subscriber.recv_with_metadata() => result,
            _ = cancellation.changed() => break,
        };

        let Some(state) = state.upgrade() else {
            break;
        };

        report_queue_loss(
            &state,
            &mut observed_dropped_samples,
            subscriber.queue_loss_stats(),
        );

        match received {
            Ok(received) => {
                let publication_id = received.publication_id();
                state.store_latest(Arc::new(SampleRecord {
                    value: received.message,
                    source_time: received.source_time,
                    transport_time: received.transport_time,
                    publication_id: Some(publication_id),
                    source_global_id: Some(received.source_global_id),
                    requested_topic: requested_topic.clone(),
                    resolved_topic: resolved_topic.clone(),
                    namespace_version: 0,
                    type_info: type_info.clone(),
                    schema: None,
                }));
            }
            Err(error) => {
                let message = error.to_string();
                state.set_receive_error(classify_receive_error(&message), message);
            }
        }
    }
}

fn report_queue_loss<V>(
    state: &SubscriptionState<V>,
    observed_dropped_samples: &mut u64,
    stats: QueueLossStats,
) {
    if stats.dropped_samples <= *observed_dropped_samples {
        return;
    }

    let newly_dropped = stats.dropped_samples - *observed_dropped_samples;
    *observed_dropped_samples = stats.dropped_samples;
    state.push_event(crate::DebugEvent::Diagnostic(format!(
        "subscriber queue dropped {newly_dropped} sample(s); total dropped {}",
        stats.dropped_samples
    )));
}

fn classify_receive_error(message: &str) -> SubscriptionStatus {
    if message.contains("without attachment metadata")
        || message.contains("failed to decode ros-z attachment metadata")
    {
        SubscriptionStatus::ProtocolError
    } else {
        SubscriptionStatus::DecodeError
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::pubsub::QueueLossStats;

    use super::{SubscriptionRegistry, classify_receive_error, report_queue_loss};
    use crate::{
        RetentionPolicy, SubscriptionStatus, SubscriptionStatusSnapshot,
        subscription::SubscriptionState,
    };

    struct TestPayload;

    #[test]
    fn classify_attachment_metadata_errors_as_protocol_errors() {
        assert_eq!(
            classify_receive_error("received ros-z sample without attachment metadata"),
            SubscriptionStatus::ProtocolError
        );
        assert_eq!(
            classify_receive_error("failed to decode ros-z attachment metadata: invalid bytes"),
            SubscriptionStatus::ProtocolError
        );
    }

    #[test]
    fn classify_other_receive_errors_as_decode_errors() {
        assert_eq!(
            classify_receive_error("failed to deserialize cdr payload"),
            SubscriptionStatus::DecodeError
        );
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

        assert_eq!(handle.status().status, SubscriptionStatus::Closed);
        assert!(matches!(
            handle.drain_events().as_slice(),
            [crate::DebugEvent::StatusChanged]
        ));
    }

    #[test]
    fn queue_loss_diagnostic_reports_new_drops_once() {
        let state = Arc::new(SubscriptionState::<TestPayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut observed_dropped_samples = 0;

        report_queue_loss(
            &state,
            &mut observed_dropped_samples,
            QueueLossStats { dropped_samples: 2 },
        );
        report_queue_loss(
            &state,
            &mut observed_dropped_samples,
            QueueLossStats { dropped_samples: 2 },
        );
        report_queue_loss(
            &state,
            &mut observed_dropped_samples,
            QueueLossStats { dropped_samples: 5 },
        );

        let events = handle.drain_events();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            crate::DebugEvent::Diagnostic(message)
                if message == "subscriber queue dropped 2 sample(s); total dropped 2"
        ));
        assert!(matches!(
            &events[1],
            crate::DebugEvent::Diagnostic(message)
                if message == "subscriber queue dropped 3 sample(s); total dropped 5"
        ));
    }
}

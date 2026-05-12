use std::{marker::PhantomData, sync::Arc, time::Duration};

use ros_z::{Message, dynamic::DynamicPayload, node::Node};
use tokio::sync::watch;

use crate::{
    JsonRenderPolicy, Result, RetentionPolicy, SampleRecord, SubscriptionHandle,
    SubscriptionStatus, SubscriptionStatusSnapshot, TopicSelector, subscription::SubscriptionState,
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
}

impl SubscriptionManager {
    pub fn new(node: Arc<Node>, options: ManagerOptions) -> Self {
        Self { node, options }
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

    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }

    pub fn namespace(&self) -> &str {
        &self.options.namespace
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
        T: Message,
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

        tokio::spawn(receive_typed_loop(
            subscriber,
            Arc::downgrade(&state),
            state.cancellation_receiver(),
            requested_topic,
            resolved_topic,
            type_info,
        ));

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

        tokio::spawn(receive_dynamic_loop(
            subscriber,
            Arc::downgrade(&state),
            state.cancellation_receiver(),
            requested_topic,
            resolved_topic,
            type_info,
        ));

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
    loop {
        let received = tokio::select! {
            result = subscriber.recv_with_metadata() => result,
            _ = cancellation.changed() => break,
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
    loop {
        let received = tokio::select! {
            result = subscriber.recv_with_metadata() => result,
            _ = cancellation.changed() => break,
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
    use super::classify_receive_error;
    use crate::SubscriptionStatus;

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
}

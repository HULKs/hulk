use serde::Deserialize;
use std::marker::PhantomData;
use tracing::warn;
use zenoh::{
    handlers::{RingChannel, RingChannelHandler},
    pubsub::Subscriber as ZenohSubscriber,
    sample::Sample,
};

use crate::{topic::TopicError, Message, Node, Topic};

#[derive(Debug, thiserror::Error)]
pub enum SubscriberError {
    #[error("Failed to deserialize message: {0}")]
    Deserialization(#[from] cdr::Error),
    #[error("Zenoh transport error: {0}")]
    Zenoh(#[from] zenoh::Error),
    #[error("Topic error: {0}")]
    Topic(#[from] TopicError),
}

pub type Result<T, E = SubscriberError> = std::result::Result<T, E>;

pub struct SubscriberBuilder<T> {
    pub(crate) node: Node,
    pub topic: Result<Topic, TopicError>,
    pub capacity: usize,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> SubscriberBuilder<T> {
    pub async fn build(self) -> Result<Subscriber<T>> {
        let topic = self.topic?;

        let subscriber = self
            .node
            .session()
            .zenoh()
            .declare_subscriber(topic.qualify(self.node.session().namespace(), self.node.name()))
            .with(RingChannel::new(self.capacity))
            .await?;

        Ok(Subscriber {
            node: self.node,
            sub: subscriber,
            _phantom: PhantomData,
        })
    }
}

pub struct Subscriber<T> {
    node: Node,
    sub: ZenohSubscriber<RingChannelHandler<Sample>>,
    _phantom: PhantomData<T>,
}

impl<T> Subscriber<T>
where
    for<'de> T: Deserialize<'de>,
{
    pub async fn recv_async(&mut self) -> Result<Message<T>> {
        let sample = self
            .sub
            .recv_async()
            .await
            .map_err(SubscriberError::Zenoh)?;

        let payload = cdr::deserialize(&sample.payload().to_bytes())
            .map_err(SubscriberError::Deserialization)?;
        let timestamp = sample.timestamp().copied().unwrap_or_else(|| {
            warn!("Sample has no timestamp, using current time instead");
            self.node.session().now()
        });
        let message = Message { timestamp, payload };
        Ok(message)
    }
}

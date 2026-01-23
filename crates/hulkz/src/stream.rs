use serde::Deserialize;
use std::marker::PhantomData;
use tracing::warn;
use zenoh::{handlers::RingChannelHandler, pubsub::Subscriber as ZenohSubscriber, sample::Sample};

use crate::Message;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Failed to deserialize message: {0}")]
    Deserialization(#[from] cdr::Error),
    #[error("Zenoh transport error: {0}")]
    Zenoh(#[from] zenoh::Error),
}

pub type Result<T, E = StreamError> = std::result::Result<T, E>;

pub struct TopicStream<T> {
    sub: ZenohSubscriber<RingChannelHandler<Sample>>,
    session: zenoh::Session,
    _phantom: PhantomData<T>,
}

impl<T> TopicStream<T>
where
    for<'de> T: Deserialize<'de>,
{
    pub fn new(sub: ZenohSubscriber<RingChannelHandler<Sample>>, session: zenoh::Session) -> Self {
        Self {
            sub,
            session,
            _phantom: PhantomData,
        }
    }

    pub async fn recv_async(&mut self) -> Result<Message<T>> {
        let sample = self.sub.recv_async().await.map_err(StreamError::Zenoh)?;

        let payload =
            cdr::deserialize(&sample.payload().to_bytes()).map_err(StreamError::Deserialization)?;
        let timestamp = sample.timestamp().copied().unwrap_or_else(|| {
            warn!("Sample has no timestamp, using current time instead");
            self.session.new_timestamp()
        });
        let message = Message { timestamp, payload };
        Ok(message)
    }
}

use serde::Deserialize;
use std::marker::PhantomData;
use zenoh::{handlers::RingChannelHandler, pubsub::Subscriber as ZenohSubscriber, sample::Sample};

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Failed to deserialize message: {0}")]
    Deserialization(#[from] cdr::Error),
    #[error("Zenoh transport error: {0}")]
    Zenoh(#[from] zenoh::Error),
}

pub type Result<T, E = StreamError> = std::result::Result<T, E>;

pub struct TopicStream<T> {
    inner: ZenohSubscriber<RingChannelHandler<Sample>>,
    _phantom: PhantomData<T>,
}

impl<T> TopicStream<T>
where
    for<'de> T: Deserialize<'de>,
{
    pub fn new(sub: ZenohSubscriber<RingChannelHandler<Sample>>) -> Self {
        Self {
            inner: sub,
            _phantom: PhantomData,
        }
    }

    pub async fn recv_async(&mut self) -> Result<T> {
        let sample = self.inner.recv_async().await.map_err(StreamError::Zenoh)?;

        let value = cdr::deserialize::<T>(&sample.payload().to_bytes())
            .map_err(StreamError::Deserialization)?;

        Ok(value)
    }
}

// impl<T> Stream for TopicStream<T>
// where
//     T:,
// {
//     type Item = T;
//
//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//          todo!()
//     }
// }

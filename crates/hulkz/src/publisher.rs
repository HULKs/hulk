use cdr::{CdrLe, Infinite};
use serde::Serialize;
use std::marker::PhantomData;
use tracing::debug;
use zenoh::{bytes::Encoding, pubsub::Publisher as ZenohPublisher};

#[derive(Debug, thiserror::Error)]
pub enum PublisherError {
    #[error("Failed to serialize data: {0}")]
    Serialization(#[from] cdr::Error),
    #[error("Zenoh transport error: {0}")]
    Zenoh(#[from] zenoh::Error),
}

pub type Result<T, E = PublisherError> = std::result::Result<T, E>;

pub struct Publisher<'a, T>
where
    T: Serialize,
{
    publisher: ZenohPublisher<'a>,
    _phantom: PhantomData<T>,
}

impl<'a, T> Publisher<'a, T>
where
    T: Serialize,
{
    pub fn new(publisher: ZenohPublisher<'a>) -> Self {
        Self {
            publisher,
            _phantom: PhantomData,
        }
    }

    pub async fn is_subscribed(&self) -> Result<bool> {
        let status = self
            .publisher
            .matching_status()
            .await
            .map_err(PublisherError::Zenoh)?;
        Ok(status.matching())
    }

    #[tracing::instrument(skip(self, value), level = "debug", err)]
    pub async fn put(&self, value: &T) -> Result<()> {
        debug!("Publishing value to topic");
        let payload = cdr::serialize::<_, _, CdrLe>(value, Infinite)
            .map_err(PublisherError::Serialization)?;

        self.publisher
            .put(payload)
            .encoding(Encoding::APPLICATION_CDR)
            .await?;
        Ok(())
    }

    pub async fn put_with_subscription(&self, mut value: impl FnMut() -> T) -> Result<()> {
        if self.is_subscribed().await? {
            let value = value();
            self.put(&value).await?;
        }
        Ok(())
    }
}

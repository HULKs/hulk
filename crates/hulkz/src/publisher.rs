use cdr::{CdrLe, Infinite};
use serde::Serialize;
use std::marker::PhantomData;
use tracing::debug;
use zenoh::{bytes::Encoding, pubsub::Publisher as ZenohPublisher};

use crate::{topic::TopicError, Node, Topic};

#[derive(Debug, thiserror::Error)]
pub enum PublisherError {
    #[error("Failed to serialize data to CDR: {0}")]
    CdrSerialization(#[from] cdr::Error),
    #[error("Failed to serialize data to JSON: {0}")]
    JsonSerialization(#[from] serde_json::Error),
    #[error("Zenoh transport error: {0}")]
    Zenoh(#[from] zenoh::Error),
    #[error("Topic error: {0}")]
    Topic(#[from] TopicError),
}

pub type Result<T, E = PublisherError> = std::result::Result<T, E>;

pub struct PublisherBuilder<T>
where
    T: Serialize,
{
    pub(crate) node: Node,
    pub topic: Result<Topic, TopicError>,
    pub enable_json: bool,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> PublisherBuilder<T>
where
    T: Serialize,
{
    pub fn enable_json(mut self) -> Self {
        self.enable_json = true;
        self
    }

    pub fn disable_json(mut self) -> Self {
        self.enable_json = false;
        self
    }

    pub async fn build(self) -> Result<Publisher<T>> {
        let topic = self.topic?;

        let publisher = self
            .node
            .session()
            .zenoh()
            .declare_publisher(topic.qualify(self.node.session().namespace(), self.node.name()))
            .await?;

        let json_publisher = if self.enable_json {
            let key_expression = topic.qualify(self.node.session().namespace(), self.node.name());
            Some(
                self.node
                    .session()
                    .zenoh()
                    .declare_publisher(format!("{}.json", key_expression))
                    .await?,
            )
        } else {
            None
        };

        Ok(Publisher {
            publisher,
            json_publisher,
            _phantom: PhantomData,
        })
    }
}

pub struct Publisher<T>
where
    T: Serialize,
{
    publisher: ZenohPublisher<'static>,
    json_publisher: Option<ZenohPublisher<'static>>,
    _phantom: PhantomData<T>,
}

impl<T> Publisher<T>
where
    T: Serialize,
{
    pub async fn is_subscribed(&self) -> Result<bool> {
        let cdr_matching = self.is_cdr_subscribed().await?;
        let json_matching = self.is_json_subscribed().await?;
        Ok(cdr_matching || json_matching)
    }

    async fn is_cdr_subscribed(&self) -> Result<bool, PublisherError> {
        let cdr_matching = self
            .publisher
            .matching_status()
            .await
            .map_err(PublisherError::Zenoh)?
            .matching();
        Ok(cdr_matching)
    }

    async fn is_json_subscribed(&self) -> Result<bool, PublisherError> {
        let json_matching = if let Some(json_publisher) = &self.json_publisher {
            json_publisher
                .matching_status()
                .await
                .map_err(PublisherError::Zenoh)?
                .matching()
        } else {
            false
        };
        Ok(json_matching)
    }

    pub async fn put(&self, value: &T) -> Result<()> {
        debug!("Publishing value to topic");
        let payload = cdr::serialize::<_, _, CdrLe>(value, Infinite)
            .map_err(PublisherError::CdrSerialization)?;

        self.publisher
            .put(payload)
            .encoding(Encoding::APPLICATION_CDR)
            .await?;
        self.put_json(value).await?;
        Ok(())
    }

    async fn put_json(&self, value: &T) -> Result<()> {
        let Some(json_publisher) = &self.json_publisher else {
            return Ok(());
        };

        let is_matched = json_publisher
            .matching_status()
            .await
            .map_err(PublisherError::Zenoh)?
            .matching();
        if !is_matched {
            return Ok(());
        }

        let json_payload = serde_json::to_vec(value).map_err(PublisherError::JsonSerialization)?;
        json_publisher
            .put(json_payload)
            .encoding(Encoding::APPLICATION_JSON)
            .await?;
        Ok(())
    }

    pub async fn put_if_subscribed(&self, mut value: impl FnMut() -> T) -> Result<()> {
        if self.is_subscribed().await? {
            let value = value();
            self.put(&value).await?;
            self.put_json(&value).await?;
        }
        Ok(())
    }
}

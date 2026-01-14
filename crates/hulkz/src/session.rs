use std::{future::Future, sync::Arc};

use serde::{Deserialize, Serialize};
use zenoh::handlers::RingChannel;

use crate::{Publisher, Timestamped, TopicBuffer, TopicStream};

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Zenoh session error: {0}")]
    Zenoh(#[from] zenoh::Error),
}

pub type Result<T> = std::result::Result<T, SessionError>;

pub struct Session {
    session: Arc<zenoh::Session>,
}

impl Session {
    #[tracing::instrument]
    pub async fn new() -> Result<Self> {
        tracing::info!("Opening new Zenoh session");

        let config = if std::env::var(zenoh::Config::DEFAULT_CONFIG_PATH_ENV).is_ok() {
            zenoh::Config::from_env()?
        } else {
            zenoh::Config::default()
        };

        let session = zenoh::open(config).await?;
        Ok(Self {
            session: Arc::new(session),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn stream<T>(&self, key_expr: &str) -> Result<TopicStream<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        tracing::debug!(topic = key_expr, "Declaring subscriber");
        let subscriber = self
            .session
            .declare_subscriber(key_expr)
            .with(RingChannel::new(10))
            .await?;

        Ok(TopicStream::new(subscriber))
    }

    #[tracing::instrument(skip(self))]
    pub async fn buffer<T>(
        &self,
        key_exp: &str,
        capacity: usize,
    ) -> Result<(TopicBuffer<T>, impl Future<Output = ()>)>
    where
        for<'de> T: Deserialize<'de> + Timestamped + Clone + Send + 'static,
    {
        tracing::debug!(topic = key_exp, capacity, "Creating topic buffer");
        let stream = self.stream::<T>(key_exp).await?;
        let (buffer, driver) = TopicBuffer::new(stream, capacity);
        Ok((buffer, driver))
    }

    #[tracing::instrument(skip(self))]
    pub async fn publish<'a, T>(&self, key_expr: &'a str) -> Result<Publisher<'a, T>>
    where
        T: Serialize,
    {
        tracing::debug!(topic = key_expr, "Declaring publisher");
        let publisher = self.session.declare_publisher(key_expr).await?;
        Ok(Publisher::new(publisher))
    }
}

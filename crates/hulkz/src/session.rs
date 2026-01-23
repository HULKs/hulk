use std::future::Future;

use serde::{Deserialize, Serialize};
use zenoh::handlers::RingChannel;

use crate::{buffer::BufferError, Parameters, Publisher, Timestamp, TopicBuffer, TopicStream};

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Zenoh session error: {0}")]
    Zenoh(#[from] zenoh::Error),
    #[error("Parameter error: {0}")]
    Parameter(#[from] crate::parameter::ParameterError),
}

pub type Result<T, E = SessionError> = std::result::Result<T, E>;

pub struct Session {
    session: zenoh::Session,
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
        Ok(Self { session })
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

        Ok(TopicStream::new(subscriber, self.session.clone()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn buffer<T>(
        &self,
        key_exp: &str,
        capacity: usize,
    ) -> Result<(
        TopicBuffer<T>,
        impl Future<Output = Result<(), BufferError>>,
    )>
    where
        for<'de> T: Deserialize<'de> + Clone + Send + 'static,
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

    #[tracing::instrument(skip(self))]
    pub async fn parameters<T>(&self) -> Result<Parameters<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        tracing::debug!("Loading parameters");
        let parameters = Parameters::load().await?;
        Ok(parameters)
    }

    #[tracing::instrument(skip(self))]
    pub fn now(&self) -> Timestamp {
        self.session.new_timestamp()
    }
}

use std::sync::Arc;

use crate::{node::NodeBuilder, Timestamp};

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Zenoh session error: {0}")]
    Zenoh(#[from] zenoh::Error),
}

pub type Result<T, E = SessionError> = std::result::Result<T, E>;

pub struct SessionBuilder {
    namespace: String,
}

impl SessionBuilder {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
        }
    }

    pub async fn build(self) -> Result<Session> {
        tracing::info!("Opening new Zenoh session");

        let config = if std::env::var(zenoh::Config::DEFAULT_CONFIG_PATH_ENV).is_ok() {
            zenoh::Config::from_env()?
        } else {
            zenoh::Config::default()
        };

        let session = zenoh::open(config).await?;
        let inner = SessionInner {
            zenoh: session,
            namespace: self.namespace,
        };
        Ok(Session {
            inner: Arc::new(inner),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    inner: Arc<SessionInner>,
}

#[derive(Debug)]
struct SessionInner {
    zenoh: zenoh::Session,
    namespace: String,
}

impl Session {
    pub async fn create(namespace: impl Into<String>) -> Result<Self> {
        let builder = SessionBuilder::new(namespace);
        builder.build().await
    }

    pub fn create_node(&self, name: impl Into<String>) -> NodeBuilder {
        NodeBuilder {
            session: self.clone(),
            name: name.into(),
        }
    }

    pub fn now(&self) -> Timestamp {
        self.inner.zenoh.new_timestamp()
    }

    pub(crate) fn zenoh(&self) -> &zenoh::Session {
        &self.inner.zenoh
    }

    pub fn namespace(&self) -> &str {
        &self.inner.namespace
    }
}

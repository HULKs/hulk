use std::{marker::PhantomData, sync::Arc};

use serde::{Deserialize, Serialize};
use zenoh::liveliness::LivelinessToken;

use crate::{
    parameter::ParameterBuilder, publisher::PublisherBuilder, subscriber::SubscriberBuilder,
    topic::TopicError, Session, Topic,
};

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Zenoh transport error: {0}")]
    Zenoh(#[from] zenoh::Error),
}

pub type Result<T, E = NodeError> = std::result::Result<T, E>;

pub struct NodeBuilder {
    pub(crate) session: Session,
    pub name: String,
}

impl NodeBuilder {
    pub async fn build(self) -> Result<Node> {
        let liveliness_token = self
            .session
            .zenoh()
            .liveliness()
            .declare_token(format!("{}/{}", self.session.namespace(), self.name))
            .await?;
        let inner = NodeInner {
            session: self.session,
            name: self.name,
            _liveliness_token: liveliness_token,
        };
        Ok(Node {
            inner: Arc::new(inner),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    inner: Arc<NodeInner>,
}

#[derive(Debug)]
struct NodeInner {
    session: Session,
    name: String,
    _liveliness_token: LivelinessToken,
}

impl Node {
    const DEFAULT_CAPACITY: usize = 3;

    pub(crate) fn session(&self) -> &Session {
        &self.inner.session
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn create_subscriber<T>(
        &self,
        topic: impl TryInto<Topic, Error = impl Into<TopicError>>,
    ) -> SubscriberBuilder<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        SubscriberBuilder {
            node: self.clone(),
            topic: topic.try_into().map_err(Into::into),
            capacity: Self::DEFAULT_CAPACITY,
            _phantom: PhantomData,
        }
    }

    pub fn create_publisher<T>(
        &self,
        topic: impl TryInto<Topic, Error = impl Into<TopicError>>,
    ) -> PublisherBuilder<T>
    where
        T: Serialize,
    {
        PublisherBuilder {
            node: self.clone(),
            topic: topic.try_into().map_err(Into::into),
            enable_json: true,
            _phantom: PhantomData,
        }
    }

    pub fn declare_parameter<T>(&self, name: impl Into<String>) -> ParameterBuilder<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        ParameterBuilder {
            node: self.clone(),
            name: name.into(),
            _phantom: PhantomData,
        }
    }
}

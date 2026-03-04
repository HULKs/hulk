//! Raw subscriber for receiving timestamped samples from the data or view plane.
//!
//! This is useful for tooling that wants to defer deserialization (e.g. multiplexing, caching,
//! and UI-driven typed decoding).

use tracing::{debug, trace};
use zenoh::{
    handlers::{RingChannel, RingChannelHandler},
    pubsub::Subscriber as ZenohSubscriber,
    sample::Sample as ZenohSample,
};

use crate::{
    error::Result,
    key::{DataKey, ViewKey},
    sample::Sample,
    Session, TopicExpression,
};

/// Builder for creating a [`RawSubscriber`].
pub struct RawSubscriberBuilder {
    pub(crate) session: Session,
    pub(crate) topic_expression: TopicExpression,
    pub(crate) capacity: usize,
    pub(crate) view: bool,
    pub(crate) namespace: String,
    pub(crate) node_name: String,
}

impl RawSubscriberBuilder {
    /// Sets the ring buffer capacity for incoming messages.
    ///
    /// When the buffer is full, the oldest messages are dropped to make room for new ones.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Subscribe to the View plane (JSON mirror) instead of the Data plane (CDR).
    pub fn view(mut self) -> Self {
        self.view = true;
        self
    }

    /// Override the namespace for this subscription.
    ///
    /// By default, subscriptions use the session's namespace.
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Override the node for private scoped topics (`~/path`).
    ///
    /// By default, private scoped subscriptions target the current node's name.
    pub fn on_node(mut self, name: impl Into<String>) -> Self {
        self.node_name = name.into();
        self
    }

    pub async fn build(self) -> Result<RawSubscriber> {
        let domain_id = self.session.domain_id();
        let resolved_topic = self
            .topic_expression
            .resolve(&self.namespace, Some(&self.node_name))?;
        let key = if self.view {
            ViewKey::topic(domain_id, &resolved_topic)
        } else {
            DataKey::topic(domain_id, &resolved_topic)
        };
        debug!(
            plane = if self.view { "view" } else { "data" },
            topic_expression = %self.topic_expression.as_str(),
            resolved_topic = %resolved_topic,
            namespace = %self.namespace,
            node = %self.node_name,
            capacity = self.capacity,
            key_expr = %key,
            "building raw subscriber",
        );
        RawSubscriber::from_key_expr(self.session, key, self.capacity).await
    }
}

/// Receives timestamped raw samples from Zenoh with a ring buffer.
pub struct RawSubscriber {
    session: Session,
    sub: ZenohSubscriber<RingChannelHandler<ZenohSample>>,
}

impl RawSubscriber {
    pub(crate) async fn from_key_expr(
        session: Session,
        key_expr: String,
        capacity: usize,
    ) -> Result<Self> {
        trace!(%key_expr, capacity, "declaring zenoh subscriber");
        let subscriber = session
            .zenoh()
            .declare_subscriber(key_expr)
            .with(RingChannel::new(capacity))
            .await?;

        Ok(Self {
            session,
            sub: subscriber,
        })
    }

    /// Receives the next sample.
    pub async fn recv_async(&mut self) -> Result<Sample> {
        let sample = self.sub.recv_async().await?;
        trace!(key = %sample.key_expr(), "received raw sample");
        Ok(Sample::from_zenoh(&self.session, sample))
    }
}

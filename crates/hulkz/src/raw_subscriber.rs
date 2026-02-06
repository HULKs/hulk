//! Raw subscriber for receiving timestamped samples from the data or view plane.
//!
//! This is useful for tooling that wants to defer deserialization (e.g. multiplexing, caching,
//! and UI-driven typed decoding).

use zenoh::{
    handlers::{RingChannel, RingChannelHandler},
    pubsub::Subscriber as ZenohSubscriber,
    sample::Sample as ZenohSample,
};

use crate::{
    error::Result,
    key::{DataKey, ViewKey},
    sample::Sample,
    scoped_path::ScopedPath,
    Session,
};

/// Builder for creating a [`RawSubscriber`].
pub struct RawSubscriberBuilder {
    pub(crate) session: Session,
    pub(crate) topic: ScopedPath,
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
        let topic = self.topic;
        let key = if self.view {
            ViewKey::from_scope(
                topic.scope(),
                &self.namespace,
                &self.node_name,
                topic.path(),
            )
        } else {
            DataKey::from_scope(
                topic.scope(),
                &self.namespace,
                &self.node_name,
                topic.path(),
            )
        };
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
        Ok(Sample::from_zenoh(&self.session, sample))
    }
}

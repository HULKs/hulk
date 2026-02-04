//! Subscriber for receiving messages from the data or view plane.
//!
//! A [`Subscriber`] receives messages from a topic. By default it subscribes to the data plane
//! (CDR). Use `.view()` to subscribe to the view plane (JSON) for debugging/CLI tools.
//!
//! Messages are delivered as [`Message<T>`](crate::Message) with payload and timestamp.
//!
//! # Example
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Debug, Clone, Serialize, Deserialize)] struct Odometry { x: f64 }
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let session = Session::create("robot").await?;
//! # let node = session.create_node("n").build().await?;
//! let mut subscriber = node.subscribe::<Odometry>("odom").build().await?;
//!
//! loop {
//!     let msg = subscriber.recv_async().await?;
//!     println!("t={:?} payload={:?}", msg.timestamp, msg.payload);
//! }
//! # }
//! ```

use serde::Deserialize;
use std::marker::PhantomData;
use tracing::warn;
use zenoh::{
    bytes::Encoding,
    handlers::{RingChannel, RingChannelHandler},
    pubsub::Subscriber as ZenohSubscriber,
    sample::Sample,
};

use crate::{
    error::{Error, Result, ScopedPathError},
    scoped_path::ScopedPath,
    Message, Node, Session,
};

/// Builder for creating a [`Subscriber`].
pub struct SubscriberBuilder<T> {
    pub(crate) node: Node,
    pub(crate) topic: Result<ScopedPath, ScopedPathError>,
    pub(crate) capacity: usize,
    pub(crate) view: bool,
    pub(crate) namespace_override: Option<String>,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> SubscriberBuilder<T> {
    /// Sets the ring buffer capacity for incoming messages.
    ///
    /// When the buffer is full, the oldest messages are dropped to make room for new ones.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Subscribe to the View plane (JSON) instead of the Data plane (CDR).
    ///
    /// This is useful for CLI tools or debugging scenarios where you want to receive
    /// human-readable JSON messages.
    pub fn view(mut self) -> Self {
        self.view = true;
        self
    }

    /// Override the namespace for this subscription.
    ///
    /// By default, subscriptions use the session's namespace. This method allows subscribing to
    /// topics in a different namespace.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use hulkz::{Session, Result};
    /// # use serde_json::Value;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let session = Session::create("twix").await?;
    /// let node = session.create_node("viewer").build().await?;
    ///
    /// // Subscribe to a topic in a different namespace
    /// let mut subscriber = node.subscribe::<Value>("camera/front")
    ///     .in_namespace("robot-nao22")
    ///     .view_only()
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace_override = Some(namespace.into());
        self
    }

    pub async fn build(self) -> Result<Subscriber<T>> {
        let topic = self.topic?;

        let session = self.node.session().clone();

        let namespace = self
            .namespace_override
            .as_deref()
            .unwrap_or_else(|| self.node.session().namespace());

        let key = if self.view {
            topic.to_view_key(namespace, self.node.name())
        } else {
            topic.to_data_key(namespace, self.node.name())
        };

        let subscriber = session
            .zenoh()
            .declare_subscriber(key)
            .with(RingChannel::new(self.capacity))
            .await?;

        Ok(Subscriber {
            session,
            sub: subscriber,
            _phantom: PhantomData,
        })
    }
}

/// Receives messages from the data plane with a ring buffer.
pub struct Subscriber<T> {
    session: Session,
    sub: ZenohSubscriber<RingChannelHandler<Sample>>,
    _phantom: PhantomData<T>,
}

impl<T> Subscriber<T>
where
    for<'de> T: Deserialize<'de>,
{
    /// Receives the next message.
    pub async fn recv_async(&mut self) -> Result<Message<T>> {
        let sample = self.sub.recv_async().await?;

        let payload = match sample.encoding() {
            &Encoding::APPLICATION_CDR => {
                cdr::deserialize(&sample.payload().to_bytes()).map_err(Error::CdrDeserialize)?
            }
            &Encoding::APPLICATION_JSON => serde_json::from_slice(&sample.payload().to_bytes())
                .map_err(Error::JsonDeserialize)?,
            encoding => {
                return Err(Error::UnsupportedEncoding(encoding.clone()));
            }
        };
        let timestamp = sample.timestamp().copied().unwrap_or_else(|| {
            warn!("Sample has no timestamp, using current time instead");
            self.session.now()
        });
        let message = Message { timestamp, payload };
        Ok(message)
    }
}

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
use tracing::trace;

use crate::{
    error::Result,
    raw_subscriber::{RawSubscriber, RawSubscriberBuilder},
    Message,
};

/// Builder for creating a [`Subscriber`].
pub struct SubscriberBuilder<T> {
    pub(crate) raw: RawSubscriberBuilder,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> SubscriberBuilder<T> {
    /// Sets the ring buffer capacity for incoming messages.
    ///
    /// When the buffer is full, the oldest messages are dropped to make room for new ones.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.raw = self.raw.capacity(capacity);
        self
    }

    /// Subscribe to the View plane (JSON) instead of the Data plane (CDR).
    ///
    /// This is useful for CLI tools or debugging scenarios where you want to receive
    /// human-readable JSON messages.
    pub fn view(mut self) -> Self {
        self.raw = self.raw.view();
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
    ///     .view()
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.raw = self.raw.in_namespace(namespace);
        self
    }

    /// Override the node for private scoped topics (`~/path`).
    ///
    /// By default, private scoped subscriptions target the current node's name.
    pub fn on_node(mut self, name: impl Into<String>) -> Self {
        self.raw = self.raw.on_node(name);
        self
    }

    pub async fn build(self) -> Result<Subscriber<T>> {
        let raw = self.raw.build().await?;
        Ok(Subscriber {
            raw,
            _phantom: PhantomData,
        })
    }
}

/// Receives messages from the data plane with a ring buffer.
pub struct Subscriber<T> {
    raw: RawSubscriber,
    _phantom: PhantomData<T>,
}

impl<T> Subscriber<T>
where
    for<'de> T: Deserialize<'de>,
{
    /// Receives the next message.
    pub async fn recv_async(&mut self) -> Result<Message<T>> {
        let sample = self.raw.recv_async().await?;
        trace!(
            timestamp_nanos = sample.timestamp.get_time().as_nanos(),
            encoding = %sample.encoding,
            "received typed subscriber sample",
        );
        let payload = sample.decode::<T>()?;
        let message = Message {
            timestamp: sample.timestamp,
            payload,
        };
        Ok(message)
    }
}

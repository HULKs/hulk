//! Time-indexed message buffer for temporal lookups.
//!
//! The [`Buffer`] provides temporal data alignment by caching messages from a subscriber and
//! allowing lookups by timestamp. This is essential for sensor fusion scenarios where data from
//! multiple sources at different rates needs to be aligned temporally.
//!
//! # Quick Start
//!
//! Use [`Node::buffer()`](crate::Node::buffer) for the common case:
//!
//! ```rust,no_run
//! # async fn example() -> hulkz::Result<()> {
//! # let session = hulkz::Session::create("robot").await?;
//! # let node = session.create_node("fusion").build().await?;
//! // Create a buffered subscription (capacity = 200 messages)
//! let (imu, driver) = node.buffer::<i32>("imu/data", 200).await?;
//! tokio::spawn(driver);
//!
//! // Look up data at a specific timestamp
//! let timestamp = session.now();
//! if let Some(msg) = imu.lookup_nearest(&timestamp).await {
//!     println!("IMU: {:?}", msg.payload);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Advanced Usage
//!
//! Use [`BufferBuilder`] directly when building from an existing subscriber:
//!
//! ```rust,no_run
//! # async fn example() -> hulkz::Result<()> {
//! # let session = hulkz::Session::create("robot").await?;
//! # let node = session.create_node("fusion").build().await?;
//! let subscriber = node.subscribe::<i32>("sensor/data").build().await?;
//! let (buffer, driver) = hulkz::BufferBuilder::new(subscriber)
//!     .capacity(100)
//!     .build();
//! tokio::spawn(driver);
//! # Ok(())
//! # }
//! ```

use std::{future::Future, sync::Arc};

use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{error::Result, Cache, Message, Subscriber, Timestamp};

/// Builder for creating a [`Buffer`] from a subscriber.
///
/// # Example
///
/// ```rust,no_run
/// # use hulkz::{BufferBuilder, Node};
/// # async fn example(node: &Node) -> hulkz::Result<()> {
/// let subscriber = node.subscribe::<i32>("topic").build().await?;
/// let (buffer, driver) = BufferBuilder::new(subscriber)
///     .capacity(50)
///     .build();
/// tokio::spawn(driver);
/// # Ok(())
/// # }
/// ```
pub struct BufferBuilder<T> {
    subscriber: Subscriber<T>,
    capacity: usize,
}

impl<T> BufferBuilder<T>
where
    for<'de> T: Deserialize<'de> + Clone + Send + Sync + 'static,
{
    /// Creates a new buffer builder with the given subscriber.
    ///
    /// Default capacity is 1 (only keeps the latest message).
    pub fn new(subscriber: Subscriber<T>) -> Self {
        Self {
            subscriber,
            capacity: 1,
        }
    }

    /// Sets the maximum number of messages to retain in the buffer.
    ///
    /// When the buffer exceeds this capacity, the oldest messages are removed
    /// from the buffer.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Builds the buffer, returning a handle and a driver future.
    ///
    /// The driver future must be spawned to receive messages from the subscriber
    /// and populate the buffer. The buffer handle can be cloned and used from
    /// multiple tasks.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use hulkz::{BufferBuilder, Subscriber};
    /// # async fn example(subscriber: Subscriber<i32>) {
    /// let (buffer, driver) = BufferBuilder::new(subscriber).build();
    ///
    /// // Spawn the driver on your async runtime
    /// tokio::spawn(driver);
    ///
    /// // Use the buffer handle
    /// let latest = buffer.get_latest().await;
    /// # }
    /// ```
    pub fn build(self) -> (Buffer<T>, impl Future<Output = Result<()>> + Send) {
        let cache = Arc::new(RwLock::new(Cache::new(self.capacity)));
        let mut subscriber = self.subscriber;

        let driver = {
            let cache = cache.clone();
            async move {
                loop {
                    let message = subscriber.recv_async().await?;
                    let mut cache = cache.write().await;
                    cache.add(message.timestamp, Arc::new(message));
                }
            }
        };

        let handle = Buffer { cache };

        (handle, driver)
    }
}

/// A buffered subscriber that caches messages for temporal lookup.
///
/// The buffer stores messages indexed by their timestamp, allowing efficient
/// lookups for temporal data alignment (e.g., sensor fusion).
///
/// Use [`BufferBuilder`] to create a buffer from a subscriber.
#[derive(Clone)]
pub struct Buffer<T> {
    cache: Arc<RwLock<Cache<Arc<Message<T>>>>>,
}

impl<T> Buffer<T>
where
    T: Clone,
{
    /// Look up the message with timestamp closest to the given timestamp.
    ///
    /// If the buffer is empty, returns `None`. If equidistant from two messages,
    /// returns the earlier one.
    pub async fn lookup_nearest(&self, timestamp: &Timestamp) -> Option<Arc<Message<T>>> {
        let cache = self.cache.read().await;

        let before = cache.get_elem_before_time(timestamp);
        let after = cache.get_elem_after_time(timestamp);

        match (before, after) {
            (Some(before), Some(after)) => {
                let diff_to_before = timestamp
                    .get_time()
                    .to_duration()
                    .abs_diff(before.timestamp.get_time().to_duration());
                let diff_to_after = timestamp
                    .get_time()
                    .to_duration()
                    .abs_diff(after.timestamp.get_time().to_duration());
                if diff_to_before <= diff_to_after {
                    Some(before.clone())
                } else {
                    Some(after.clone())
                }
            }
            (Some(x), None) | (None, Some(x)) => Some(x.clone()),
            (None, None) => None,
        }
    }

    /// Look up the latest message with timestamp <= the given timestamp.
    ///
    /// Returns `None` if no such message exists.
    pub async fn lookup_before(&self, timestamp: &Timestamp) -> Option<Arc<Message<T>>> {
        self.cache
            .read()
            .await
            .get_elem_before_time(timestamp)
            .cloned()
    }

    /// Look up the earliest message with timestamp >= the given timestamp.
    ///
    /// Returns `None` if no such message exists.
    pub async fn lookup_after(&self, timestamp: &Timestamp) -> Option<Arc<Message<T>>> {
        self.cache
            .read()
            .await
            .get_elem_after_time(timestamp)
            .cloned()
    }

    /// Get all messages in the given time range (inclusive).
    ///
    /// Returns messages where `start <= timestamp <= end`.
    pub async fn lookup_interval(
        &self,
        start: &Timestamp,
        end: &Timestamp,
    ) -> Vec<Arc<Message<T>>> {
        self.cache
            .read()
            .await
            .get_interval(start, end)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Get the latest (most recent) message in the buffer.
    ///
    /// Returns `None` if the buffer is empty.
    pub async fn get_latest(&self) -> Option<Arc<Message<T>>> {
        self.cache.read().await.get_latest().cloned()
    }

    /// Get the timestamp of the latest message.
    ///
    /// Returns `None` if the buffer is empty.
    pub async fn get_latest_time(&self) -> Option<Timestamp> {
        self.cache.read().await.get_latest_time().copied()
    }

    /// Get the timestamp of the oldest message in the buffer.
    ///
    /// Returns `None` if the buffer is empty.
    pub async fn get_oldest_time(&self) -> Option<Timestamp> {
        self.cache.read().await.get_oldest_time().copied()
    }

    /// Returns the number of messages currently in the buffer.
    pub async fn len(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Returns `true` if the buffer is empty.
    pub async fn is_empty(&self) -> bool {
        self.cache.read().await.is_empty()
    }
}

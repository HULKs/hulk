//! Timestamp-indexed, capacity-bounded message cache.
//!
//! [`Cache`](crate::cache::Cache) retains a sliding window of received messages
//! and lets callers query them by time.
//!
//! # Stamp strategies
//!
//! Two indexing strategies are available, selected at build time:
//!
//! - **[`ZenohStamp`](crate::cache::ZenohStamp)** (default) — indexes by the
//!   Zenoh transport timestamp (`uhlc::Timestamp` → [`crate::time::Time`]). Zero-config;
//!   works for any message type as long as timestamping is enabled in the Zenoh
//!   config (already enabled in the ros-z default config).
//! - **[`ExtractorStamp`](crate::cache::ExtractorStamp)** — indexes by a
//!   user-supplied closure that extracts a [`crate::time::Time`] from each deserialized
//!   message. Required for `header.stamp` / sensor capture time alignment.
//!
//! # Example
//!
//! ```rust,ignore
//! use ros_z::prelude::*;
//! use ros_z::time::Time;
//! use ros_z_msgs::sensor_msgs::Imu;
//! use std::time::Duration;
//!
//! # async fn example() -> zenoh::Result<()> {
//! let context = ContextBuilder::default().build().await?;
//! let node = context.create_node("cache_demo").build().await?;
//!
//! // Zero-config: indexed by Zenoh transport timestamp
//! let cache = node.create_cache::<Imu>("/imu/data", 200).build().await?;
//!
//! let now = Time::from_wallclock(std::time::SystemTime::now());
//! let window = cache.get_interval(now - Duration::from_millis(100), now);
//!
//! // Application timestamp: indexed by header.stamp
//! let cache = node
//!     .create_cache::<Imu>("/imu/data", 200)
//!     .with_stamp(|message: &Imu| {
//!         let nanos = (message.header.stamp.sec as i64 * 1_000_000_000)
//!             + i64::from(message.header.stamp.nanosec);
//!         Time::from_nanos(nanos)
//!     })
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use parking_lot::RwLock;
use std::collections::{BTreeMap, VecDeque};
use std::marker::PhantomData;
use std::sync::Arc;
use tracing::{debug, warn};
use zenoh::Result;

use crate::msg::{SerdeCdrCodec, WireDecoder, WireMessage};
use crate::pubsub::SubscriberBuilder;
use crate::time::Time;

// ---------------------------------------------------------------------------
// Stamp strategy markers
// ---------------------------------------------------------------------------

/// Index by the Zenoh transport timestamp (`uhlc::Timestamp` → [`crate::time::Time`]).
///
/// This is the default stamp strategy. It works for any message type without
/// any configuration. If the incoming [`zenoh::sample::Sample`] carries no
/// timestamp (timestamping disabled on the peer), the cache falls back to
/// the current wallclock time at receive time and logs a one-time warning.
pub struct ZenohStamp;

/// Index by an application-supplied extractor closure.
///
/// The closure receives a reference to the deserialized message and returns a
/// `Time` representing its logical timestamp (e.g. `header.stamp`).
pub struct ExtractorStamp<T, F, O>(pub(crate) F, pub(crate) PhantomData<(T, O)>)
where
    F: Fn(&T) -> O,
    O: Into<Time>;

// ---------------------------------------------------------------------------
// CacheInner — shared mutable state
// ---------------------------------------------------------------------------

/// Internal cache storage — public for benchmarks only.
#[doc(hidden)]
pub struct CacheInner<T> {
    entries: BTreeMap<Time, VecDeque<Arc<T>>>,
    capacity: usize,
    len: usize,
    /// Guards against logging the missing-timestamp warning more than once.
    warned_no_ts: bool,
}

impl<T> CacheInner<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            capacity,
            len: 0,
            warned_no_ts: false,
        }
    }

    pub fn insert(&mut self, stamp: Time, message: T) {
        if self.capacity == 0 {
            return;
        }

        self.entries
            .entry(stamp)
            .or_default()
            .push_back(Arc::new(message));
        self.len += 1;

        while self.len > self.capacity {
            let Some(mut oldest) = self.entries.first_entry() else {
                break;
            };

            let bucket = oldest.get_mut();
            if bucket.pop_front().is_some() {
                self.len -= 1;
            }
            if bucket.is_empty() {
                oldest.remove_entry();
            }
        }
    }

    pub fn get_interval(&self, t_start: Time, t_end: Time) -> Vec<Arc<T>> {
        if t_start > t_end {
            return Vec::new();
        }

        self.entries
            .range(t_start..=t_end)
            .flat_map(|(_, bucket)| bucket.iter().map(Arc::clone))
            .collect()
    }

    pub fn get_before(&self, t: Time) -> Option<Arc<T>> {
        self.entries
            .range(..=t)
            .next_back()
            .and_then(|(_, bucket)| bucket.back().map(Arc::clone))
    }

    pub fn get_after(&self, t: Time) -> Option<Arc<T>> {
        self.entries
            .range(t..)
            .next()
            .and_then(|(_, bucket)| bucket.front().map(Arc::clone))
    }

    pub fn get_nearest(&self, t: Time) -> Option<Arc<T>> {
        if self.entries.is_empty() {
            return None;
        }

        let before = self
            .entries
            .range(..=t)
            .next_back()
            .and_then(|(k, bucket)| bucket.back().map(|v| (*k, Arc::clone(v))));
        let after = self
            .entries
            .range(t..)
            .next()
            .and_then(|(k, bucket)| bucket.front().map(|v| (*k, Arc::clone(v))));

        match (before, after) {
            (None, Some((_, v))) => Some(v),
            (Some((_, v)), None) => Some(v),
            (Some((kb, vb)), Some((ka, va))) => {
                let dist_before = t.duration_since(kb);
                let dist_after = ka.duration_since(t);
                // On a tie prefer earlier (before) timestamp.
                if dist_after < dist_before {
                    Some(va)
                } else {
                    Some(vb)
                }
            }
            (None, None) => None,
        }
    }

    pub fn oldest_stamp(&self) -> Option<Time> {
        self.entries.keys().next().copied()
    }

    pub fn newest_stamp(&self) -> Option<Time> {
        self.entries.keys().next_back().copied()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.len = 0;
    }
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// A timestamp-indexed, capacity-bounded sliding-window cache of received
/// messages.
///
/// Built via [`CacheBuilder`], created through
/// [`Node::create_cache`](crate::node::Node::create_cache).
///
/// Messages are stored as [`Arc<T>`] so query methods return shared references
/// without deep-copying the message payload.
///
/// Dropping `Cache` automatically deregisters the underlying Zenoh subscriber.
pub struct Cache<T: WireMessage> {
    inner: Arc<RwLock<CacheInner<T>>>,
    _raw_subscriber_task: tokio::task::JoinHandle<()>,
}

impl<T: WireMessage> Drop for Cache<T> {
    fn drop(&mut self) {
        self._raw_subscriber_task.abort();
    }
}

impl<T: WireMessage> Cache<T> {
    /// All messages with timestamp in `[t_start, t_end]`, inclusive, ordered
    /// by timestamp ascending. Messages sharing the same timestamp are returned
    /// in insertion order.
    ///
    /// Returns `Arc<T>` handles — no deep copy of message payload. If
    /// `t_start > t_end` the result is always empty (no panic).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let window = cache.get_interval(
    ///     Time::from_wallclock(std::time::SystemTime::now()) - Duration::from_millis(100),
    ///     Time::from_wallclock(std::time::SystemTime::now()),
    /// );
    /// ```
    pub fn get_interval<TStart, TEnd>(&self, t_start: TStart, t_end: TEnd) -> Vec<Arc<T>>
    where
        TStart: Into<Time>,
        TEnd: Into<Time>,
    {
        let t_start = t_start.into();
        let t_end = t_end.into();
        if t_start > t_end {
            return Vec::new();
        }
        let inner = self.inner.read();
        inner.get_interval(t_start, t_end)
    }

    /// The most recent message with timestamp ≤ `t`, or `None` if the cache is
    /// empty or all messages are strictly after `t`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let latest = cache.get_before(Time::from_wallclock(std::time::SystemTime::now()));
    /// ```
    pub fn get_before<TStamp>(&self, t: TStamp) -> Option<Arc<T>>
    where
        TStamp: Into<Time>,
    {
        let t = t.into();
        let inner = self.inner.read();
        inner.get_before(t)
    }

    /// The earliest message with timestamp ≥ `t`, or `None` if the cache is
    /// empty or all messages are strictly before `t`.
    ///
    /// If multiple messages share the selected timestamp, the earliest inserted
    /// message at that timestamp is returned.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let next = cache.get_after(camera_timestamp);
    /// ```
    pub fn get_after<TStamp>(&self, t: TStamp) -> Option<Arc<T>>
    where
        TStamp: Into<Time>,
    {
        let t = t.into();
        let inner = self.inner.read();
        inner.get_after(t)
    }

    /// The message whose timestamp is nearest to `t` (either side).
    ///
    /// When two messages are equidistant, the one with the earlier (before)
    /// timestamp is returned.
    ///
    /// For duplicate timestamps, the selected bucket is stable: the latest
    /// inserted message is used for the before/exact side and the earliest
    /// inserted message is used for the after side.
    ///
    /// Returns `None` if the cache is empty.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let nearest_imu = cache.get_nearest(camera_stamp);
    /// ```
    pub fn get_nearest<TStamp>(&self, t: TStamp) -> Option<Arc<T>>
    where
        TStamp: Into<Time>,
    {
        let t = t.into();
        let inner = self.inner.read();
        inner.get_nearest(t)
    }

    /// Timestamp of the oldest cached message, or `None` if empty.
    pub fn oldest_stamp(&self) -> Option<Time> {
        self.inner.read().oldest_stamp()
    }

    /// Timestamp of the newest cached message, or `None` if empty.
    pub fn newest_stamp(&self) -> Option<Time> {
        self.inner.read().newest_stamp()
    }

    /// Number of messages currently in the cache.
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// `true` if the cache holds no messages.
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Remove all messages from the cache.
    pub fn clear(&self) {
        self.inner.write().clear();
    }
}

// ---------------------------------------------------------------------------
// CacheBuilder
// ---------------------------------------------------------------------------

/// Builder for [`Cache<T>`].
///
/// Created by [`Node::create_cache`](crate::node::Node::create_cache).
/// Use [`with_stamp`](CacheBuilder::with_stamp) to switch from the default
/// Zenoh transport timestamp to an application-level extractor.
pub struct CacheBuilder<T, S = SerdeCdrCodec<T>, Stamp = ZenohStamp> {
    pub(crate) sub_builder: SubscriberBuilder<T, S>,
    capacity: usize,
    stamp: Stamp,
}

impl<T: WireMessage, S> CacheBuilder<T, S, ZenohStamp> {
    pub(crate) fn new(sub_builder: SubscriberBuilder<T, S>, capacity: usize) -> Self {
        Self {
            sub_builder,
            capacity,
            stamp: ZenohStamp,
        }
    }

    /// Switch to application-level timestamp extraction.
    ///
    /// The extractor receives a reference to the deserialized message and
    /// returns a `Time` representing its logical timestamp (e.g.
    /// `header.stamp`).
    pub fn with_stamp<F, O>(self, extractor: F) -> CacheBuilder<T, S, ExtractorStamp<T, F, O>>
    where
        F: Fn(&T) -> O + Send + Sync + 'static,
        O: Into<Time> + 'static,
    {
        CacheBuilder {
            sub_builder: self.sub_builder,
            capacity: self.capacity,
            stamp: ExtractorStamp(extractor, PhantomData),
        }
    }

    /// Maximum number of messages to retain. Oldest are evicted when full.
    /// A capacity of `0` disables retention and stores no messages.
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Apply a QoS profile to the underlying subscriber.
    pub fn with_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.sub_builder = self.sub_builder.qos(qos);
        self
    }
}

impl<T, S> CacheBuilder<T, S, ZenohStamp>
where
    T: WireMessage + Send + Sync + 'static,
    S: for<'a> WireDecoder<Input<'a> = &'a [u8], Output = T> + 'static,
{
    pub async fn build(self) -> Result<Cache<T>> {
        self.build_with_stamp_async().await
    }

    async fn build_with_stamp_async(self) -> Result<Cache<T>> {
        let CacheBuilder {
            sub_builder,
            capacity,
            ..
        } = self;
        let inner = Arc::new(RwLock::new(CacheInner::<T>::new(capacity)));
        let inner_cb = inner.clone();

        let raw_subscriber = sub_builder.raw().build().await?;
        let mut raw_subscriber_task = raw_subscriber;
        let task = tokio::spawn(async move {
            loop {
                let sample = match raw_subscriber_task.recv().await {
                    Ok(sample) => sample,
                    Err(error) => {
                        tracing::error!("[CACHE] Failed to receive raw sample: {}", error);
                        break;
                    }
                };
                let payload = sample.payload().to_bytes();
                match S::deserialize(&payload) {
                    Ok(message) => {
                        let stamp = match sample.timestamp() {
                            Some(ts) => Time::from_wallclock(ts.get_time().to_system_time()),
                            None => {
                                let mut guard = inner_cb.write();
                                if !guard.warned_no_ts {
                                    warn!(
                                        "[CACHE] Incoming sample has no Zenoh timestamp; \
                                         falling back to current wallclock time. \
                                         Enable timestamping in the Zenoh config to avoid this."
                                    );
                                    guard.warned_no_ts = true;
                                }
                                drop(guard);
                                Time::from_wallclock(std::time::SystemTime::now())
                            }
                        };
                        inner_cb.write().insert(stamp, message);
                    }
                    Err(e) => tracing::error!("[CACHE] Failed to deserialize message: {}", e),
                }
            }
        });

        debug!("[CACHE] ZenohStamp cache ready");
        Ok(Cache {
            inner,
            _raw_subscriber_task: task,
        })
    }
}

impl<T: WireMessage, S, F, O> CacheBuilder<T, S, ExtractorStamp<T, F, O>>
where
    F: Fn(&T) -> O + Send + Sync + 'static,
    O: Into<Time> + 'static,
{
    /// Maximum number of messages to retain. Oldest are evicted when full.
    /// A capacity of `0` disables retention and stores no messages.
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Apply a QoS profile to the underlying subscriber.
    pub fn with_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.sub_builder = self.sub_builder.qos(qos);
        self
    }
}

impl<T, S, F, O> CacheBuilder<T, S, ExtractorStamp<T, F, O>>
where
    T: WireMessage + Send + Sync + 'static,
    S: for<'a> WireDecoder<Input<'a> = &'a [u8], Output = T> + 'static,
    F: Fn(&T) -> O + Send + Sync + 'static,
    O: Into<Time> + 'static,
{
    pub async fn build(self) -> Result<Cache<T>> {
        self.build_with_stamp_async().await
    }

    async fn build_with_stamp_async(self) -> Result<Cache<T>> {
        let CacheBuilder {
            sub_builder,
            capacity,
            stamp: ExtractorStamp(extractor, _),
        } = self;
        let inner = Arc::new(RwLock::new(CacheInner::<T>::new(capacity)));
        let inner_cb = inner.clone();

        let raw_subscriber = sub_builder.raw().build().await?;
        let mut raw_subscriber_task = raw_subscriber;
        let task = tokio::spawn(async move {
            loop {
                let sample = match raw_subscriber_task.recv().await {
                    Ok(sample) => sample,
                    Err(error) => {
                        tracing::error!("[CACHE] Failed to receive raw sample: {}", error);
                        break;
                    }
                };
                let payload = sample.payload().to_bytes();
                match S::deserialize(&payload) {
                    Ok(message) => {
                        let stamp = extractor(&message).into();
                        inner_cb.write().insert(stamp, message);
                    }
                    Err(e) => tracing::error!("[CACHE] Failed to deserialize message: {}", e),
                }
            }
        });

        debug!("[CACHE] ExtractorStamp cache ready");
        Ok(Cache {
            inner,
            _raw_subscriber_task: task,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_inner_capacity_zero_retains_no_messages() {
        let mut inner = CacheInner::new(0);
        inner.insert(Time::from_nanos(1), "first");
        inner.insert(Time::from_nanos(2), "second");

        assert_eq!(inner.len(), 0);
        assert!(inner.is_empty());
    }

    #[test]
    fn cache_inner_preserves_duplicate_timestamps() {
        let mut inner = CacheInner::new(10);
        let stamp = Time::from_nanos(1_000);
        inner.insert(stamp, "first");
        inner.insert(stamp, "second");

        assert_eq!(inner.len(), 2);
        let values = inner.get_interval(stamp, stamp);
        assert_eq!(values.len(), 2);
        assert_eq!(*values[0], "first");
        assert_eq!(*values[1], "second");
    }

    #[test]
    fn cache_inner_evicts_oldest_duplicate_samples_first() {
        let mut inner = CacheInner::new(2);
        let first_stamp = Time::from_nanos(1_000);
        let second_stamp = Time::from_nanos(2_000);
        inner.insert(first_stamp, "first");
        inner.insert(first_stamp, "second");
        inner.insert(second_stamp, "third");

        assert_eq!(inner.len(), 2);
        let values = inner.get_interval(first_stamp, second_stamp);
        assert_eq!(values.len(), 2);
        assert_eq!(*values[0], "second");
        assert_eq!(*values[1], "third");
    }

    #[test]
    fn cache_inner_duplicate_query_selection_is_stable() {
        let mut inner = CacheInner::new(10);
        let stamp = Time::from_nanos(1_000);
        let later_stamp = Time::from_nanos(2_000);
        inner.insert(stamp, "first");
        inner.insert(stamp, "second");
        inner.insert(later_stamp, "third");

        assert_eq!(*inner.get_before(stamp).unwrap(), "second");
        assert_eq!(*inner.get_after(stamp).unwrap(), "first");
        assert_eq!(*inner.get_nearest(stamp).unwrap(), "second");
        assert_eq!(*inner.get_after(Time::from_nanos(1_500)).unwrap(), "third");
    }
}

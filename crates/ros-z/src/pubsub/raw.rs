use std::sync::Arc;
use std::time::Duration;

use zenoh::{Result, sample::Sample};

use crate::pubsub::subscriber::{SubscriberBuilder, SubscriberResources};
use crate::qos::QosProfile;
use crate::queue::BoundedQueue;

/// Subscriber that receives raw Zenoh samples.
///
/// Raw subscribers preserve the normal subscriber setup, including QoS,
/// liveliness, locality, and transient-local replay.
/// Received samples are delivered as [`Sample`] values without deserialization.
pub struct RawSubscriber {
    queue: Arc<BoundedQueue<Sample>>,
    _resources: SubscriberResources,
}

impl RawSubscriber {
    pub(super) fn new(queue: Arc<BoundedQueue<Sample>>, resources: SubscriberResources) -> Self {
        Self {
            queue,
            _resources: resources,
        }
    }

    /// Wait for the next raw [`Sample`].
    ///
    /// This returns the sample payload and metadata exactly as delivered by
    /// Zenoh and does not deserialize it into a message type. The receive is
    /// cancel-safe: cancelling this future before it completes does not remove a
    /// sample from the queue.
    pub async fn recv(&mut self) -> Result<Sample> {
        Ok(self.queue.recv_async().await)
    }
}

/// Builder for raw sample subscribers.
///
/// This is produced by [`crate::pubsub::SubscriberBuilder::raw`]. The `T`
/// and `C` parameters are retained only to preserve the source builder's
/// message type, type-info, and manual-codec path. Built subscribers deliver
/// [`Sample`] values directly and do not deserialize with `C`.
pub struct RawSubscriberBuilder<T, C = <T as crate::Message>::Codec> {
    pub(crate) inner: SubscriberBuilder<T, C>,
}

impl<T, C> RawSubscriberBuilder<T, C>
where
    T: Send + Sync + 'static,
{
    pub fn qos(self, qos: QosProfile) -> Self {
        Self {
            inner: self.inner.qos(qos),
        }
    }

    pub fn locality(self, locality: zenoh::sample::Locality) -> Self {
        Self {
            inner: self.inner.locality(locality),
        }
    }

    pub fn transient_local_replay_timeout(self, timeout: Duration) -> Self {
        Self {
            inner: self.inner.transient_local_replay_timeout(timeout),
        }
    }

    pub async fn build(self) -> Result<RawSubscriber> {
        self.inner.build_raw_queue_async().await
    }
}

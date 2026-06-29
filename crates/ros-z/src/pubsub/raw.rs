use std::sync::Arc;
use std::time::Duration;

use zenoh::sample::Sample;

use crate::Result;
use crate::entity::EndpointEntity;
use crate::graph::Graph;
use crate::pubsub::subscriber::{
    SubscriberBuilder, SubscriberResources, recv_sample_with_publisher_warning,
};
use crate::qos::QosProfile;
use crate::queue::BoundedQueue;

/// Subscriber that receives raw Zenoh samples.
///
/// Raw subscribers preserve the normal subscriber setup, including QoS,
/// liveliness, locality, and transient-local replay.
/// Received samples are delivered as [`Sample`] values without deserialization.
pub struct RawSubscriber {
    queue: Arc<BoundedQueue<Sample>>,
    graph: Arc<Graph>,
    entity: EndpointEntity,
    publisher_warning_timeout: Option<Duration>,
    _resources: SubscriberResources,
}

impl RawSubscriber {
    pub(super) fn new(
        queue: Arc<BoundedQueue<Sample>>,
        resources: SubscriberResources,
        graph: Arc<Graph>,
        entity: EndpointEntity,
        publisher_warning_timeout: Option<Duration>,
    ) -> Self {
        Self {
            queue,
            graph,
            entity,
            publisher_warning_timeout,
            _resources: resources,
        }
    }

    /// Wait for the next raw [`Sample`].
    ///
    /// This returns the sample payload and metadata exactly as delivered by
    /// Zenoh and does not deserialize it into a message type. The receive is
    /// cancel-safe: cancelling this future before it completes does not remove a
    /// sample from the queue.
    ///
    /// By default, this logs a warning after
    /// [`DEFAULT_PUBLISHER_WARNING_TIMEOUT`](crate::pubsub::DEFAULT_PUBLISHER_WARNING_TIMEOUT) if
    /// no sample arrives and no publishers are visible for the topic. The warning does not end the
    /// receive; this method continues waiting for the next sample.
    pub async fn recv(&mut self) -> Result<Sample> {
        let sample = recv_sample_with_publisher_warning(
            &self.queue,
            &self.graph,
            &self.entity,
            self.publisher_warning_timeout,
        )
        .await;
        Ok(sample)
    }
}

/// Builder for raw sample subscribers.
///
/// This is produced by [`crate::pubsub::SubscriberBuilder::raw`]. The `T`
/// and `C` parameters are retained only to preserve the source builder's
/// message type and associated codec type. Built subscribers deliver [`Sample`]
/// values directly and do not deserialize with `C`.
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

    /// Configure how long receive waits before warning that no publishers are visible.
    ///
    /// The warning is emitted only when no sample arrives before `timeout` and the graph has no
    /// visible publishers for the subscriber topic. Receiving continues waiting after the warning.
    pub fn publisher_warning_timeout(self, timeout: Duration) -> Self {
        Self {
            inner: self.inner.publisher_warning_timeout(timeout),
        }
    }

    /// Disable warnings when receive waits without any visible publishers.
    pub fn without_publisher_warning(self) -> Self {
        Self {
            inner: self.inner.without_publisher_warning(),
        }
    }

    pub async fn build(self) -> Result<RawSubscriber> {
        self.inner.build_raw_queue_async().await
    }
}

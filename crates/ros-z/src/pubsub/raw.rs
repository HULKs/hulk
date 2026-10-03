use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use zenoh::sample::Sample;

use crate::Result;
use crate::message::WireDecoder;
use crate::pubsub::subscriber::{QueueOverflowReporting, SubscriberBuilder, SubscriberResources};
use crate::qos::QosProfile;
use crate::queue::BoundedQueue;

/// Publisher for serialized CDR. The caller must supply bytes matching its schema.
pub struct RawPublisher {
    inner: super::Publisher<Vec<u8>>,
}

impl RawPublisher {
    pub fn entity(&self) -> &crate::entity::EndpointEntity {
        self.inner.entity()
    }

    /// Preserve payload bytes and source time, assigning a fresh publication ID.
    pub async fn publish_with_source_time(
        &self,
        payload: zenoh::bytes::ZBytes,
        source_time: crate::time::Time,
    ) -> Result<()> {
        let id = self.inner.next_publication_id();
        let attachment = crate::attachment::Attachment::with_source_time(
            id.sequence_number(),
            id.endpoint_global_id(),
            source_time,
        );
        self.inner.publish_payload(payload, attachment).await
    }
}

pub struct RawPublisherBuilder {
    inner: super::PublisherBuilder<Vec<u8>>,
    type_info: crate::TypeInfo,
    schema: crate::dynamic::Schema,
}

impl RawPublisherBuilder {
    pub(crate) fn new(
        context: crate::endpoint_builder::EndpointBuilderContext,
        topic: String,
        type_info: crate::TypeInfo,
        schema: crate::dynamic::Schema,
    ) -> Self {
        Self {
            inner: super::PublisherBuilder::new(
                context,
                topic,
                crate::endpoint_builder::MessageEndpointType::prevalidated_dynamic(
                    type_info.clone(),
                    schema.clone(),
                ),
            ),
            type_info,
            schema,
        }
    }

    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.inner = self.inner.qos(qos);
        self
    }

    pub async fn build(self) -> Result<RawPublisher> {
        let validate = || -> std::result::Result<(), crate::dynamic::DynamicError> {
            self.schema.validate().map_err(|error| {
                crate::dynamic::DynamicError::SerializationError(error.to_string())
            })?;
            let hash = ros_z_schema::compute_hash(self.schema.as_ref()).map_err(|error| {
                crate::dynamic::DynamicError::SerializationError(error.to_string())
            })?;
            if hash != self.type_info.hash {
                return Err(crate::dynamic::DynamicError::SerializationError(
                    "raw publisher schema hash mismatch".into(),
                ));
            }
            Ok(())
        };
        validate().map_err(|source| crate::error::WireError::DynamicSchema {
            endpoint_kind: "publisher",
            topic: self.inner.topic.clone(),
            source,
        })?;
        Ok(RawPublisher {
            inner: self.inner.build().await?,
        })
    }
}

/// Marker payload for schema-free raw subscribers.
///
/// The raw subscriber path never decodes payload bytes into this type. It only
/// needs a concrete payload/codec pair so it can reuse the normal subscriber
/// builder and advertise discovered topic type metadata.
pub struct RawPayload;

/// No-op decoder used by schema-free raw subscribers.
pub struct RawPayloadCodec;

impl WireDecoder for RawPayloadCodec {
    type Input<'a> = &'a [u8];
    type Output = RawPayload;
    type Error = Infallible;

    fn deserialize(_input: Self::Input<'_>) -> std::result::Result<Self::Output, Self::Error> {
        Ok(RawPayload)
    }
}

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
/// message type and associated codec type. Built subscribers deliver [`Sample`]
/// values directly and do not deserialize with `C`.
pub struct RawSubscriberBuilder<T, C = <T as crate::Message>::Codec> {
    pub(crate) inner: SubscriberBuilder<T, C>,
}

impl<T, C> RawSubscriberBuilder<T, C>
where
    T: Send + Sync + 'static,
{
    /// Accept samples only from this publisher, including retained delivery.
    pub fn publisher(self, publisher: crate::attachment::EndpointGlobalId) -> Self {
        Self {
            inner: self.inner.publisher(publisher),
        }
    }
    pub fn qos(self, qos: QosProfile) -> Self {
        Self {
            inner: self.inner.qos(qos),
        }
    }

    /// Set the local raw subscriber receive queue capacity.
    ///
    /// This does not change advertised endpoint QoS. If unset, capacity is
    /// derived from the effective QoS history depth.
    pub fn queue_capacity(self, queue_capacity: std::num::NonZeroUsize) -> Self {
        Self {
            inner: self.inner.queue_capacity(queue_capacity),
        }
    }

    /// Set how local raw subscriber queue overflow is reported.
    ///
    /// This only controls log output. Overflow still drops the oldest queued
    /// sample and does not alter advertised endpoint QoS.
    pub fn queue_overflow_reporting(self, reporting: QueueOverflowReporting) -> Self {
        Self {
            inner: self.inner.queue_overflow_reporting(reporting),
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

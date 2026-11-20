use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tracing::{debug, warn};
use zenoh::liveliness::LivelinessToken;
use zenoh::{Session, sample::Sample};

use crate::dynamic::{DynamicCdrCodec, DynamicPayload, Schema};
use crate::endpoint_builder::{EndpointBuilderContext, MessageEndpointType};
use crate::entity::{EndpointEntity, EndpointKind};
use crate::graph::Graph;
use crate::message::WireDecoder;
use crate::pubsub::metadata::{Received, publication_id_from_sample};
use crate::pubsub::raw::{self, RawSubscriberBuilder};
use crate::pubsub::replay::{self, TransientLocalReplayCoordinator};
use crate::qos::QosProfile;
use crate::queue::BoundedQueue;
use crate::topic_name::qualify_topic_name;
use crate::{EndpointGlobalId, Result};
use ros_z_protocol::qos::{QosDurability, QosHistory};

pub(super) fn subscriber_queue_capacity(
    qos: &ros_z_protocol::qos::QosProfile,
    queue_capacity: Option<NonZeroUsize>,
) -> usize {
    if let Some(queue_capacity) = queue_capacity {
        return queue_capacity.get();
    }

    match qos.history {
        QosHistory::KeepLast(depth) => depth,
        QosHistory::KeepAll => usize::MAX,
    }
}

/// Controls how local subscriber queue overflow is reported.
///
/// This setting only affects reporting. It does not change advertised QoS,
/// queue capacity, or the queue drop policy. When the local receive queue is
/// full, the oldest queued sample is still dropped to make room for the newest
/// sample.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum QueueOverflowReporting {
    /// Do not emit a log message when the local queue overflows.
    ///
    /// Dropped samples are still accounted internally.
    Silent,
    /// Emit a debug log message for each local queue overflow.
    Debug,
    /// Emit a warning log message for each local queue overflow. This is the default reporting mode.
    #[default]
    Warn,
}

#[derive(Debug, Clone)]
pub(crate) struct SubscriberOptions {
    pub(crate) qos: ros_z_protocol::qos::QosProfile,
    pub(crate) locality: Option<zenoh::sample::Locality>,
    pub(crate) transient_local_replay_timeout: Duration,
    pub(crate) queue_capacity: Option<NonZeroUsize>,
    pub(crate) queue_overflow_reporting: QueueOverflowReporting,
}

impl Default for SubscriberOptions {
    fn default() -> Self {
        Self {
            qos: crate::endpoint_builder::default_protocol_qos(),
            locality: None,
            transient_local_replay_timeout: crate::pubsub::DEFAULT_TRANSIENT_LOCAL_REPLAY_TIMEOUT,
            queue_capacity: None,
            queue_overflow_reporting: QueueOverflowReporting::Warn,
        }
    }
}

impl SubscriberOptions {
    pub(crate) fn qos(mut self, qos: QosProfile) -> Self {
        self.qos = qos.to_protocol_qos();
        self
    }

    pub(crate) fn locality(mut self, locality: zenoh::sample::Locality) -> Self {
        self.locality = Some(locality);
        self
    }

    pub(crate) fn transient_local_replay_timeout(mut self, timeout: Duration) -> Self {
        self.transient_local_replay_timeout = timeout;
        self
    }

    pub(crate) fn queue_capacity(mut self, queue_capacity: NonZeroUsize) -> Self {
        self.queue_capacity = Some(queue_capacity);
        self
    }

    pub(crate) fn queue_overflow_reporting(mut self, reporting: QueueOverflowReporting) -> Self {
        self.queue_overflow_reporting = reporting;
        self
    }
}

pub struct SubscriberBuilder<T, C = <T as crate::Message>::Codec> {
    pub(crate) context: EndpointBuilderContext,
    pub(crate) topic: String,
    pub(crate) type_source: MessageEndpointType,
    pub(crate) options: SubscriberOptions,
    pub(crate) _phantom_data: PhantomData<(T, C)>,
}

pub(super) struct SubscriberResources {
    _replay_guard: Option<replay::TransientLocalReplayGuard>,
    _subscriber: zenoh::pubsub::Subscriber<()>,
    _liveliness_token: LivelinessToken,
}

struct PreparedSubscriberBuild {
    context: EndpointBuilderContext,
    options: SubscriberOptions,
    dyn_schema: Option<Schema>,
    entity: EndpointEntity,
}

#[derive(Debug, Clone)]
struct QueueDropContext {
    subscriber_id: EndpointGlobalId,
    log_prefix: &'static str,
    topic: String,
    node_namespace: String,
    node_name: String,
    type_name: String,
    queue_capacity: usize,
    queue_overflow_reporting: QueueOverflowReporting,
}

impl QueueDropContext {
    fn from_entity(
        log_prefix: &'static str,
        entity: &EndpointEntity,
        queue_capacity: usize,
        queue_overflow_reporting: QueueOverflowReporting,
    ) -> Result<Self> {
        let topic = qualify_topic_name(&entity.topic, &entity.node.namespace, &entity.node.name)
            .map_err(|source| crate::Error::topic_name(entity.topic.clone(), source))?;

        Ok(Self {
            subscriber_id: EndpointGlobalId::from(entity),
            log_prefix,
            topic,
            node_namespace: entity.node.namespace.clone(),
            node_name: entity.node.name.clone(),
            type_name: entity.type_info.name.clone(),
            queue_capacity,
            queue_overflow_reporting,
        })
    }
}

// Admission to the local queue, after any transient-local replay coordination.
#[tracing::instrument(name = "enqueue_sample", level = "trace", skip_all, fields(
    publication_id = trace_publication_id(&sample), subscriber_id = %context.subscriber_id,
    topic = %context.topic, node_name = %context.node_name, node_namespace = %context.node_namespace,
    queue_capacity = context.queue_capacity,
))]
fn record_queue_push(
    queue: &BoundedQueue<Sample>,
    dropped_samples: &AtomicU64,
    context: &QueueDropContext,
    sample: Sample,
) {
    let Some(victim) = queue.push(sample) else {
        return;
    };
    // The containing enqueue span describes the incoming sample, not the victim.
    tracing::trace!(name: "discarded", parent: None,
        publication_id = trace_publication_id(&victim), subscriber_id = %context.subscriber_id,
        topic = %context.topic, node_name = %context.node_name, node_namespace = %context.node_namespace,
        reason = "queue_overflow", queue_capacity = context.queue_capacity,
    );

    let total_dropped_samples = dropped_samples.fetch_add(1, Ordering::Relaxed) + 1;
    match context.queue_overflow_reporting {
        QueueOverflowReporting::Silent => {}
        QueueOverflowReporting::Debug => {
            debug!(
                subscriber = context.log_prefix,
                topic = %context.topic,
                node_namespace = %context.node_namespace,
                node_name = %context.node_name,
                type_name = %context.type_name,
                queue_capacity = context.queue_capacity,
                queue_overflow_reporting = ?context.queue_overflow_reporting,
                drop_policy = "oldest",
                total_dropped_samples,
                "subscriber queue full; dropped oldest sample"
            );
        }
        QueueOverflowReporting::Warn => {
            warn!(
                subscriber = context.log_prefix,
                topic = %context.topic,
                node_namespace = %context.node_namespace,
                node_name = %context.node_name,
                type_name = %context.type_name,
                queue_capacity = context.queue_capacity,
                queue_overflow_reporting = ?context.queue_overflow_reporting,
                drop_policy = "oldest",
                total_dropped_samples,
                "subscriber queue full; dropped oldest sample"
            );
        }
    }
}

// Only call inside a tracing field: raw samples need not have a ROS-Z attachment.
fn trace_publication_id(sample: &Sample) -> Option<impl tracing::field::Value> {
    // The `log` fallback can evaluate span fields even without a dispatcher.
    if tracing::level_filters::LevelFilter::current() < tracing::Level::TRACE {
        return None;
    }
    publication_id_from_sample(sample).map(tracing::field::display)
}

pub(super) fn trace_dequeue(id: EndpointGlobalId, entity: &EndpointEntity, sample: &Sample) {
    tracing::trace!(name: "dequeue",
        publication_id = trace_publication_id(sample), subscriber_id = %id,
        topic = %entity.topic, node_name = %entity.node.name, node_namespace = %entity.node.namespace,
    );
}

#[tracing::instrument(level = "trace", skip_all, fields(
    publication_id = trace_publication_id(sample), subscriber_id = %id,
    topic = %entity.topic, node_name = %entity.node.name, node_namespace = %entity.node.namespace,
), err(level = "trace"))]
fn process_sample<T>(
    id: EndpointGlobalId,
    entity: &EndpointEntity,
    sample: &Sample,
    decode: impl FnOnce(&[u8]) -> Result<T>,
) -> Result<Received<T>> {
    let message = decode(&sample.payload().to_bytes())?;
    Received::try_from_sample(sample, message)
}

async fn declare_liveliness(session: &Session, entity: &EndpointEntity) -> Result<LivelinessToken> {
    let liveliness_key_expr = entity.liveliness_key_expr()?.0;
    session
        .liveliness()
        .declare_token(liveliness_key_expr)
        .await
        .map_err(|source| crate::Error::zenoh("declare subscriber liveliness token", source))
}

impl<T, C> SubscriberBuilder<T, C> {
    pub(crate) fn new(
        context: EndpointBuilderContext,
        topic: String,
        type_source: MessageEndpointType,
    ) -> Self {
        Self {
            context,
            topic,
            type_source,
            options: SubscriberOptions::default(),
            _phantom_data: Default::default(),
        }
    }

    pub(crate) fn options(mut self, options: SubscriberOptions) -> Self {
        self.options = options;
        self
    }

    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.options = self.options.qos(qos);
        self
    }

    /// Set the local subscriber receive queue capacity.
    ///
    /// This does not change advertised endpoint QoS. If unset, capacity is
    /// derived from the effective QoS history depth.
    pub fn queue_capacity(mut self, queue_capacity: NonZeroUsize) -> Self {
        self.options = self.options.queue_capacity(queue_capacity);
        self
    }

    /// Set how local subscriber queue overflow is reported.
    ///
    /// This only controls log output. Overflow still drops the oldest queued
    /// sample and does not alter advertised endpoint QoS.
    pub fn queue_overflow_reporting(mut self, reporting: QueueOverflowReporting) -> Self {
        self.options = self.options.queue_overflow_reporting(reporting);
        self
    }

    /// Set the locality restriction for this subscription.
    ///
    /// This restricts the subscription to only receive samples from publishers
    /// with the specified locality (local/remote/any).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use zenoh::sample::Locality;
    ///
    /// let subscriber = node
    ///     .subscriber::<String>("/topic")
    ///     .locality(Locality::Remote)  // Only receive from remote publishers
    ///     .build()
    ///     .await?;
    /// ```
    pub fn locality(mut self, locality: zenoh::sample::Locality) -> Self {
        self.options = self.options.locality(locality);
        self
    }

    pub fn transient_local_replay_timeout(mut self, timeout: Duration) -> Self {
        self.options = self.options.transient_local_replay_timeout(timeout);
        self
    }

    /// Switch this builder to raw sample delivery.
    ///
    /// Only settings that affect raw sample delivery continue to apply.
    pub fn raw(self) -> RawSubscriberBuilder<T, C> {
        RawSubscriberBuilder { inner: self }
    }

    fn prepare_build(self, log_prefix: &str) -> Result<PreparedSubscriberBuild> {
        let Self {
            context,
            topic,
            type_source,
            options,
            ..
        } = self;
        let (type_info, dyn_schema) = type_source.resolve_for_subscriber(&topic)?;
        let qualified_topic =
            qualify_topic_name(&topic, &context.node.namespace, &context.node.name)
                .map_err(|source| crate::Error::topic_name(topic, source))?;

        let entity = context.endpoint_entity(
            EndpointKind::Subscription,
            qualified_topic,
            type_info,
            options.qos,
        );
        debug!("[{}] Qualified topic: {}", log_prefix, entity.topic);
        Ok(PreparedSubscriberBuild {
            context,
            options,
            dyn_schema,
            entity,
        })
    }

    pub(crate) async fn build_raw_queue_async(self) -> Result<raw::RawSubscriber> {
        let prepared = self.prepare_build("RAW_SUB")?;
        let (subscriber_id, queue, resources) = prepared.build_queue("RAW_SUB").await?;
        Ok(raw::RawSubscriber::new(
            subscriber_id,
            prepared.entity,
            queue,
            resources,
        ))
    }
}

impl PreparedSubscriberBuild {
    async fn build_queue(
        &self,
        log_prefix: &'static str,
    ) -> Result<(
        EndpointGlobalId,
        Arc<BoundedQueue<Sample>>,
        SubscriberResources,
    )> {
        let capacity = subscriber_queue_capacity(&self.entity.qos, self.options.queue_capacity);
        let queue = Arc::new(BoundedQueue::new(capacity));
        let receiving_queue = queue.clone();
        let context = QueueDropContext::from_entity(
            log_prefix,
            &self.entity,
            capacity,
            self.options.queue_overflow_reporting,
        )?;
        let id = context.subscriber_id;
        let dropped = AtomicU64::new(0);
        let resources = self
            .build_subscriber_resources(
                &self.entity,
                move |sample| {
                    record_queue_push(&receiving_queue, &dropped, &context, sample);
                },
                log_prefix,
            )
            .await?;
        self.warn_about_incompatible_endpoints(log_prefix);
        // Report successful ownership only. Initial replay may already have enqueued samples.
        tracing::trace!(name: "subscriber_created",
            endpoint_id = %id, topic = %self.entity.topic,
            node_zid = %self.entity.node.z_id, node_id = self.entity.node.id,
            node_name = %self.entity.node.name, node_namespace = %self.entity.node.namespace,
        );
        Ok((id, queue, resources))
    }

    fn warn_about_incompatible_endpoints(&self, log_prefix: &str) {
        for endpoint in self
            .context
            .graph
            .type_incompatible_endpoints_for(&self.entity)
        {
            warn!(
                topic = %self.entity.topic,
                subscriber_node = %self.entity.node.fully_qualified_name(),
                subscriber_type = %self.entity.type_info.name,
                subscriber_schema_hash = %self.entity.type_info.hash,
                endpoint_kind = ?endpoint.kind,
                endpoint_node = %endpoint.node.fully_qualified_name(),
                endpoint_type = %endpoint.type_info.name,
                endpoint_schema_hash = %endpoint.type_info.hash,
                "[{log_prefix}] endpoint type metadata does not match subscriber"
            );
        }
    }

    async fn build_subscriber_resources<F>(
        &self,
        entity: &EndpointEntity,
        callback: F,
        log_prefix: &str,
    ) -> Result<SubscriberResources>
    where
        F: Fn(Sample) + Send + Sync + 'static,
    {
        let topic_key_expr = ros_z_protocol::format::topic_key_expr(entity)?;
        let key_expr = (*topic_key_expr).clone();
        debug!(
            "[{}] Key expression: {}, qos={:?}",
            log_prefix, key_expr, entity.qos
        );

        let callback: Arc<dyn Fn(Sample) + Send + Sync> = Arc::new(callback);

        if !matches!(entity.qos.durability, QosDurability::TransientLocal) {
            let subscriber_callback = callback.clone();
            let mut subscriber = self
                .context
                .session
                .declare_subscriber(key_expr)
                .callback(move |sample| subscriber_callback(sample));

            if let Some(locality) = self.options.locality {
                subscriber = subscriber.allowed_origin(locality);
            }

            let subscriber = subscriber
                .await
                .map_err(|source| crate::Error::zenoh("declare subscriber", source))?;
            let liveliness_token = declare_liveliness(&self.context.session, entity).await?;
            Ok(SubscriberResources {
                _subscriber: subscriber,
                _liveliness_token: liveliness_token,
                _replay_guard: None,
            })
        } else {
            let Some(live_capacity) = replay::transient_local_replay_live_capacity(
                &entity.qos,
                self.options.queue_capacity,
            ) else {
                warn!(
                    "[{}] TransientLocal + KeepAll requested; replay coordination is disabled because history is unbounded",
                    log_prefix
                );
                let subscriber_callback = callback.clone();
                let mut subscriber = self
                    .context
                    .session
                    .declare_subscriber(key_expr)
                    .callback(move |sample| subscriber_callback(sample));

                if let Some(locality) = self.options.locality {
                    subscriber = subscriber.allowed_origin(locality);
                }

                let subscriber = subscriber
                    .await
                    .map_err(|source| crate::Error::zenoh("declare subscriber", source))?;
                let liveliness_token = declare_liveliness(&self.context.session, entity).await?;
                return Ok(SubscriberResources {
                    _subscriber: subscriber,
                    _liveliness_token: liveliness_token,
                    _replay_guard: None,
                });
            };
            let cancelled = Arc::new(AtomicBool::new(false));
            let coordinator = Arc::new(TransientLocalReplayCoordinator::new(
                live_capacity,
                callback,
                cancelled.clone(),
            ));
            let live_coordinator = coordinator.clone();
            let mut subscriber = self
                .context
                .session
                .declare_subscriber(key_expr)
                .callback(move |sample| live_coordinator.handle_live(sample));

            if let Some(locality) = self.options.locality {
                subscriber = subscriber.allowed_origin(locality);
            }

            let subscriber = subscriber
                .await
                .map_err(|source| crate::Error::zenoh("declare subscriber", source))?;

            let (initial_replay_publishers, initial_replay_seen) = replay::initial_replay_plan(
                replay::replay_capable_publishers(&self.context.graph, &entity.topic),
            );
            for &(publisher_global_id, live_capacity) in &initial_replay_publishers {
                coordinator.begin_initial_publisher(publisher_global_id, live_capacity);
            }
            for (publisher_global_id, _) in initial_replay_publishers {
                let replay_result = replay::query_initial_transient_local_replay_async(
                    &self.context.session,
                    &topic_key_expr,
                    publisher_global_id,
                    self.options.transient_local_replay_timeout,
                    coordinator.clone(),
                )
                .await;
                coordinator.finish_initial_publisher(publisher_global_id);
                replay_result?;
            }
            coordinator.finish_initial_replay();
            let replay_task = replay::spawn_transient_local_replay_task(
                self.context.graph.clone(),
                entity.topic.clone(),
                coordinator,
                self.context.session.clone(),
                topic_key_expr.to_string(),
                self.options.transient_local_replay_timeout,
                initial_replay_seen,
            );
            let replay_guard = replay::TransientLocalReplayGuard::new(cancelled, replay_task);
            let liveliness_token = declare_liveliness(&self.context.session, entity).await?;
            Ok(SubscriberResources {
                _subscriber: subscriber,
                _liveliness_token: liveliness_token,
                _replay_guard: Some(replay_guard),
            })
        }
    }
}

impl<T, C> SubscriberBuilder<T, C>
where
    C: WireDecoder,
{
    pub async fn build(self) -> Result<Subscriber<T, C>>
    where
        C: WireDecoder,
    {
        let prepared = self.prepare_build("SUB")?;
        let (subscriber_id, queue, resources) = prepared.build_queue("SUB").await?;
        debug!("[SUB] Subscriber ready: topic={}", prepared.entity.topic);

        let PreparedSubscriberBuild {
            context,
            dyn_schema,
            entity,
            ..
        } = prepared;

        Ok(Subscriber {
            subscriber_id,
            entity,
            _resources: resources,
            queue,
            graph: context.graph,
            dyn_schema,
            _phantom_data: Default::default(),
        })
    }
}

pub struct Subscriber<T, C: WireDecoder = <T as crate::Message>::Codec> {
    subscriber_id: EndpointGlobalId,
    entity: EndpointEntity,
    queue: Arc<BoundedQueue<Sample>>,
    _resources: SubscriberResources,
    graph: Arc<Graph>,
    /// Schema for dynamic message deserialization.
    /// Required for runtime-typed dynamic subscribers using `DynamicPayload`.
    dyn_schema: Option<Schema>,
    _phantom_data: PhantomData<(T, C)>,
}

impl<T, C: WireDecoder> std::fmt::Debug for Subscriber<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber")
            .field("entity", &self.entity)
            .finish_non_exhaustive()
    }
}

impl<T, C> Subscriber<T, C>
where
    T: Send + Sync + 'static,
    C: WireDecoder,
{
    /// Get a reference to the endpoint entity for this subscriber.
    pub fn entity(&self) -> &EndpointEntity {
        &self.entity
    }

    /// Check if there are messages available in the queue
    pub fn is_ready(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Wait until at least `count` publishers are matched on this subscriber's topic,
    /// or until `timeout` elapses.
    ///
    /// Returns `true` if the required number of publishers appeared within the
    /// timeout, `false` otherwise.
    ///
    /// This mirrors `Publisher::wait_for_subscribers` but from the subscriber side.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Ensure at least one publisher is ready before receiving.
    /// assert!(subscriber.wait_for_publishers(1, Duration::from_secs(5)).await);
    /// ```
    pub async fn wait_for_publishers(&self, count: usize, timeout: Duration) -> bool {
        let topic = self.entity.topic.as_str();
        let wait = self
            .graph
            .wait_until(move |data| data.publishers_on(topic).count() >= count);

        match tokio::time::timeout(timeout, wait).await {
            Ok(true) => true,
            Ok(false) | Err(_) => {
                self.graph.lock().publishers_on(&self.entity.topic).count() >= count
            }
        }
    }
}

impl<T, C> Subscriber<T, C>
where
    T: Send + Sync + 'static,
    C: for<'a> WireDecoder<Input<'a> = &'a [u8]>,
{
    /// Return the number of matched publishers currently visible in the graph.
    pub fn publisher_count(&self) -> usize {
        self.graph.lock().publishers_on(&self.entity.topic).count()
    }

    /// Return whether at least one publisher is currently matched.
    pub fn has_publishers(&self) -> bool {
        self.publisher_count() > 0
    }

    pub async fn recv(&self) -> Result<C::Output> {
        self.recv_with_metadata().await.map(Received::into_message)
    }

    /// Receive and deserialize the next message together with metadata.
    #[tracing::instrument(level = "trace", skip_all, fields(
        subscriber_id = %self.subscriber_id, topic = %self.entity.topic,
        node_name = %self.entity.node.name, node_namespace = %self.entity.node.namespace,
    ))]
    pub async fn recv_with_metadata(&self) -> Result<Received<C::Output>> {
        let sample = self.queue.recv_async().await;
        trace_dequeue(self.subscriber_id, &self.entity, &sample);
        process_sample(self.subscriber_id, &self.entity, &sample, |payload| {
            C::deserialize(payload)
                .map_err(|source| crate::Error::decode(std::any::type_name::<C::Output>(), source))
        })
    }
}

// Specialized implementation for DynamicPayload
impl Subscriber<DynamicPayload, DynamicCdrCodec> {
    /// Receive and deserialize the next dynamic message.
    ///
    /// This method requires that the subscriber was built through a dynamic
    /// subscriber factory.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The subscriber was built with a callback (no queue available)
    /// - The dynamic schema was not set by the dynamic subscriber factory
    /// - Deserialization fails
    pub async fn recv(&self) -> Result<DynamicPayload> {
        self.recv_with_metadata().await.map(Received::into_message)
    }

    #[tracing::instrument(level = "trace", skip_all, fields(
        subscriber_id = %self.subscriber_id, topic = %self.entity.topic,
        node_name = %self.entity.node.name, node_namespace = %self.entity.node.namespace,
    ))]
    pub async fn recv_with_metadata(&self) -> Result<Received<DynamicPayload>> {
        let schema = self.dyn_schema.as_ref().ok_or_else(|| {
            crate::error::WireError::MissingDynamicSchema {
                topic: self.entity.topic.clone(),
            }
        })?;

        let sample = self.queue.recv_async().await;
        trace_dequeue(self.subscriber_id, &self.entity, &sample);
        process_sample(self.subscriber_id, &self.entity, &sample, |payload| {
            DynamicCdrCodec::deserialize((payload, schema))
                .map_err(|source| crate::Error::decode("ros_z::dynamic::DynamicPayload", source))
        })
    }

    /// Get the dynamic schema.
    pub fn schema(&self) -> Option<&ros_z_schema::SchemaBundle> {
        self.dyn_schema.as_ref().map(|s| s.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_drop_context(reporting: QueueOverflowReporting) -> QueueDropContext {
        QueueDropContext {
            subscriber_id: EndpointGlobalId::ZERO,
            log_prefix: "TEST_SUB",
            topic: "/test_topic".to_owned(),
            node_namespace: "/".to_owned(),
            node_name: "test_node".to_owned(),
            type_name: "test_msgs/msg/Test".to_owned(),
            queue_capacity: 2,
            queue_overflow_reporting: reporting,
        }
    }

    fn overflow_queue(reporting: QueueOverflowReporting) {
        let queue = BoundedQueue::new(2);
        let dropped_samples = AtomicU64::new(0);
        let context = queue_drop_context(reporting);

        // Raw samples without ROS-Z attachments still participate in overflow.
        for byte in [1, 2, 3] {
            let key = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
            let sample = zenoh::sample::SampleBuilder::put(key, [byte]).into();
            record_queue_push(&queue, &dropped_samples, &context, sample);
        }

        assert_eq!(&*queue.try_recv().unwrap().payload().to_bytes(), &[2]);
        assert_eq!(&*queue.try_recv().unwrap().payload().to_bytes(), &[3]);
        assert!(queue.try_recv().is_none());
        assert_eq!(dropped_samples.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn overflow_drops_oldest_and_counts_regardless_of_reporting() {
        use QueueOverflowReporting::{Debug, Silent, Warn};
        for policy in [Silent, Debug, Warn] {
            overflow_queue(policy);
        }
    }
}

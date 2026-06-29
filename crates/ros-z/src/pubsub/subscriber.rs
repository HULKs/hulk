use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tracing::{debug, warn};
use zenoh::liveliness::LivelinessToken;
use zenoh::{Session, sample::Sample};

use crate::Result;
use crate::dynamic::{DynamicCdrCodec, DynamicPayload, Schema};
use crate::endpoint_builder::{EndpointBuilderContext, MessageEndpointType};
use crate::entity::{EndpointEntity, EndpointKind};
use crate::graph::Graph;
use crate::message::WireDecoder;
use crate::pubsub::metadata::Received;
use crate::pubsub::raw::{self, RawSubscriberBuilder};
use crate::pubsub::replay::{self, TransientLocalReplayCoordinator};
use crate::qos::QosProfile;
use crate::queue::BoundedQueue;
use crate::topic_name::qualify_topic_name;
use ros_z_protocol::qos::{QosDurability, QosHistory};

pub(super) fn subscriber_queue_capacity(qos: &ros_z_protocol::qos::QosProfile) -> usize {
    match qos.history {
        QosHistory::KeepLast(depth) => depth,
        QosHistory::KeepAll => usize::MAX,
    }
}

/// Default time a subscriber receive waits before warning that no publishers are visible.
///
/// The warning is informational only: receiving continues waiting for a sample after the
/// timeout fires.
pub const DEFAULT_PUBLISHER_WARNING_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub(crate) struct SubscriberOptions {
    pub(crate) qos: ros_z_protocol::qos::QosProfile,
    pub(crate) locality: Option<zenoh::sample::Locality>,
    pub(crate) transient_local_replay_timeout: Duration,
    pub(crate) publisher_warning_timeout: Option<Duration>,
}

impl Default for SubscriberOptions {
    fn default() -> Self {
        Self {
            qos: crate::endpoint_builder::default_protocol_qos(),
            locality: None,
            transient_local_replay_timeout: crate::pubsub::DEFAULT_TRANSIENT_LOCAL_REPLAY_TIMEOUT,
            publisher_warning_timeout: Some(DEFAULT_PUBLISHER_WARNING_TIMEOUT),
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

    pub(crate) fn publisher_warning_timeout(mut self, timeout: Duration) -> Self {
        self.publisher_warning_timeout = Some(timeout);
        self
    }

    pub(crate) fn without_publisher_warning(mut self) -> Self {
        self.publisher_warning_timeout = None;
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
    log_prefix: &'static str,
    topic: String,
    node_namespace: String,
    node_name: String,
    type_name: String,
    queue_capacity: usize,
}

impl QueueDropContext {
    fn from_entity(
        log_prefix: &'static str,
        entity: &EndpointEntity,
        queue_capacity: usize,
    ) -> Result<Self> {
        let topic = qualify_topic_name(&entity.topic, &entity.node.namespace, &entity.node.name)
            .map_err(|source| crate::Error::topic_name(entity.topic.clone(), source))?;

        Ok(Self {
            log_prefix,
            topic,
            node_namespace: entity.node.namespace.clone(),
            node_name: entity.node.name.clone(),
            type_name: entity.type_info.name.clone(),
            queue_capacity,
        })
    }
}

fn record_queue_push<T>(
    queue: &BoundedQueue<T>,
    dropped_samples: &AtomicU64,
    context: &QueueDropContext,
    sample: T,
) {
    if queue.push(sample) {
        let total_dropped_samples = dropped_samples.fetch_add(1, Ordering::Relaxed) + 1;
        warn!(
            subscriber = context.log_prefix,
            topic = %context.topic,
            node_namespace = %context.node_namespace,
            node_name = %context.node_name,
            type_name = %context.type_name,
            queue_capacity = context.queue_capacity,
            drop_policy = "oldest",
            total_dropped_samples,
            "subscriber queue full; dropped oldest sample"
        );
    }
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

    /// Configure how long receive waits before warning that no publishers are visible.
    ///
    /// The warning is emitted only when no sample arrives before `timeout` and the graph has no
    /// visible publishers for the subscriber topic. Receiving continues waiting after the warning.
    pub fn publisher_warning_timeout(mut self, timeout: Duration) -> Self {
        self.options = self.options.publisher_warning_timeout(timeout);
        self
    }

    /// Disable warnings when receive waits without any visible publishers.
    pub fn without_publisher_warning(mut self) -> Self {
        self.options = self.options.without_publisher_warning();
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
        let entity = &prepared.entity;
        let queue_size = subscriber_queue_capacity(&entity.qos);
        let queue = Arc::new(BoundedQueue::new(queue_size));
        let raw_queue = queue.clone();
        let dropped_samples = Arc::new(AtomicU64::new(0));
        let raw_dropped_samples = dropped_samples.clone();
        let drop_context = QueueDropContext::from_entity("RAW_SUB", entity, queue_size)?;
        let resources = prepared
            .build_subscriber_resources(
                entity,
                move |sample| {
                    record_queue_push(&raw_queue, &raw_dropped_samples, &drop_context, sample);
                },
                "RAW_SUB",
            )
            .await?;

        prepared.warn_about_incompatible_endpoints("RAW_SUB");

        Ok(raw::RawSubscriber::new(
            queue,
            resources,
            Arc::clone(&prepared.context.graph),
            prepared.entity.clone(),
            prepared.options.publisher_warning_timeout,
        ))
    }
}

impl PreparedSubscriberBuild {
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
            let Some(live_capacity) = replay::transient_local_replay_live_capacity(&entity.qos)
            else {
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
            for (publisher_global_id, _) in initial_replay_publishers {
                replay::query_initial_transient_local_replay_async(
                    &self.context.session,
                    &topic_key_expr,
                    publisher_global_id,
                    self.options.transient_local_replay_timeout,
                    coordinator.clone(),
                )
                .await?;
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
        let entity = &prepared.entity;
        let queue_size = subscriber_queue_capacity(&entity.qos);
        let queue = Arc::new(BoundedQueue::new(queue_size));
        let subscriber_queue = queue.clone();
        let dropped_samples = Arc::new(AtomicU64::new(0));
        let subscriber_dropped_samples = dropped_samples.clone();
        let drop_context = QueueDropContext::from_entity("SUB", entity, queue_size)?;
        let resources = prepared
            .build_subscriber_resources(
                entity,
                move |sample| {
                    record_queue_push(
                        &subscriber_queue,
                        &subscriber_dropped_samples,
                        &drop_context,
                        sample,
                    );
                },
                "SUB",
            )
            .await?;

        prepared.warn_about_incompatible_endpoints("SUB");

        debug!("[SUB] Subscriber ready: topic={}", entity.topic);

        let PreparedSubscriberBuild {
            context,
            dyn_schema,
            entity,
            options,
        } = prepared;

        Ok(Subscriber {
            entity,
            _resources: resources,
            queue,
            graph: context.graph,
            dyn_schema,
            publisher_warning_timeout: options.publisher_warning_timeout,
            _phantom_data: Default::default(),
        })
    }
}

pub(super) async fn recv_sample_with_publisher_warning(
    queue: &BoundedQueue<Sample>,
    graph: &Graph,
    entity: &EndpointEntity,
    publisher_warning_timeout: Option<Duration>,
) -> Sample {
    let Some(timeout) = publisher_warning_timeout else {
        return queue.recv_async().await;
    };

    match tokio::time::timeout(timeout, queue.recv_async()).await {
        Ok(sample) => sample,
        Err(_) => {
            if !graph.view().has_publishers_on(&entity.topic) {
                warn!(
                    topic = %entity.topic,
                    subscriber_node = %entity.node.fully_qualified_name(),
                    subscriber_type = %entity.type_info.name,
                    subscriber_schema_hash = %entity.type_info.hash,
                    timeout_seconds = timeout.as_secs_f64(),
                    "[SUB] no message received and no publishers are visible for subscriber topic"
                );
            }
            queue.recv_async().await
        }
    }
}

pub struct Subscriber<T, C: WireDecoder = <T as crate::Message>::Codec> {
    entity: EndpointEntity,
    queue: Arc<BoundedQueue<Sample>>,
    _resources: SubscriberResources,
    graph: Arc<Graph>,
    /// Schema for dynamic message deserialization.
    /// Required for runtime-typed dynamic subscribers using `DynamicPayload`.
    dyn_schema: Option<Schema>,
    publisher_warning_timeout: Option<Duration>,
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
            .wait_until(move |view| view.publisher_count_on(topic) >= count);

        match tokio::time::timeout(timeout, wait).await {
            Ok(true) => true,
            Ok(false) | Err(_) => self.graph.view().publisher_count_on(&self.entity.topic) >= count,
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
        self.graph.view().publisher_count_on(&self.entity.topic)
    }

    /// Return whether at least one publisher is currently matched.
    pub fn has_publishers(&self) -> bool {
        self.publisher_count() > 0
    }

    /// Receive and deserialize the next message.
    ///
    /// By default, this logs a warning after [`DEFAULT_PUBLISHER_WARNING_TIMEOUT`] if no sample
    /// arrives and no publishers are visible for the topic. The warning does not end the receive;
    /// this method continues waiting for the next message.
    pub async fn recv(&self) -> Result<C::Output> {
        self.recv_with_metadata().await.map(Received::into_message)
    }

    /// Receive and deserialize the next message together with metadata.
    ///
    /// The publisher warning behavior is the same as [`recv`](Self::recv).
    pub async fn recv_with_metadata(&self) -> Result<Received<C::Output>> {
        let sample = recv_sample_with_publisher_warning(
            &self.queue,
            &self.graph,
            &self.entity,
            self.publisher_warning_timeout,
        )
        .await;
        let payload = sample.payload().to_bytes();
        let message = C::deserialize(&payload)
            .map_err(|source| crate::Error::decode(std::any::type_name::<C::Output>(), source))?;
        Received::try_from_sample(&sample, message)
    }
}

// Specialized implementation for DynamicPayload
impl Subscriber<DynamicPayload, DynamicCdrCodec> {
    /// Receive and deserialize the next dynamic message.
    ///
    /// This method requires that the subscriber was built through a dynamic
    /// subscriber factory.
    ///
    /// By default, this logs a warning after [`DEFAULT_PUBLISHER_WARNING_TIMEOUT`] if no sample
    /// arrives and no publishers are visible for the topic. The warning does not end the receive;
    /// this method continues waiting for the next message.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The subscriber was built with a callback (no queue available)
    /// - The dynamic schema was not set by the dynamic subscriber factory
    /// - Deserialization fails
    #[tracing::instrument(name = "recv_dynamic", skip(self), fields(
        topic = %self.entity.topic,
        payload_len = tracing::field::Empty
    ))]
    pub async fn recv(&self) -> Result<DynamicPayload> {
        self.recv_with_metadata().await.map(Received::into_message)
    }

    /// Receive and deserialize the next dynamic message together with metadata.
    ///
    /// The publisher warning behavior is the same as [`recv`](Self::recv).
    pub async fn recv_with_metadata(&self) -> Result<Received<DynamicPayload>> {
        let schema = self.dyn_schema.as_ref().ok_or_else(|| {
            crate::error::WireError::MissingDynamicSchema {
                topic: self.entity.topic.clone(),
            }
        })?;

        let sample = recv_sample_with_publisher_warning(
            &self.queue,
            &self.graph,
            &self.entity,
            self.publisher_warning_timeout,
        )
        .await;
        let payload = sample.payload().to_bytes();

        let message = DynamicCdrCodec::deserialize((&payload, schema))
            .map_err(|source| crate::Error::decode("ros_z::dynamic::DynamicPayload", source))?;
        Received::try_from_sample(&sample, message)
    }

    /// Get the dynamic schema.
    pub fn schema(&self) -> Option<&ros_z_schema::SchemaBundle> {
        self.dyn_schema.as_ref().map(|s| s.as_ref())
    }
}

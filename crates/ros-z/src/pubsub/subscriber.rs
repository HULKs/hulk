use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tracing::{debug, warn};
use zenoh::liveliness::LivelinessToken;
use zenoh::{Result, Session, sample::Sample};

use crate::entity::{EndpointEntity, EntityKind};
use crate::event::EventsManager;
use crate::graph::Graph;
use crate::impl_with_type_info;
use crate::msg::WireDecoder;
use crate::pubsub::metadata::Received;
use crate::pubsub::raw::{self, RawSubscriberBuilder};
use crate::pubsub::replay::{self, TransientLocalReplayCoordinator};
use crate::qos::QosProfile;
use crate::queue::BoundedQueue;
use ros_z_protocol::qos::{QosDurability, QosHistory};

pub(super) fn subscriber_queue_capacity(qos: &ros_z_protocol::qos::QosProfile) -> usize {
    match qos.history {
        QosHistory::KeepLast(depth) => depth,
        QosHistory::KeepAll => usize::MAX,
    }
}

pub struct SubscriberBuilder<T, C = <T as crate::Message>::Codec> {
    pub(crate) entity: EndpointEntity,
    pub(crate) session: Arc<Session>,
    pub(crate) graph: Arc<Graph>,
    pub(crate) dyn_schema: Option<Arc<crate::dynamic::schema::TypeShape>>,
    pub(crate) locality: Option<zenoh::sample::Locality>,
    pub(crate) transient_local_replay_timeout: Duration,
    pub(crate) _phantom_data: PhantomData<(T, C)>,
}

pub(super) struct SubscriberResources {
    _replay_guard: Option<replay::TransientLocalReplayGuard>,
    _subscriber: zenoh::pubsub::Subscriber<()>,
    _liveliness_token: LivelinessToken,
}

async fn declare_liveliness(session: &Session, entity: &EndpointEntity) -> Result<LivelinessToken> {
    let liveliness_key_expr = ros_z_protocol::format::liveliness_key_expr(entity, &session.zid())?;
    session
        .liveliness()
        .declare_token((*liveliness_key_expr).clone())
        .await
}

impl_with_type_info!(SubscriberBuilder<T, C>);

impl<T, C> SubscriberBuilder<T, C>
where
    T: Send + Sync + 'static,
{
    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.entity.qos = qos.to_protocol_qos();
        self
    }

    pub fn codec<C2>(self) -> SubscriberBuilder<T, C2> {
        SubscriberBuilder {
            entity: self.entity,
            session: self.session,
            graph: self.graph,
            dyn_schema: self.dyn_schema,
            locality: self.locality,
            transient_local_replay_timeout: self.transient_local_replay_timeout,
            _phantom_data: PhantomData,
        }
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
        self.locality = Some(locality);
        self
    }

    pub fn transient_local_replay_timeout(mut self, timeout: Duration) -> Self {
        self.transient_local_replay_timeout = timeout;
        self
    }

    /// Set the dynamic root schema for runtime-typed messages.
    pub fn dyn_root_schema(
        mut self,
        root_name: &str,
        schema: Arc<crate::dynamic::schema::TypeShape>,
    ) -> Self {
        if self.entity.type_info.is_none()
            && let Some(hash) = crate::dynamic::schema_tree_hash(root_name, &schema)
        {
            self.entity.type_info = Some(crate::entity::TypeInfo::with_hash(root_name, hash));
        }

        self.dyn_schema = Some(schema);
        self
    }

    /// Switch this builder to raw sample delivery.
    ///
    /// Only settings that affect raw sample delivery continue to apply.
    pub fn raw(self) -> RawSubscriberBuilder<T, C> {
        RawSubscriberBuilder { inner: self }
    }

    pub(crate) async fn build_raw_queue_async(mut self) -> Result<raw::RawSubscriber> {
        let queue_size = subscriber_queue_capacity(&self.entity.qos);
        let queue = Arc::new(BoundedQueue::new(queue_size));
        let raw_queue = queue.clone();
        let resources = self
            .build_subscriber_resources(
                move |sample| {
                    if raw_queue.push(sample) {
                        tracing::debug!("Queue full, dropped oldest message");
                    }
                },
                "RAW_SUB",
            )
            .await?;

        Ok(raw::RawSubscriber::new(queue, resources))
    }

    async fn build_subscriber_resources<F>(
        &mut self,
        callback: F,
        log_prefix: &str,
    ) -> Result<SubscriberResources>
    where
        F: Fn(Sample) + Send + Sync + 'static,
    {
        let Some(node) = self.entity.node.as_ref() else {
            return Err(zenoh::Error::from(
                "subscriber build requires node identity",
            ));
        };
        let qualified_topic =
            crate::topic_name::qualify_topic_name(&self.entity.topic, &node.namespace, &node.name)
                .map_err(|e| zenoh::Error::from(format!("Failed to qualify topic: {}", e)))?;

        self.entity.topic = qualified_topic.clone();
        debug!("[{}] Qualified topic: {}", log_prefix, qualified_topic);

        let topic_key_expr = ros_z_protocol::format::topic_key_expr(&self.entity)?;
        let key_expr = (*topic_key_expr).clone();
        debug!(
            "[{}] Key expression: {}, qos={:?}",
            log_prefix, key_expr, self.entity.qos
        );

        let callback: Arc<dyn Fn(Sample) + Send + Sync> = Arc::new(callback);

        if !matches!(self.entity.qos.durability, QosDurability::TransientLocal) {
            let subscriber_callback = callback.clone();
            let mut subscriber = self
                .session
                .declare_subscriber(key_expr)
                .callback(move |sample| subscriber_callback(sample));

            if let Some(locality) = self.locality {
                subscriber = subscriber.allowed_origin(locality);
            }

            let subscriber = subscriber.await?;
            let liveliness_token = declare_liveliness(&self.session, &self.entity).await?;
            Ok(SubscriberResources {
                _subscriber: subscriber,
                _liveliness_token: liveliness_token,
                _replay_guard: None,
            })
        } else {
            let Some(live_capacity) =
                replay::transient_local_replay_live_capacity(&self.entity.qos)
            else {
                warn!(
                    "[{}] TransientLocal + KeepAll requested; replay coordination is disabled because history is unbounded",
                    log_prefix
                );
                let subscriber_callback = callback.clone();
                let mut subscriber = self
                    .session
                    .declare_subscriber(key_expr)
                    .callback(move |sample| subscriber_callback(sample));

                if let Some(locality) = self.locality {
                    subscriber = subscriber.allowed_origin(locality);
                }

                let subscriber = subscriber.await?;
                let liveliness_token = declare_liveliness(&self.session, &self.entity).await?;
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
                .session
                .declare_subscriber(key_expr)
                .callback(move |sample| live_coordinator.handle_live(sample));

            if let Some(locality) = self.locality {
                subscriber = subscriber.allowed_origin(locality);
            }

            let subscriber = subscriber.await?;

            let (initial_replay_publishers, initial_replay_seen) = replay::initial_replay_plan(
                replay::replay_capable_publishers(&self.graph, &self.entity.topic),
            );
            for (publisher_global_id, _) in initial_replay_publishers {
                replay::query_initial_transient_local_replay_async(
                    &self.session,
                    &topic_key_expr,
                    publisher_global_id,
                    self.transient_local_replay_timeout,
                    coordinator.clone(),
                )
                .await?;
            }
            coordinator.finish_initial_replay();
            let replay_task = replay::spawn_transient_local_replay_task(
                self.graph.clone(),
                self.entity.topic.clone(),
                coordinator,
                self.session.clone(),
                topic_key_expr.to_string(),
                self.transient_local_replay_timeout,
                initial_replay_seen,
            );
            let liveliness_token = declare_liveliness(&self.session, &self.entity).await?;
            Ok(SubscriberResources {
                _subscriber: subscriber,
                _liveliness_token: liveliness_token,
                _replay_guard: Some(replay::TransientLocalReplayGuard::new(
                    cancelled,
                    replay_task,
                )),
            })
        }
    }

    pub async fn build(self) -> Result<Subscriber<T, C>>
    where
        C: WireDecoder,
    {
        let mut builder = self;
        let queue_size = subscriber_queue_capacity(&builder.entity.qos);
        let queue = Arc::new(BoundedQueue::new(queue_size));
        let subscriber_queue = queue.clone();
        let resources = builder
            .build_subscriber_resources(
                move |sample| {
                    if subscriber_queue.push(sample) {
                        tracing::debug!("Queue full, dropped oldest message");
                    }
                },
                "SUB",
            )
            .await?;

        let endpoint_global_id = crate::entity::endpoint_global_id(&builder.entity);

        debug!("[SUB] Subscriber ready: topic={}", builder.entity.topic);

        Ok(Subscriber {
            entity: builder.entity,
            _resources: resources,
            queue,
            events_mgr: Arc::new(Mutex::new(EventsManager::new(endpoint_global_id))),
            graph: builder.graph,
            dyn_schema: builder.dyn_schema,
            _phantom_data: Default::default(),
        })
    }
}

pub struct Subscriber<T, C: WireDecoder = <T as crate::Message>::Codec> {
    entity: EndpointEntity,
    queue: Arc<BoundedQueue<Sample>>,
    _resources: SubscriberResources,
    events_mgr: Arc<Mutex<EventsManager>>,
    graph: Arc<Graph>,
    /// Schema for dynamic message deserialization.
    /// Required when using `DynamicStruct` with `DynamicCdrCodec`.
    dyn_schema: Option<Arc<crate::dynamic::schema::TypeShape>>,
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
    pub fn events_mgr(&self) -> &Arc<Mutex<EventsManager>> {
        &self.events_mgr
    }

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
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.graph.change_notify.notified();
            tokio::pin!(notified);

            let n = self
                .graph
                .get_entities_by_topic(EntityKind::Publisher, &self.entity.topic)
                .len();
            if n >= count {
                return true;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return false;
            }

            if tokio::time::timeout(remaining, &mut notified)
                .await
                .is_err()
            {
                return self
                    .graph
                    .get_entities_by_topic(EntityKind::Publisher, &self.entity.topic)
                    .len()
                    >= count;
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
        self.graph
            .get_entities_by_topic(EntityKind::Publisher, &self.entity.topic)
            .len()
    }

    /// Return whether at least one publisher is currently matched.
    pub fn has_publishers(&self) -> bool {
        self.publisher_count() > 0
    }

    pub async fn recv(&self) -> Result<C::Output> {
        self.recv_with_metadata().await.map(Received::into_message)
    }

    /// Receive and deserialize the next message together with metadata.
    pub async fn recv_with_metadata(&self) -> Result<Received<C::Output>> {
        let sample = self.queue.recv_async().await;
        let payload = sample.payload().to_bytes();
        let message = C::deserialize(&payload).map_err(|e| zenoh::Error::from(e.to_string()))?;
        Ok(Received::from_sample(&sample, message))
    }
}

// Specialized implementation for DynamicPayload
impl Subscriber<crate::dynamic::DynamicPayload, crate::dynamic::DynamicCdrCodec> {
    /// Receive and deserialize the next dynamic message.
    ///
    /// This method requires that the subscriber was built with `.dyn_schema()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The subscriber was built with a callback (no queue available)
    /// - The `dyn_schema` was not set via `.dyn_schema()`
    /// - Deserialization fails
    #[tracing::instrument(name = "recv_dynamic", skip(self), fields(
        topic = %self.entity.topic,
        payload_len = tracing::field::Empty
    ))]
    pub async fn recv(&self) -> Result<crate::dynamic::DynamicPayload> {
        self.recv_with_metadata().await.map(Received::into_message)
    }

    pub async fn recv_with_metadata(&self) -> Result<Received<crate::dynamic::DynamicPayload>> {
        let schema = self
            .dyn_schema
            .as_ref()
            .ok_or_else(|| zenoh::Error::from("dyn_schema required for DynamicPayload"))?;

        let sample = self.queue.recv_async().await;
        let payload = sample.payload().to_bytes();

        let message = crate::dynamic::DynamicCdrCodec::deserialize((&payload, schema))
            .map_err(|e| zenoh::Error::from(e.to_string()))?;
        Ok(Received::from_sample(&sample, message))
    }

    /// Get the dynamic schema.
    pub fn schema(&self) -> Option<&crate::dynamic::schema::TypeShape> {
        self.dyn_schema.as_ref().map(|s| s.as_ref())
    }
}

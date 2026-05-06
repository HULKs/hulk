use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::{debug, trace, warn};
use zenoh::liveliness::LivelinessToken;
use zenoh::{Result, Session};

use crate::attachment::{Attachment, EndpointGlobalId};
use crate::entity::{EndpointEntity, EntityKind};
use crate::event::EventsManager;
use crate::graph::Graph;
use crate::impl_with_type_info;
use crate::msg::WireEncoder;
use crate::pubsub::metadata::PublicationId;
use crate::pubsub::replay::{self, RetainedSample, TransientLocalCache};
use crate::qos::QosProfile;
use crate::topic_name;
use ros_z_protocol::qos::{QosDurability, QosHistory, QosReliability};

pub(super) fn validate_dynamic_publish_schema(
    advertised_schema: Option<&Arc<crate::dynamic::TypeShape>>,
    message: &crate::dynamic::DynamicPayload,
) -> Result<()> {
    let Some(advertised_schema) = advertised_schema else {
        return Ok(());
    };

    if advertised_schema.as_ref() == message.schema.as_ref() {
        return Ok(());
    }

    Err(zenoh::Error::from(
        "schema mismatch: dynamic payload schema does not match advertised schema",
    ))
}

pub struct Publisher<T, C: WireEncoder = <T as crate::Message>::Codec> {
    entity: EndpointEntity,
    /// Local monotonically increasing sequence used in publication attachments.
    sequence_number: AtomicUsize,
    /// Stable ros-z endpoint global ID derived from the node Zenoh id and endpoint-local id.
    endpoint_global_id: EndpointGlobalId,
    inner: zenoh::pubsub::Publisher<'static>,
    _lv_token: LivelinessToken,
    attachment: bool,
    clock: crate::time::Clock,
    events_mgr: Arc<Mutex<EventsManager>>,
    shm_config: Option<Arc<crate::shm::ShmConfig>>,
    /// Schema for dynamic message publishing.
    dyn_schema: Option<Arc<crate::dynamic::schema::TypeShape>>,
    /// Cached Zenoh CDR encoding for all published messages.
    encoding: Arc<zenoh::bytes::Encoding>,
    graph: Arc<Graph>,
    transient_local_cache: Option<Arc<TransientLocalCache>>,
    transient_local_replay_task: Option<JoinHandle<()>>,
    _phantom_data: PhantomData<(T, C)>,
}

#[must_use = "prepared publications reserve an id; call publish to send the message"]
pub struct PreparedPublication<'a, T, C>
where
    C: WireEncoder,
{
    publisher: &'a Publisher<T, C>,
    publication_id: PublicationId,
}

struct ReplayTaskGuard(Option<JoinHandle<()>>);

impl ReplayTaskGuard {
    fn new(task: Option<JoinHandle<()>>) -> Self {
        Self(task)
    }

    fn into_task(mut self) -> Option<JoinHandle<()>> {
        self.0.take()
    }
}

impl Drop for ReplayTaskGuard {
    fn drop(&mut self) {
        if let Some(task) = &self.0 {
            task.abort();
        }
    }
}

async fn spawn_transient_local_replay_queryable(
    session: &Arc<Session>,
    topic_key_expr: &ros_z_protocol::entity::TopicKE,
    endpoint_global_id: EndpointGlobalId,
    cache: Arc<TransientLocalCache>,
) -> Result<JoinHandle<()>> {
    let replay_key = replay::transient_local_replay_key(topic_key_expr, endpoint_global_id);
    let reply_key_expr = (**topic_key_expr).clone();
    let queryable = session.declare_queryable(replay_key).complete(true).await?;
    let queries = queryable.handler().clone();

    Ok(tokio::spawn(async move {
        let _queryable = queryable;
        loop {
            let query = match queries.recv_async().await {
                Ok(query) => query,
                Err(err) => {
                    warn!(
                        "[PUB] Transient local replay query receiver closed: {}",
                        err
                    );
                    break;
                }
            };
            for sample in cache.samples() {
                let mut reply = query.reply(&reply_key_expr, sample.payload);
                if let Some(encoding) = sample.encoding {
                    reply = reply.encoding(encoding);
                }
                if let Some(attachment) = sample.attachment {
                    reply = reply.attachment(attachment);
                }
                if let Err(err) = reply.await {
                    warn!("[PUB] Failed to replay transient local sample: {}", err);
                }
            }
        }
    }))
}

impl<'a, T, C> PreparedPublication<'a, T, C>
where
    T: Send + Sync + 'static,
    C: for<'de> WireEncoder<Input<'de> = &'de T> + 'static,
{
    pub fn id(&self) -> PublicationId {
        self.publication_id
    }

    pub async fn publish(self, message: &T) -> Result<()> {
        self.publisher
            .publish_with_reserved_id(message, self.publication_id)
            .await
    }
}

impl<T, C: WireEncoder> std::fmt::Debug for Publisher<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Publisher")
            .field("entity", &self.entity)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct PublisherBuilder<T, C = <T as crate::Message>::Codec> {
    pub(crate) entity: EndpointEntity,
    pub(crate) session: Arc<Session>,
    pub(crate) graph: Arc<Graph>,
    pub(crate) clock: crate::time::Clock,
    pub(crate) attachment: bool,
    pub(crate) shm_config: Option<Arc<crate::shm::ShmConfig>>,
    /// Schema for dynamic message publishing.
    /// When set, the schema will be registered with the schema service.
    pub(crate) dyn_schema: Option<Arc<crate::dynamic::schema::TypeShape>>,
    pub(crate) _phantom_data: PhantomData<(T, C)>,
}

impl_with_type_info!(PublisherBuilder<T, C>);

impl<T, C> PublisherBuilder<T, C> {
    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.entity.qos = qos.to_protocol_qos();
        self
    }

    pub fn attachment(mut self, with_attachment: bool) -> Self {
        self.attachment = with_attachment;
        self
    }

    /// Override SHM configuration for this publisher only.
    ///
    /// This overrides any SHM configuration inherited from the node or context.
    ///
    /// # Example
    /// ```no_run
    /// use ros_z::shm::{ShmConfig, ShmProviderBuilder};
    /// use std::sync::Arc;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> zenoh::Result<()> {
    /// # let context = ros_z::context::ContextBuilder::default().build().await?;
    /// # let node = context.create_node("test").build().await?;
    /// let provider = Arc::new(ShmProviderBuilder::new(20 * 1024 * 1024).build()?);
    /// let config = ShmConfig::new(provider).with_threshold(5_000);
    ///
    /// let publisher = node.publisher::<String>("topic")
    ///     .shm_config(config)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn shm_config(mut self, config: crate::shm::ShmConfig) -> Self {
        self.shm_config = Some(Arc::new(config));
        self
    }

    /// Disable SHM for this publisher.
    ///
    /// Even if SHM is enabled at the node or context level, this publisher
    /// will not use shared memory.
    ///
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> zenoh::Result<()> {
    /// # let context = ros_z::context::ContextBuilder::default().with_shm_enabled()?.build().await?;
    /// # let node = context.create_node("test").build().await?;
    /// // Context has SHM enabled, but disable for this publisher
    /// let publisher = node.publisher::<String>("small_messages")
    ///     .without_shm()
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn without_shm(mut self) -> Self {
        self.shm_config = None;
        self
    }

    pub fn codec<C2>(self) -> PublisherBuilder<T, C2> {
        PublisherBuilder {
            entity: self.entity,
            session: self.session,
            graph: self.graph,
            clock: self.clock,
            attachment: self.attachment,
            shm_config: self.shm_config,
            dyn_schema: self.dyn_schema,
            _phantom_data: PhantomData,
        }
    }

    /// Set the dynamic root schema for runtime-typed publishers.
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
}

impl<T, C> PublisherBuilder<T, C>
where
    T: Send + Sync + 'static,
    C: for<'a> WireEncoder<Input<'a> = &'a T> + 'static,
{
    #[tracing::instrument(name = "pub_build", skip(self), fields(
        topic = %self.entity.topic,
        qos_reliability = ?self.entity.qos.reliability,
        qos_durability = ?self.entity.qos.durability
    ))]
    pub async fn build(self) -> Result<Publisher<T, C>> {
        self.build_inner_async().await
    }

    fn prepare_build(
        &mut self,
    ) -> Result<(
        zenoh::key_expr::KeyExpr<'static>,
        ros_z_protocol::entity::TopicKE,
        Option<Arc<TransientLocalCache>>,
        EndpointGlobalId,
    )> {
        let Some(node) = self.entity.node.as_ref() else {
            return Err(zenoh::Error::from("publisher build requires node identity"));
        };
        // Qualify the topic name as a ros-z graph name.
        let qualified_topic =
            topic_name::qualify_topic_name(&self.entity.topic, &node.namespace, &node.name)
                .map_err(|e| zenoh::Error::from(format!("Failed to qualify topic: {}", e)))?;

        self.entity.topic = qualified_topic.clone();
        debug!("[PUB] Qualified topic: {}", qualified_topic);

        let topic_key_expr = ros_z_protocol::format::topic_key_expr(&self.entity)?;
        let data_key_expr = (*topic_key_expr).clone();
        debug!("[PUB] Key expression: {}", data_key_expr);

        if matches!(
            self.entity.qos,
            ros_z_protocol::qos::QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepAll,
                ..
            }
        ) {
            warn!(
                "[PUB] TransientLocal + KeepAll requested; late-join replay is disabled because history is unbounded"
            );
        }

        let transient_local_cache = replay::transient_local_cache_capacity(&self.entity.qos)
            .map(|capacity| Arc::new(TransientLocalCache::new(capacity)));
        let endpoint_global_id = crate::entity::endpoint_global_id(&self.entity);

        Ok((
            data_key_expr,
            topic_key_expr,
            transient_local_cache,
            endpoint_global_id,
        ))
    }

    async fn build_inner_async(mut self) -> Result<Publisher<T, C>> {
        let (key_expr, topic_key_expr, transient_local_cache, endpoint_global_id) =
            self.prepare_build()?;

        let mut pub_builder = self.session.declare_publisher(key_expr);

        match self.entity.qos.reliability {
            QosReliability::Reliable => {
                pub_builder = pub_builder.congestion_control(zenoh::qos::CongestionControl::Block);
                debug!("[PUB] QoS: Reliable (Block)");
            }
            QosReliability::BestEffort => {
                pub_builder = pub_builder.congestion_control(zenoh::qos::CongestionControl::Drop);
                debug!("[PUB] QoS: BestEffort (Drop)");
            }
        }

        let transient_local_replay_task = if let Some(cache) = transient_local_cache.as_ref() {
            Some(
                spawn_transient_local_replay_queryable(
                    &self.session,
                    &topic_key_expr,
                    endpoint_global_id,
                    cache.clone(),
                )
                .await?,
            )
        } else {
            None
        };
        let transient_local_replay_task = ReplayTaskGuard::new(transient_local_replay_task);

        let inner = pub_builder.await?;
        debug!("[PUB] Publisher ready: topic={}", self.entity.topic);

        let liveliness_key_expr =
            ros_z_protocol::format::liveliness_key_expr(&self.entity, &self.session.zid())?;
        let lv_token = self
            .session
            .liveliness()
            .declare_token((*liveliness_key_expr).clone())
            .await?;
        let encoding = Arc::new(crate::encoding::Encoding::cdr().to_zenoh_encoding());
        debug!("[PUB] Using encoding: {}", encoding);

        Ok(Publisher {
            entity: self.entity,
            sequence_number: AtomicUsize::new(0),
            inner,
            _lv_token: lv_token,
            endpoint_global_id,
            clock: self.clock,
            events_mgr: Arc::new(Mutex::new(EventsManager::new(endpoint_global_id))),
            attachment: self.attachment,
            shm_config: self.shm_config,
            dyn_schema: self.dyn_schema,
            encoding,
            graph: self.graph,
            transient_local_cache,
            transient_local_replay_task: transient_local_replay_task.into_task(),
            _phantom_data: Default::default(),
        })
    }
}

impl<T, C> Drop for Publisher<T, C>
where
    C: WireEncoder,
{
    fn drop(&mut self) {
        if let Some(task) = &self.transient_local_replay_task {
            task.abort();
        }
    }
}

impl<T, C> Publisher<T, C>
where
    T: Send + Sync + 'static,
    C: for<'a> WireEncoder<Input<'a> = &'a T> + 'static,
{
    /// Return the number of matched subscribers currently visible in the graph.
    pub fn subscriber_count(&self) -> usize {
        self.graph
            .get_entities_by_topic(EntityKind::Subscription, &self.entity.topic)
            .len()
    }

    /// Return whether at least one subscriber is currently matched.
    pub fn has_subscribers(&self) -> bool {
        self.subscriber_count() > 0
    }

    /// Wait until at least `count` subscribers are matched on this publisher's topic,
    /// or until `timeout` elapses.
    ///
    /// Returns `true` if the required number of subscribers appeared within the
    /// timeout, `false` otherwise.
    ///
    /// This mirrors rclcpp's `rcl_wait_for_subscribers()` pattern: the publisher
    /// registers a graph-change notification *before* sampling the subscriber count,
    /// so no arrival is missed between the check and the wait.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Ensure at least one subscriber is ready before publishing.
    /// assert!(publisher.wait_for_subscribers(1, Duration::from_secs(5)).await);
    /// ```
    pub async fn wait_for_subscribers(&self, count: usize, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            // Arm the notification *before* reading the count to avoid a TOCTOU
            // race where a subscriber arrives between the count check and the await.
            let notified = self.graph.change_notify.notified();
            tokio::pin!(notified);

            let n = self
                .graph
                .get_entities_by_topic(EntityKind::Subscription, &self.entity.topic)
                .len();
            if n >= count {
                return true;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return false;
            }

            // Sleep until either a graph change fires or the deadline passes.
            if tokio::time::timeout(remaining, &mut notified)
                .await
                .is_err()
            {
                // Timeout — do one final check in case a late notification was missed.
                return self
                    .graph
                    .get_entities_by_topic(EntityKind::Subscription, &self.entity.topic)
                    .len()
                    >= count;
            }
        }
    }

    /// Prepare a single publish operation with a reserved publisher-owned id.
    pub fn prepare(&self) -> PreparedPublication<'_, T, C> {
        PreparedPublication {
            publisher: self,
            publication_id: self.next_publication_id(),
        }
    }

    /// Return the next unique publication id for this publisher.
    fn next_publication_id(&self) -> PublicationId {
        let sequence_number = self.sequence_number.fetch_add(1, Ordering::Relaxed) as i64;
        PublicationId::new(self.endpoint_global_id, sequence_number)
    }

    fn new_attachment_for_publication(&self, publication_id: PublicationId) -> Attachment {
        trace!(
            "[PUB] Creating attachment: sequence_number={}, endpoint_global_id={:02x?}",
            publication_id.sequence_number(),
            &publication_id.endpoint_global_id()[..4]
        );
        Attachment::with_clock(
            publication_id.sequence_number(),
            publication_id.endpoint_global_id(),
            &self.clock,
        )
    }

    /// Serialize and publish `message` on the topic.
    #[tracing::instrument(name = "publish", skip(self, message), fields(
        topic = %self.entity.topic,
        sequence_number = self.sequence_number.load(Ordering::Acquire),
        endpoint_global_id = tracing::field::Empty,
        payload_len = tracing::field::Empty,
        used_shm = tracing::field::Empty
    ))]
    pub async fn publish(&self, message: &T) -> Result<()> {
        self.prepare().publish(message).await
    }

    async fn publish_with_reserved_id(
        &self,
        message: &T,
        publication_id: PublicationId,
    ) -> Result<()> {
        let (zbytes, attachment) = self.prepare_publish_payload(message, publication_id)?;
        let mut put_builder = self.inner.put(zbytes.clone());

        put_builder = put_builder.encoding((*self.encoding).clone());

        if let Some(att) = attachment.clone() {
            put_builder = put_builder.attachment(att);
        }

        put_builder.await?;
        self.retain_transient_local_sample(zbytes, attachment);
        Ok(())
    }

    fn prepare_publish_payload(
        &self,
        message: &T,
        publication_id: PublicationId,
    ) -> Result<(zenoh::bytes::ZBytes, Option<Attachment>)> {
        tracing::Span::current().record(
            "endpoint_global_id",
            format_args!("{:02x?}", publication_id.endpoint_global_id()),
        );
        use zenoh_buffers::buffer::Buffer;

        if let Some(schema) = self.dyn_schema.as_ref()
            && let Some(message) =
                (message as &dyn std::any::Any).downcast_ref::<crate::dynamic::DynamicPayload>()
        {
            validate_dynamic_publish_schema(Some(schema), message)?;
        }

        // Try direct SHM serialization if configured
        let (zbuf, actual_size) = if let Some(ref shm_cfg) = self.shm_config {
            let estimated_size = C::serialized_size_hint(message);

            if estimated_size >= shm_cfg.threshold() {
                match C::serialize_to_shm(message, estimated_size, shm_cfg.provider()) {
                    Ok((zbuf, actual_size)) => {
                        tracing::Span::current().record("used_shm", true);
                        (zbuf, actual_size)
                    }
                    Err(_) => {
                        tracing::Span::current().record("used_shm", false);
                        let zbuf = C::serialize_to_zbuf(message);
                        let size = zbuf.len();
                        (zbuf, size)
                    }
                }
            } else {
                tracing::Span::current().record("used_shm", false);
                let zbuf = C::serialize_to_zbuf(message);
                let size = zbuf.len();
                (zbuf, size)
            }
        } else {
            tracing::Span::current().record("used_shm", false);
            let zbuf = C::serialize_to_zbuf(message);
            let size = zbuf.len();
            (zbuf, size)
        };
        tracing::Span::current().record("payload_len", actual_size);

        let zbytes = zenoh::bytes::ZBytes::from(zbuf);
        let attachment = self
            .attachment
            .then(|| self.new_attachment_for_publication(publication_id));

        Ok((zbytes, attachment))
    }

    fn retain_transient_local_sample(
        &self,
        payload: zenoh::bytes::ZBytes,
        attachment: Option<Attachment>,
    ) {
        if let Some(cache) = &self.transient_local_cache {
            cache.retain(RetainedSample {
                payload,
                encoding: Some((*self.encoding).clone()),
                attachment,
            });
        }
    }

    /// Publish a lazily constructed message only when a subscriber is matched.
    pub async fn publish_if_subscribed<F, Fut>(&self, build: F) -> Result<bool>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        if !self.has_subscribers() {
            return Ok(false);
        }

        let message = build().await;
        self.publish(&message).await?;
        Ok(true)
    }
}

impl<T, C: WireEncoder> Publisher<T, C> {
    pub fn events_mgr(&self) -> &Arc<Mutex<EventsManager>> {
        &self.events_mgr
    }

    /// Get a reference to the endpoint entity for this publisher.
    pub fn entity(&self) -> &EndpointEntity {
        &self.entity
    }
}

// Specialized implementation for DynamicStruct publisher
impl Publisher<crate::dynamic::DynamicPayload, crate::dynamic::DynamicCdrCodec> {
    /// Get the dynamic schema used by this publisher.
    ///
    /// Returns `None` if the publisher was not created with `.dyn_schema()`.
    pub fn schema(&self) -> Option<&crate::dynamic::schema::TypeShape> {
        self.dyn_schema.as_ref().map(|s| s.as_ref())
    }
}

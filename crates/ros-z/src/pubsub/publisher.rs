use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::{debug, trace, warn};
use zenoh::Session;
use zenoh::liveliness::LivelinessToken;

use crate::Result;
use crate::attachment::{Attachment, EndpointGlobalId};
use crate::dynamic::{DynamicCdrCodec, DynamicPayload, Schema};
use crate::encoding::Encoding;
use crate::endpoint_builder::{EndpointBuilderContext, MessageEndpointType};
use crate::entity::{EndpointEntity, EndpointKind};
use crate::graph::Graph;
use crate::message::WireEncoder;
use crate::pubsub::metadata::PublicationId;
use crate::pubsub::replay::{self, RetainedSample, TransientLocalCache};
use crate::qos::QosProfile;
use crate::shm::ShmConfig;
use crate::time::Clock;
use crate::topic_name;
use ros_z_protocol::qos::{QosDurability, QosHistory, QosReliability};
use ros_z_schema::SchemaBundle;

pub(super) fn validate_dynamic_publish_schema(
    advertised_schema: Option<&Schema>,
    message: &DynamicPayload,
) -> Result<()> {
    let Some(advertised_schema) = advertised_schema else {
        return Ok(());
    };

    if advertised_schema.as_ref() == message.schema.as_ref() {
        return Ok(());
    }

    Err(crate::error::WireError::DynamicSchemaMismatch.into())
}

pub struct Publisher<T, C: WireEncoder = <T as crate::Message>::Codec> {
    entity: EndpointEntity,
    /// Local monotonically increasing sequence used in publication attachments.
    sequence_number: AtomicUsize,
    /// Stable ros-z endpoint global ID derived from the node Zenoh id and endpoint-local id.
    endpoint_global_id: EndpointGlobalId,
    inner: zenoh::pubsub::Publisher<'static>,
    _lv_token: LivelinessToken,
    clock: Clock,
    shm_config: Option<Arc<ShmConfig>>,
    /// Schema for dynamic message publishing.
    dyn_schema: Option<Schema>,
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

struct PreparedPublisherBuild {
    session: Session,
    graph: Arc<Graph>,
    clock: Clock,
    shm_config: Option<Arc<ShmConfig>>,
    entity: EndpointEntity,
    key_expr: zenoh::key_expr::KeyExpr<'static>,
    topic_key_expr: ros_z_protocol::entity::TopicKE,
    transient_local_cache: Option<Arc<TransientLocalCache>>,
    endpoint_global_id: EndpointGlobalId,
    dyn_schema: Option<Schema>,
}

impl PreparedPublisherBuild {
    fn warn_about_incompatible_endpoints(graph: &Graph, entity: &EndpointEntity) {
        for endpoint in graph.type_incompatible_endpoints_for(entity) {
            warn!(
                topic = %entity.topic,
                publisher_node = %entity.node.fully_qualified_name(),
                publisher_type = %entity.type_info.name,
                publisher_schema_hash = %entity.type_info.hash,
                endpoint_kind = ?endpoint.kind,
                endpoint_node = %endpoint.node.fully_qualified_name(),
                endpoint_type = %endpoint.type_info.name,
                endpoint_schema_hash = %endpoint.type_info.hash,
                "[PUB] endpoint type metadata does not match publisher"
            );
        }
    }
}

async fn spawn_transient_local_replay_queryable(
    session: &Session,
    topic_key_expr: &ros_z_protocol::entity::TopicKE,
    endpoint_global_id: EndpointGlobalId,
    cache: Arc<TransientLocalCache>,
) -> Result<JoinHandle<()>> {
    let replay_key = replay::transient_local_replay_key(topic_key_expr, endpoint_global_id);
    let reply_key_expr = (**topic_key_expr).clone();
    let queryable = session
        .declare_queryable(replay_key)
        .complete(true)
        .await
        .map_err(|source| {
            crate::Error::zenoh("declare transient-local replay queryable", source)
        })?;
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
                reply = reply.attachment(sample.attachment);
                if let Err(err) = reply.await.map_err(|source| {
                    crate::Error::zenoh("reply transient-local replay sample", source)
                }) {
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
    pub(crate) context: EndpointBuilderContext,
    pub(crate) topic: String,
    pub(crate) type_source: MessageEndpointType,
    pub(crate) qos: ros_z_protocol::qos::QosProfile,
    pub(crate) shm_config: Option<Arc<ShmConfig>>,
    pub(crate) _phantom_data: PhantomData<(T, C)>,
}

impl<T, C> PublisherBuilder<T, C> {
    pub(crate) fn new(
        context: EndpointBuilderContext,
        topic: String,
        type_source: MessageEndpointType,
    ) -> Self {
        let shm_config = context.shm_config.clone();
        Self {
            context,
            topic,
            type_source,
            qos: crate::endpoint_builder::default_protocol_qos(),
            shm_config,
            _phantom_data: Default::default(),
        }
    }

    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.qos = qos.to_protocol_qos();
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
    /// # async fn main() -> ros_z::Result<()> {
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
    pub fn shm_config(mut self, config: ShmConfig) -> Self {
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
    /// # async fn main() -> ros_z::Result<()> {
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
}

impl<T, C> PublisherBuilder<T, C>
where
    T: Send + Sync + 'static,
    C: for<'a> WireEncoder<Input<'a> = &'a T> + 'static,
{
    #[tracing::instrument(name = "pub_build", skip(self), fields(
        topic = %self.topic
    ))]
    pub async fn build(self) -> Result<Publisher<T, C>> {
        self.build_inner_async().await
    }

    fn prepare_build(self) -> Result<PreparedPublisherBuild> {
        let (type_info, dyn_schema) = self
            .type_source
            .resolve_for_publisher(&self.context, &self.topic)?;

        // Qualify the topic name as a ros-z graph name.
        let topic = self.topic;
        let qualified_topic = topic_name::qualify_topic_name(
            &topic,
            &self.context.node.namespace,
            &self.context.node.name,
        )
        .map_err(|source| crate::Error::topic_name(topic, source))?;

        debug!("[PUB] Qualified topic: {}", qualified_topic);

        let entity = self.context.endpoint_entity(
            EndpointKind::Publisher,
            qualified_topic,
            type_info,
            self.qos,
        );
        let topic_key_expr = ros_z_protocol::format::topic_key_expr(&entity)?;
        let key_expr = (*topic_key_expr).clone();
        debug!("[PUB] Key expression: {}", key_expr);

        if matches!(
            entity.qos,
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

        let transient_local_cache = replay::transient_local_cache_capacity(&entity.qos)
            .map(|capacity| Arc::new(TransientLocalCache::new(capacity)));
        let endpoint_global_id = EndpointGlobalId::from(&entity);

        Ok(PreparedPublisherBuild {
            session: self.context.session.clone(),
            graph: self.context.graph.clone(),
            clock: self.context.clock.clone(),
            shm_config: self.shm_config,
            entity,
            key_expr,
            topic_key_expr,
            transient_local_cache,
            endpoint_global_id,
            dyn_schema,
        })
    }

    async fn build_inner_async(self) -> Result<Publisher<T, C>> {
        let prepared = self.prepare_build()?;

        let mut pub_builder = prepared.session.declare_publisher(prepared.key_expr);

        match prepared.entity.qos.reliability {
            QosReliability::Reliable => {
                pub_builder = pub_builder.congestion_control(zenoh::qos::CongestionControl::Block);
                debug!("[PUB] QoS: Reliable (Block)");
            }
            QosReliability::BestEffort => {
                pub_builder = pub_builder.congestion_control(zenoh::qos::CongestionControl::Drop);
                debug!("[PUB] QoS: BestEffort (Drop)");
            }
        }

        let transient_local_replay_task =
            if let Some(cache) = prepared.transient_local_cache.as_ref() {
                Some(
                    spawn_transient_local_replay_queryable(
                        &prepared.session,
                        &prepared.topic_key_expr,
                        prepared.endpoint_global_id,
                        cache.clone(),
                    )
                    .await?,
                )
            } else {
                None
            };
        let transient_local_replay_task = ReplayTaskGuard::new(transient_local_replay_task);

        let inner = pub_builder
            .await
            .map_err(|source| crate::Error::zenoh("declare publisher", source))?;
        debug!("[PUB] Publisher ready: topic={}", prepared.entity.topic);

        let liveliness_key_expr = prepared.entity.liveliness_key_expr()?.0;
        let lv_token = prepared
            .session
            .liveliness()
            .declare_token(liveliness_key_expr)
            .await
            .map_err(|source| crate::Error::zenoh("declare publisher liveliness token", source))?;
        let encoding = Arc::new(Encoding::cdr().to_zenoh_encoding());
        debug!("[PUB] Using encoding: {}", encoding);
        PreparedPublisherBuild::warn_about_incompatible_endpoints(
            &prepared.graph,
            &prepared.entity,
        );

        Ok(Publisher {
            entity: prepared.entity,
            sequence_number: AtomicUsize::new(0),
            inner,
            _lv_token: lv_token,
            endpoint_global_id: prepared.endpoint_global_id,
            clock: prepared.clock,
            shm_config: prepared.shm_config,
            dyn_schema: prepared.dyn_schema,
            encoding,
            graph: prepared.graph,
            transient_local_cache: prepared.transient_local_cache,
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
        self.graph.view().subscriptions_on(&self.entity.topic).len()
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

            let n = self.graph.view().subscriptions_on(&self.entity.topic).len();
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
                return self.graph.view().subscriptions_on(&self.entity.topic).len() >= count;
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
            &publication_id.endpoint_global_id().as_bytes()[..4]
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

        put_builder = put_builder.attachment(attachment.clone());

        put_builder
            .await
            .map_err(|source| crate::Error::zenoh("publish sample", source))?;
        self.retain_transient_local_sample(zbytes, attachment);
        Ok(())
    }

    fn prepare_publish_payload(
        &self,
        message: &T,
        publication_id: PublicationId,
    ) -> Result<(zenoh::bytes::ZBytes, Attachment)> {
        tracing::Span::current().record(
            "endpoint_global_id",
            format_args!("{:02x?}", publication_id.endpoint_global_id().as_bytes()),
        );
        use zenoh_buffers::buffer::Buffer;

        if let Some(schema) = self.dyn_schema.as_ref()
            && let Some(message) = (message as &dyn std::any::Any).downcast_ref::<DynamicPayload>()
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
                    Err(error) => {
                        warn!(error = %error, "SHM serialization failed; falling back to heap payload");
                        tracing::Span::current().record("used_shm", false);
                        let zbuf = C::serialize_to_zbuf(message).map_err(|source| {
                            crate::Error::encode(std::any::type_name::<T>(), source)
                        })?;
                        let size = zbuf.len();
                        (zbuf, size)
                    }
                }
            } else {
                tracing::Span::current().record("used_shm", false);
                let zbuf = C::serialize_to_zbuf(message)
                    .map_err(|source| crate::Error::encode(std::any::type_name::<T>(), source))?;
                let size = zbuf.len();
                (zbuf, size)
            }
        } else {
            tracing::Span::current().record("used_shm", false);
            let zbuf = C::serialize_to_zbuf(message)
                .map_err(|source| crate::Error::encode(std::any::type_name::<T>(), source))?;
            let size = zbuf.len();
            (zbuf, size)
        };
        tracing::Span::current().record("payload_len", actual_size);

        let zbytes = zenoh::bytes::ZBytes::from(zbuf);
        let attachment = self.new_attachment_for_publication(publication_id);

        Ok((zbytes, attachment))
    }

    fn retain_transient_local_sample(&self, payload: zenoh::bytes::ZBytes, attachment: Attachment) {
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
    /// Get a reference to the endpoint entity for this publisher.
    pub fn entity(&self) -> &EndpointEntity {
        &self.entity
    }
}

// Specialized implementation for DynamicPayload publisher
impl Publisher<DynamicPayload, DynamicCdrCodec> {
    /// Get the dynamic schema used by this publisher.
    ///
    /// Returns `None` if the publisher was not created through
    /// `Node::dynamic_publisher`.
    pub fn schema(&self) -> Option<&SchemaBundle> {
        self.dyn_schema.as_ref().map(|s| s.as_ref())
    }
}

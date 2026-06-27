use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, atomic::AtomicUsize},
    time::Duration,
};

use tracing::{debug, info, trace, warn};
use zenoh::{
    Wait, bytes, key_expr::KeyExpr, liveliness::LivelinessToken, query::Query, sample::Sample,
};

use std::sync::atomic::Ordering;

use crate::{Error, Result, error::WireError, topic_name};

use crate::{
    attachment::{Attachment, EndpointGlobalId},
    endpoint_builder::{EndpointBuilderContext, ServiceEndpointType},
    entity::{EndpointEntity, EndpointKind},
    message::{Message, Service, WireDecoder, WireEncoder},
    qos::QosProfile,
    queue::BoundedQueue,
    time::Clock,
};

#[derive(Debug)]
pub struct ServiceClientBuilder<T> {
    pub(crate) context: EndpointBuilderContext,
    pub(crate) name: String,
    pub(crate) type_source: ServiceEndpointType,
    pub(crate) qos: ros_z_protocol::qos::QosProfile,
    pub(crate) _phantom_data: PhantomData<T>,
}

/// A native ros-z reusable service handle for typed request/response calls.
///
/// Create a client via
/// [`Node::service_client`](crate::node::Node::service_client).
/// Invoke the service with [`call`](ServiceClient::call) for blocking code or
/// [`call_async`](ServiceClient::call_async) for async code.
///
/// # Example
///
/// ```rust,ignore
/// use ros_z::prelude::*;
/// use std::time::Duration;
///
/// // client: ServiceClient<MyService>
/// let response = client
///     .call_with_timeout_async(&request, Duration::from_secs(5))
///     .await?;
/// ```
pub struct ServiceClient<T: Service> {
    /// Local monotonically increasing sequence used in request attachments.
    sequence_number: AtomicUsize,
    /// Stable ros-z endpoint global ID derived from the node Zenoh id and endpoint-local id.
    endpoint_global_id: EndpointGlobalId,
    inner: zenoh::query::Querier<'static>,
    _lv_token: LivelinessToken,
    topic: String,
    clock: Clock,
    _phantom_data: PhantomData<T>,
}

impl<T: Service> std::fmt::Debug for ServiceClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceClient")
            .field("topic", &self.topic)
            .finish_non_exhaustive()
    }
}

impl<T> ServiceClientBuilder<T>
where
    T: Service,
{
    #[tracing::instrument(name = "client_build", skip(self), fields(
        service = %self.name
    ))]
    pub async fn build(self) -> Result<ServiceClient<T>> {
        let entity = self.prepare_entity()?;
        let topic_key_expr = ros_z_protocol::format::topic_key_expr(&entity)?;
        let key_expr = (*topic_key_expr).clone();
        debug!("[CLN] Key expression: {}", key_expr);

        let inner = self
            .context
            .session
            .declare_querier(key_expr)
            .target(zenoh::query::QueryTarget::AllComplete)
            .consolidation(zenoh::query::ConsolidationMode::None)
            .await
            .map_err(|source| crate::Error::zenoh("declare service querier", source))?;
        let liveliness_key_expr = entity.liveliness_key_expr()?.0;
        let lv_token = self
            .context
            .session
            .liveliness()
            .declare_token(liveliness_key_expr)
            .await
            .map_err(|source| {
                crate::Error::zenoh("declare service client liveliness token", source)
            })?;
        self.warn_about_incompatible_endpoints(&entity);
        debug!("[CLN] Client ready: service={}", entity.topic);

        Ok(ServiceClient {
            sequence_number: AtomicUsize::new(1), // Start at 1; zero is reserved for missing sequence values.
            inner,
            _lv_token: lv_token,
            endpoint_global_id: EndpointGlobalId::from(&entity),
            topic: entity.topic,
            clock: self.context.clock,
            _phantom_data: Default::default(),
        })
    }

    fn warn_about_incompatible_endpoints(&self, entity: &EndpointEntity) -> usize {
        let endpoints = self.context.graph.type_incompatible_endpoints_for(entity);
        let count = endpoints.len();
        for endpoint in endpoints {
            warn!(
                service = %entity.topic,
                client_node = %entity.node.fully_qualified_name(),
                client_type = %entity.type_info.name,
                client_schema_hash = %entity.type_info.hash,
                endpoint_kind = ?endpoint.kind,
                endpoint_node = %endpoint.node.fully_qualified_name(),
                endpoint_type = %endpoint.type_info.name,
                endpoint_schema_hash = %endpoint.type_info.hash,
                "[CLN] endpoint type metadata does not match service client"
            );
        }
        count
    }
}

impl<T> ServiceClient<T>
where
    T: Service,
{
    fn timeout_error(&self, timeout: Duration) -> crate::Error {
        crate::error::ServiceCallError::Timeout {
            service: self.topic.clone(),
            timeout,
        }
        .into()
    }

    fn new_attachment(&self) -> Attachment {
        Attachment::with_clock(
            self.sequence_number.fetch_add(1, Ordering::AcqRel) as _,
            self.endpoint_global_id,
            &self.clock,
        )
    }

    fn call_sample_blocking(
        &self,
        payload: impl Into<bytes::ZBytes>,
        timeout: Option<Duration>,
    ) -> Result<Sample> {
        let attachment = self.new_attachment();
        let payload = payload.into();
        let (response_tx, response_rx) =
            std::sync::mpsc::sync_channel::<std::result::Result<Sample, zenoh::Error>>(1);
        let response_tx = Arc::new(Mutex::new(Some(response_tx)));

        let callback = move |reply: zenoh::query::Reply| {
            let sender = response_tx
                .lock()
                .expect("service reply sender mutex poisoned")
                .take();
            match sender {
                Some(sender) => {
                    let reply = reply
                        .into_result()
                        .map_err(|source| Box::new(source) as zenoh::Error);
                    if sender.send(reply).is_err() {
                        tracing::warn!("Service call receiver dropped before reply delivery");
                    }
                }
                None => {
                    tracing::warn!("Service call received extra reply after completion");
                }
            };
        };

        self.inner
            .get()
            .payload(payload)
            .attachment(attachment)
            .callback(callback)
            .wait()
            .map_err(|source| crate::Error::zenoh("query service", source))?;

        let reply = match timeout {
            Some(timeout) => match response_rx.recv_timeout(timeout) {
                Ok(reply) => reply,
                Err(_) => return Err(self.timeout_error(timeout)),
            },
            None => response_rx.recv().map_err(|_| {
                crate::Error::from(crate::error::ServiceCallError::NoResponse {
                    service: self.topic.clone(),
                })
            })?,
        };

        reply.map_err(|source| {
            crate::error::ServiceCallError::Reply {
                service: self.topic.clone(),
                source,
            }
            .into()
        })
    }

    async fn call_sample_async(&self, payload: impl Into<bytes::ZBytes>) -> Result<Sample> {
        let attachment = self.new_attachment();
        let payload = payload.into();
        let (response_tx, response_rx) =
            tokio::sync::oneshot::channel::<std::result::Result<Sample, zenoh::Error>>();
        let response_tx = Arc::new(Mutex::new(Some(response_tx)));

        let callback = move |reply: zenoh::query::Reply| {
            let sender = response_tx
                .lock()
                .expect("service reply sender mutex poisoned")
                .take();
            match sender {
                Some(sender) => {
                    let reply = reply
                        .into_result()
                        .map_err(|source| Box::new(source) as zenoh::Error);
                    if sender.send(reply).is_err() {
                        tracing::warn!("Service call receiver dropped before reply delivery");
                    }
                }
                None => {
                    tracing::warn!("Service call received extra reply after completion");
                }
            };
        };

        self.inner
            .get()
            .payload(payload)
            .attachment(attachment)
            .callback(callback)
            .await
            .map_err(|source| crate::Error::zenoh("query service", source))?;

        let reply = response_rx
            .await
            .map_err(|_| crate::error::ServiceCallError::NoResponse {
                service: self.topic.clone(),
            })?;

        reply.map_err(|source| {
            crate::error::ServiceCallError::Reply {
                service: self.topic.clone(),
                source,
            }
            .into()
        })
    }

    fn decode_response(&self, sample: Sample) -> Result<T::Response>
    where
        for<'a> <T::Response as Message>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let payload_bytes = sample.payload().to_bytes();
        <<T::Response as Message>::Codec as WireDecoder>::deserialize(&payload_bytes[..])
            .map_err(|source| crate::Error::decode(<T::Response as Message>::type_name(), source))
    }

    /// Call the service and wait indefinitely for the first reply.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`call_async`](Self::call_async) there.
    pub fn call(&self, message: &T::Request) -> Result<T::Response>
    where
        for<'a> <T::Request as Message>::Codec: WireEncoder<Input<'a> = &'a T::Request>,
        for<'a> <T::Response as Message>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let payload = <<T::Request as Message>::Codec as WireEncoder>::serialize(message)
            .map_err(|source| crate::Error::encode(<T::Request as Message>::type_name(), source))?;
        let sample = self.call_sample_blocking(payload, None)?;
        self.decode_response(sample)
    }

    /// Call the service and wait indefinitely for the first reply.
    pub async fn call_async(&self, message: &T::Request) -> Result<T::Response>
    where
        for<'a> <T::Request as Message>::Codec: WireEncoder<Input<'a> = &'a T::Request>,
        for<'a> <T::Response as Message>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let payload = <<T::Request as Message>::Codec as WireEncoder>::serialize(message)
            .map_err(|source| crate::Error::encode(<T::Request as Message>::type_name(), source))?;
        let sample = self.call_sample_async(payload).await?;
        self.decode_response(sample)
    }

    /// Call the service and fail if no reply arrives before `timeout` elapses.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`call_with_timeout_async`](Self::call_with_timeout_async) there.
    pub fn call_with_timeout(&self, message: &T::Request, timeout: Duration) -> Result<T::Response>
    where
        for<'a> <T::Request as Message>::Codec: WireEncoder<Input<'a> = &'a T::Request>,
        for<'a> <T::Response as Message>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let payload = <<T::Request as Message>::Codec as WireEncoder>::serialize(message)
            .map_err(|source| crate::Error::encode(<T::Request as Message>::type_name(), source))?;
        let sample = self.call_sample_blocking(payload, Some(timeout))?;
        self.decode_response(sample)
    }

    /// Call the service and fail if no reply arrives before `timeout` elapses.
    pub async fn call_with_timeout_async(
        &self,
        message: &T::Request,
        timeout: Duration,
    ) -> Result<T::Response>
    where
        for<'a> <T::Request as Message>::Codec: WireEncoder<Input<'a> = &'a T::Request>,
        for<'a> <T::Response as Message>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let payload = <<T::Request as Message>::Codec as WireEncoder>::serialize(message)
            .map_err(|source| crate::Error::encode(<T::Request as Message>::type_name(), source))?;
        let sample_result = tokio::time::timeout(timeout, self.call_sample_async(payload)).await;

        let sample = match sample_result {
            Ok(Ok(sample)) => sample,
            Ok(Err(crate::Error::ServiceCall(crate::error::ServiceCallError::NoResponse {
                ..
            }))) => return Err(self.timeout_error(timeout)),
            Ok(Err(error)) => return Err(error),
            Err(_) => return Err(self.timeout_error(timeout)),
        };

        self.decode_response(sample)
    }
}

#[derive(Debug)]
pub struct ServiceServerBuilder<T> {
    pub(crate) context: EndpointBuilderContext,
    pub(crate) name: String,
    pub(crate) type_source: ServiceEndpointType,
    pub(crate) qos: ros_z_protocol::qos::QosProfile,
    pub(crate) _phantom_data: PhantomData<T>,
}

fn prepare_service_endpoint(
    context: &EndpointBuilderContext,
    name: &str,
    type_source: &ServiceEndpointType,
    kind: EndpointKind,
    qos: ros_z_protocol::qos::QosProfile,
    log_prefix: &str,
) -> Result<EndpointEntity> {
    let type_info = type_source.resolve();
    let qualified_service =
        topic_name::qualify_service_name(name, &context.node.namespace, &context.node.name)
            .map_err(|source| crate::Error::service_name(name, source))?;

    debug!("[{}] Qualified service: {}", log_prefix, qualified_service);

    Ok(context.endpoint_entity(kind, qualified_service, type_info, qos))
}

impl<T> ServiceClientBuilder<T> {
    pub(crate) fn new(
        context: crate::endpoint_builder::EndpointBuilderContext,
        name: String,
        type_source: crate::endpoint_builder::ServiceEndpointType,
    ) -> Self {
        Self {
            context,
            name,
            type_source,
            qos: crate::endpoint_builder::default_protocol_qos(),
            _phantom_data: Default::default(),
        }
    }

    fn prepare_entity(&self) -> Result<EndpointEntity> {
        prepare_service_endpoint(
            &self.context,
            &self.name,
            &self.type_source,
            EndpointKind::Client,
            self.qos,
            "CLN",
        )
    }

    /// Set the QoS profile for this client.
    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.qos = qos.to_protocol_qos();
        self
    }
}

impl<T> ServiceServerBuilder<T> {
    pub(crate) fn new(
        context: crate::endpoint_builder::EndpointBuilderContext,
        name: String,
        type_source: crate::endpoint_builder::ServiceEndpointType,
    ) -> Self {
        Self {
            context,
            name,
            type_source,
            qos: crate::endpoint_builder::default_protocol_qos(),
            _phantom_data: Default::default(),
        }
    }

    fn prepare_entity(&self) -> Result<EndpointEntity> {
        prepare_service_endpoint(
            &self.context,
            &self.name,
            &self.type_source,
            EndpointKind::Service,
            self.qos,
            "SRV",
        )
    }

    /// Set the QoS profile for this server.
    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.qos = qos.to_protocol_qos();
        self
    }
}

pub struct ServiceServer<T: Service, Q = Query> {
    key_expr: KeyExpr<'static>,
    _inner: zenoh::query::Queryable<()>,
    _lv_token: LivelinessToken,
    clock: Clock,
    pub(crate) queue: Option<Arc<BoundedQueue<Q>>>,
    _phantom_data: PhantomData<T>,
}

enum ServiceQueryHandler {
    Queue(Arc<BoundedQueue<Query>>),
    Callback(Arc<dyn Fn(Query) + Send + Sync>),
}

impl ServiceQueryHandler {
    fn handle(&self, query: Query) {
        match self {
            ServiceQueryHandler::Queue(queue) => {
                if queue.push(query) {
                    tracing::debug!("Queue full, dropped oldest service request");
                }
            }
            ServiceQueryHandler::Callback(callback) => callback(query),
        }
    }
}

impl<T: Service, Q> std::fmt::Debug for ServiceServer<T, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceServer")
            .field("key_expr", &self.key_expr.as_str())
            .finish_non_exhaustive()
    }
}

impl<T, Q> ServiceServer<T, Q>
where
    T: Service,
{
    /// Access the receiver queue.
    ///
    /// # Panics
    ///
    /// Panics if the server was built with `build_with_callback()` and has no queue.
    /// Action servers always have queues and will never panic.
    pub fn queue(&self) -> &Arc<BoundedQueue<Q>> {
        self.queue
            .as_ref()
            .expect("Server was built with callback mode, no queue available")
    }

    /// Access the receiver queue if present (returns `None` in callback mode).
    pub fn try_queue(&self) -> Option<&Arc<BoundedQueue<Q>>> {
        self.queue.as_ref()
    }
}

impl<T> ServiceServerBuilder<T>
where
    T: Service,
{
    /// Internal method that all build variants use.
    async fn build_internal<Q>(
        self,
        handler: ServiceQueryHandler,
        queue: Option<Arc<BoundedQueue<Q>>>,
    ) -> Result<ServiceServer<T, Q>> {
        let entity = self.prepare_entity()?;
        let topic_key_expr = ros_z_protocol::format::topic_key_expr(&entity)?;
        let key_expr = (*topic_key_expr).clone();
        tracing::debug!("[SRV] KE: {key_expr}");

        info!("[SRV] Declaring queryable on key expression: {}", key_expr);

        let inner = self
            .context
            .session
            .declare_queryable(&key_expr)
            .complete(true)
            .callback(move |query| {
                trace!(
                    "[SRV] Query received: key_expr={}, selector={}, parameters={}",
                    query.key_expr(),
                    query.selector(),
                    query.parameters()
                );

                if let Some(att) = query.attachment() {
                    trace!("[SRV] Query has attachment: {} bytes", att.len());
                } else {
                    trace!("[SRV] Query has NO attachment");
                }

                if let Some(payload) = query.payload() {
                    trace!("[SRV] Query has payload: {} bytes", payload.len());
                } else {
                    trace!("[SRV] Query has NO payload");
                }

                handler.handle(query);
            })
            .await
            .map_err(|source| crate::Error::zenoh("declare service queryable", source))?;

        let liveliness_key_expr = entity.liveliness_key_expr()?.0;
        let lv_token = self
            .context
            .session
            .liveliness()
            .declare_token(liveliness_key_expr)
            .await
            .map_err(|source| {
                crate::Error::zenoh("declare service server liveliness token", source)
            })?;
        self.warn_about_incompatible_endpoints(&entity);

        Ok(ServiceServer {
            key_expr,
            _inner: inner,
            _lv_token: lv_token,
            clock: self.context.clock,
            queue,
            _phantom_data: Default::default(),
        })
    }

    pub async fn build_with_callback<F>(self, callback: F) -> Result<ServiceServer<T, ()>>
    where
        F: Fn(Query) + Send + Sync + 'static,
    {
        self.build_internal(ServiceQueryHandler::Callback(Arc::new(callback)), None)
            .await
    }

    pub async fn build(self) -> Result<ServiceServer<T>> {
        let queue_size = match self.qos.history {
            ros_z_protocol::qos::QosHistory::KeepLast(depth) => depth,
            ros_z_protocol::qos::QosHistory::KeepAll => usize::MAX,
        };
        let queue = Arc::new(BoundedQueue::new(queue_size));
        self.build_internal(ServiceQueryHandler::Queue(queue.clone()), Some(queue))
            .await
    }

    fn warn_about_incompatible_endpoints(&self, entity: &EndpointEntity) -> usize {
        let endpoints = self.context.graph.type_incompatible_endpoints_for(entity);
        let count = endpoints.len();
        for endpoint in endpoints {
            warn!(
                service = %entity.topic,
                server_node = %entity.node.fully_qualified_name(),
                server_type = %entity.type_info.name,
                server_schema_hash = %entity.type_info.hash,
                endpoint_kind = ?endpoint.kind,
                endpoint_node = %endpoint.node.fully_qualified_name(),
                endpoint_type = %endpoint.type_info.name,
                endpoint_schema_hash = %endpoint.type_info.hash,
                "[SRV] endpoint type metadata does not match service server"
            );
        }
        count
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RequestId {
    pub sequence_number: i64,
    pub writer_global_id: EndpointGlobalId,
}

impl From<Attachment> for RequestId {
    fn from(value: Attachment) -> Self {
        Self {
            sequence_number: value.sequence_number,
            writer_global_id: value.source_global_id,
        }
    }
}

pub struct ServiceReply<T: Service> {
    request_id: RequestId,
    key_expr: KeyExpr<'static>,
    query: Query,
    clock: Clock,
    _phantom_data: PhantomData<T>,
}

impl<T: Service> ServiceReply<T> {
    pub fn id(&self) -> &RequestId {
        &self.request_id
    }

    pub fn reply(self, message: &T::Response) -> Result<()>
    where
        for<'a> <T::Response as Message>::Codec: WireEncoder<Input<'a> = &'a T::Response>,
    {
        let payload = <<T::Response as Message>::Codec as WireEncoder>::serialize(message)
            .map_err(|source| {
                crate::Error::encode(<T::Response as Message>::type_name(), source)
            })?;
        let mut reply = self.query.reply(&self.key_expr, payload);
        let attachment = Attachment::with_clock(
            self.request_id.sequence_number,
            self.request_id.writer_global_id,
            &self.clock,
        );
        reply = reply.attachment(attachment);
        reply
            .wait()
            .map_err(|source| crate::Error::zenoh("send service reply", source))
    }

    pub async fn reply_async(self, message: &T::Response) -> Result<()>
    where
        for<'a> <T::Response as Message>::Codec: WireEncoder<Input<'a> = &'a T::Response>,
    {
        let payload = <<T::Response as Message>::Codec as WireEncoder>::serialize(message)
            .map_err(|source| {
                crate::Error::encode(<T::Response as Message>::type_name(), source)
            })?;
        let mut reply = self.query.reply(&self.key_expr, payload);
        let attachment = Attachment::with_clock(
            self.request_id.sequence_number,
            self.request_id.writer_global_id,
            &self.clock,
        );
        reply = reply.attachment(attachment);
        reply
            .await
            .map_err(|source| crate::Error::zenoh("send service reply", source))
    }
}

pub struct ServiceRequest<T: Service> {
    message: T::Request,
    reply: ServiceReply<T>,
}

impl<T: Service> ServiceRequest<T> {
    pub fn id(&self) -> &RequestId {
        self.reply.id()
    }

    pub fn message(&self) -> &T::Request {
        &self.message
    }

    pub fn into_message(self) -> T::Request {
        self.message
    }

    pub fn into_parts(self) -> (T::Request, ServiceReply<T>) {
        (self.message, self.reply)
    }

    pub fn reply(self, response: &T::Response) -> Result<()>
    where
        for<'a> <T::Response as Message>::Codec: WireEncoder<Input<'a> = &'a T::Response>,
    {
        self.reply.reply(response)
    }

    pub async fn reply_async(self, response: &T::Response) -> Result<()>
    where
        for<'a> <T::Response as Message>::Codec: WireEncoder<Input<'a> = &'a T::Response>,
    {
        self.reply.reply_async(response).await
    }
}

impl<T> ServiceServer<T, Query>
where
    T: Service,
{
    fn decode_request(&self, query: Query) -> Result<ServiceRequest<T>>
    where
        for<'a> <T::Request as Message>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        let attachment = {
            let raw = query
                .attachment()
                .ok_or(WireError::MissingServiceRequestAttachment)?;

            Attachment::try_from(raw).map_err(|source| {
                Error::from(WireError::ServiceRequestAttachmentDecode { source })
            })?
        };
        let request_id = RequestId::from(attachment);

        let payload_bytes = query
            .payload()
            .map(|payload| payload.to_bytes())
            .unwrap_or_default();
        let message =
            <<T::Request as Message>::Codec as WireDecoder>::deserialize(&payload_bytes[..])
                .map_err(|source| {
                    crate::Error::decode(<T::Request as Message>::type_name(), source)
                })?;

        Ok(ServiceRequest {
            message,
            reply: ServiceReply {
                request_id,
                key_expr: self.key_expr.clone(),
                query,
                clock: self.clock.clone(),
                _phantom_data: PhantomData,
            },
        })
    }

    pub fn try_take_request(&mut self) -> Result<Option<ServiceRequest<T>>>
    where
        for<'a> <T::Request as Message>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        let queue = self.queue.as_ref().ok_or_else(|| {
            crate::Error::service_server_state(
                "access service request queue",
                "server was built with callback, no queue available",
            )
        })?;
        match queue.try_recv() {
            Some(query) => self.decode_request(query).map(Some),
            None => Ok(None),
        }
    }

    /// Blocks waiting to receive the next request on the service and then deserializes the payload.
    ///
    /// This method may fail if the message does not deserialize as the requested type.
    #[tracing::instrument(name = "take_request", skip(self), fields(
        service = %self.key_expr,
        sequence_number = tracing::field::Empty,
        payload_len = tracing::field::Empty
    ))]
    pub fn take_request(&mut self) -> Result<ServiceRequest<T>>
    where
        for<'a> <T::Request as Message>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        trace!("[SRV] Waiting for request");

        let queue = self.queue.as_ref().ok_or_else(|| {
            crate::Error::service_server_state(
                "access service request queue",
                "server was built with callback, no queue available",
            )
        })?;
        let query = queue.recv();
        self.decode_request(query)
    }

    /// Awaits the next request on the service and then deserializes the payload.
    ///
    /// This method may fail if the message does not deserialize as the requested type.
    pub async fn take_request_async(&mut self) -> Result<ServiceRequest<T>>
    where
        for<'a> <T::Request as Message>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        let queue = self.queue.as_ref().ok_or_else(|| {
            crate::Error::service_server_state(
                "access service request queue",
                "server was built with callback, no queue available",
            )
        })?;
        let query = queue.recv_async().await;
        self.decode_request(query)
    }
}

impl<T> ServiceServer<T, ()>
where
    T: Service,
{
    pub fn try_take_request(&mut self) -> Result<Option<ServiceRequest<T>>> {
        Err(crate::Error::service_server_state(
            "access service request queue",
            "server was built with callback, no queue available",
        ))
    }

    /// Blocks waiting to receive the next request on the service and then deserializes the payload.
    ///
    /// Callback-mode service servers do not expose a request queue.
    pub fn take_request(&mut self) -> Result<ServiceRequest<T>> {
        Err(crate::Error::service_server_state(
            "access service request queue",
            "server was built with callback, no queue available",
        ))
    }

    /// Awaits the next request on the service and then deserializes the payload.
    ///
    /// Callback-mode service servers do not expose a request queue.
    pub async fn take_request_async(&mut self) -> Result<ServiceRequest<T>> {
        Err(crate::Error::service_server_state(
            "access service request queue",
            "server was built with callback, no queue available",
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        Message, SerdeCdrCodec, ServiceTypeInfo,
        context::ContextBuilder,
        entity::{Entity, TypeInfo},
        message::{Service, WireEncoder},
    };
    use ros_z_schema::ServiceDef;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, crate::Message)]
    #[message(name = "test_msgs::AddTwoIntsRequest")]
    struct AddTwoIntsRequest {
        a: i64,
        b: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, crate::Message)]
    #[message(name = "test_msgs::AddTwoIntsResponse")]
    struct AddTwoIntsResponse {
        sum: i64,
    }

    struct AddTwoInts;

    impl Service for AddTwoInts {
        type Request = AddTwoIntsRequest;
        type Response = AddTwoIntsResponse;
    }

    impl ServiceTypeInfo for AddTwoInts {
        fn service_type_info() -> TypeInfo {
            let descriptor = ServiceDef::new(
                "test_msgs::AddTwoInts",
                AddTwoIntsRequest::type_name(),
                AddTwoIntsResponse::type_name(),
            )
            .expect("test service descriptor should be static and valid");
            let hash = ros_z_schema::compute_hash(&descriptor)
                .expect("test service hash should be static and valid");
            TypeInfo::new(descriptor.type_name.as_str(), hash)
        }
    }

    struct MultiplyTwoInts;

    impl Service for MultiplyTwoInts {
        type Request = AddTwoIntsRequest;
        type Response = AddTwoIntsResponse;
    }

    impl ServiceTypeInfo for MultiplyTwoInts {
        fn service_type_info() -> TypeInfo {
            let descriptor = ServiceDef::new(
                "test_msgs::MultiplyTwoInts",
                AddTwoIntsRequest::type_name(),
                AddTwoIntsResponse::type_name(),
            )
            .expect("test service descriptor should be static and valid");
            let hash = ros_z_schema::compute_hash(&descriptor)
                .expect("test service hash should be static and valid");
            TypeInfo::new(descriptor.type_name.as_str(), hash)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn service_builder_warning_helpers_report_same_name_type_mismatches() {
        let context = ContextBuilder::default()
            .disable_multicast_scouting()
            .with_json("connect/endpoints", json!([]))
            .build()
            .await
            .expect("Failed to create context");
        let node = context
            .create_node("service_warning_helpers")
            .without_schema_service()
            .build()
            .await
            .expect("Failed to create node");

        let server_builder = node.service_server::<AddTwoInts>("/service_warning_helpers");
        let client_builder = node.service_client::<MultiplyTwoInts>("/service_warning_helpers");
        let server_entity = server_builder
            .prepare_entity()
            .expect("service server entity should prepare");
        let client_entity = client_builder
            .prepare_entity()
            .expect("service client entity should prepare");

        server_builder
            .context
            .graph
            .add_local_entity(Entity::Endpoint(client_entity.clone()))
            .expect("client endpoint should be inserted");
        client_builder
            .context
            .graph
            .add_local_entity(Entity::Endpoint(server_entity.clone()))
            .expect("server endpoint should be inserted");

        assert_eq!(
            server_builder.warn_about_incompatible_endpoints(&server_entity),
            1
        );
        assert_eq!(
            client_builder.warn_about_incompatible_endpoints(&client_entity),
            1
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn service_request_without_attachment_returns_clear_error() {
        let context = ContextBuilder::default()
            .disable_multicast_scouting()
            .with_json("connect/endpoints", json!([]))
            .build()
            .await
            .expect("Failed to create context");

        let server_node = context
            .create_node("raw_query_server")
            .build()
            .await
            .expect("Failed to create server node");
        let client_node = context
            .create_node("raw_query_client")
            .build()
            .await
            .expect("Failed to create client node");

        let mut server = server_node
            .service_server::<AddTwoInts>("raw_query_add_two_ints")
            .build()
            .await
            .expect("Failed to create service server");
        let key_expr = server.key_expr.clone();

        let request_task = tokio::spawn(async move {
            let error = match server.take_request_async().await {
                Ok(_) => panic!("request without attachment should fail"),
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("received ros-z service request without attachment metadata"),
                "unexpected error: {error}"
            );
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        client_node
            .session()
            .get(key_expr)
            .payload(
                SerdeCdrCodec::<AddTwoIntsRequest>::serialize(&AddTwoIntsRequest { a: 10, b: 32 })
                    .unwrap(),
            )
            .callback(|_| panic!("raw request should not receive a reply"))
            .await
            .expect("Failed to send raw service query");

        request_task.await.expect("Service task panicked");
    }
}

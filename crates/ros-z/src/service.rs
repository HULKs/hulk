use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, atomic::AtomicUsize},
    time::Duration,
};

use tracing::{debug, info, trace, warn};
use zenoh::{
    Result, Session, Wait, bytes, key_expr::KeyExpr, liveliness::LivelinessToken, query::Query,
    sample::Sample,
};

use std::sync::atomic::Ordering;

use crate::topic_name;

use crate::{
    attachment::{Attachment, EndpointGlobalId},
    entity::EndpointEntity,
    impl_with_type_info,
    msg::{Service, WireDecoder, WireMessage},
    queue::BoundedQueue,
};

#[derive(Debug)]
pub struct ServiceClientBuilder<T> {
    pub(crate) entity: EndpointEntity,
    pub(crate) session: Arc<Session>,
    pub(crate) clock: crate::time::Clock,
    pub(crate) _phantom_data: PhantomData<T>,
}

impl_with_type_info!(ServiceClientBuilder<T>);
impl_with_type_info!(ServiceServerBuilder<T>);

/// A native ros-z reusable service handle for typed request/response calls.
///
/// Create a client via
/// [`Node::create_service_client`](crate::node::Node::create_service_client).
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
    clock: crate::time::Clock,
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
        service = %self.entity.topic
    ))]
    pub async fn build(mut self) -> Result<ServiceClient<T>> {
        let Some(node) = self.entity.node.as_ref() else {
            return Err(zenoh::Error::from("client build requires node identity"));
        };
        // Qualify the service name as a ros-z graph name.
        let qualified_service =
            topic_name::qualify_service_name(&self.entity.topic, &node.namespace, &node.name)
                .map_err(|e| zenoh::Error::from(format!("Failed to qualify service: {}", e)))?;

        self.entity.topic = qualified_service.clone();
        debug!("[CLN] Qualified service: {}", qualified_service);

        let topic_key_expr = ros_z_protocol::format::topic_key_expr(&self.entity)?;
        let key_expr = (*topic_key_expr).clone();
        debug!("[CLN] Key expression: {}", key_expr);

        let inner = self
            .session
            .declare_querier(key_expr)
            .target(zenoh::query::QueryTarget::AllComplete)
            .consolidation(zenoh::query::ConsolidationMode::None)
            .await?;
        let liveliness_key_expr =
            ros_z_protocol::format::liveliness_key_expr(&self.entity, &self.session.zid())?;
        let lv_token = self
            .session
            .liveliness()
            .declare_token((*liveliness_key_expr).clone())
            .await?;
        debug!("[CLN] Client ready: service={}", self.entity.topic);

        Ok(ServiceClient {
            sequence_number: AtomicUsize::new(1), // Start at 1; zero is reserved for missing sequence values.
            inner,
            _lv_token: lv_token,
            endpoint_global_id: crate::entity::endpoint_global_id(&self.entity),
            topic: self.entity.topic.clone(),
            clock: self.clock,
            _phantom_data: Default::default(),
        })
    }
}

impl<T> ServiceClient<T>
where
    T: Service,
{
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
        let (response_tx, response_rx) = std::sync::mpsc::sync_channel(1);
        let response_tx = Arc::new(Mutex::new(Some(response_tx)));

        self.inner
            .get()
            .payload(payload)
            .attachment(attachment)
            .callback(move |reply| match reply.into_result() {
                Ok(sample) => {
                    let sender = response_tx
                        .lock()
                        .expect("service reply sender mutex poisoned")
                        .take();
                    match sender {
                        Some(sender) => {
                            if sender.send(sample).is_err() {
                                tracing::warn!(
                                    "Service call receiver dropped before reply delivery"
                                );
                            }
                        }
                        None => {
                            tracing::warn!("Service call received extra reply after completion");
                        }
                    }
                }
                Err(error) => {
                    tracing::debug!("Service reply error: {error:?}");
                }
            })
            .wait()?;

        match timeout {
            Some(timeout) => response_rx
                .recv_timeout(timeout)
                .map_err(|error| match error {
                    std::sync::mpsc::RecvTimeoutError::Timeout => {
                        zenoh::Error::from(format!("Service call timed out after {timeout:?}"))
                    }
                    std::sync::mpsc::RecvTimeoutError::Disconnected => {
                        zenoh::Error::from("Service call ended before any response was received")
                    }
                }),
            None => response_rx.recv().map_err(|_| {
                zenoh::Error::from("Service call ended before any response was received")
            }),
        }
    }

    async fn call_sample_async(&self, payload: impl Into<bytes::ZBytes>) -> Result<Sample> {
        let attachment = self.new_attachment();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let response_tx = Arc::new(Mutex::new(Some(response_tx)));

        self.inner
            .get()
            .payload(payload)
            .attachment(attachment)
            .callback(move |reply| match reply.into_result() {
                Ok(sample) => {
                    let sender = response_tx
                        .lock()
                        .expect("service reply sender mutex poisoned")
                        .take();
                    match sender {
                        Some(sender) => {
                            if sender.send(sample).is_err() {
                                tracing::warn!(
                                    "Service call receiver dropped before reply delivery"
                                );
                            }
                        }
                        None => {
                            tracing::warn!("Service call received extra reply after completion");
                        }
                    }
                }
                Err(error) => {
                    tracing::debug!("Service reply error: {error:?}");
                }
            })
            .await?;

        let sample = response_rx.await.map_err(|_| {
            zenoh::Error::from("Service call ended before any response was received")
        })?;

        Ok(sample)
    }

    fn decode_response(&self, sample: Sample) -> Result<T::Response>
    where
        T::Response: WireMessage,
        for<'a> <T::Response as WireMessage>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let payload_bytes = sample.payload().to_bytes();
        <T::Response as WireMessage>::deserialize(&payload_bytes[..])
            .map_err(|e| zenoh::Error::from(e.to_string()))
    }

    /// Call the service and wait indefinitely for the first reply.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`call_async`](Self::call_async) there.
    pub fn call(&self, message: &T::Request) -> Result<T::Response>
    where
        T::Request: WireMessage,
        T::Response: WireMessage,
        for<'a> <T::Response as WireMessage>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let sample = self.call_sample_blocking(message.serialize(), None)?;
        self.decode_response(sample)
    }

    /// Call the service and wait indefinitely for the first reply.
    pub async fn call_async(&self, message: &T::Request) -> Result<T::Response>
    where
        T::Request: WireMessage,
        T::Response: WireMessage,
        for<'a> <T::Response as WireMessage>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let sample = self.call_sample_async(message.serialize()).await?;
        self.decode_response(sample)
    }

    /// Call the service and fail if no reply arrives before `timeout` elapses.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`call_with_timeout_async`](Self::call_with_timeout_async) there.
    pub fn call_with_timeout(&self, message: &T::Request, timeout: Duration) -> Result<T::Response>
    where
        T::Request: WireMessage,
        T::Response: WireMessage,
        for<'a> <T::Response as WireMessage>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        let sample = self.call_sample_blocking(message.serialize(), Some(timeout))?;
        self.decode_response(sample)
    }

    /// Call the service and fail if no reply arrives before `timeout` elapses.
    pub async fn call_with_timeout_async(
        &self,
        message: &T::Request,
        timeout: Duration,
    ) -> Result<T::Response>
    where
        T::Request: WireMessage,
        T::Response: WireMessage,
        for<'a> <T::Response as WireMessage>::Codec:
            WireDecoder<Output = T::Response, Input<'a> = &'a [u8]>,
    {
        tokio::time::timeout(timeout, self.call_async(message))
            .await
            .map_err(|_| zenoh::Error::from(format!("Service call timed out after {timeout:?}")))?
    }
}

#[derive(Debug)]
pub struct ServiceServerBuilder<T> {
    pub(crate) entity: EndpointEntity,
    pub(crate) session: Arc<Session>,
    pub(crate) clock: crate::time::Clock,
    pub(crate) _phantom_data: PhantomData<T>,
}

impl<T> ServiceClientBuilder<T> {
    /// Set the QoS profile for this client.
    pub fn with_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.entity.qos = qos.to_protocol_qos();
        self
    }

    /// Get a reference to the native ros-z entity.
    pub fn entity(&self) -> &EndpointEntity {
        &self.entity
    }
}

impl<T> ServiceServerBuilder<T> {
    /// Set the QoS profile for this server.
    pub fn with_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.entity.qos = qos.to_protocol_qos();
        self
    }

    /// Get a reference to the native ros-z entity.
    pub fn entity(&self) -> &EndpointEntity {
        &self.entity
    }
}

pub struct ServiceServer<T: Service, Q = Query> {
    key_expr: KeyExpr<'static>,
    _inner: zenoh::query::Queryable<()>,
    _lv_token: LivelinessToken,
    clock: crate::time::Clock,
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
        mut self,
        handler: ServiceQueryHandler,
        queue: Option<Arc<BoundedQueue<Q>>>,
    ) -> Result<ServiceServer<T, Q>> {
        let Some(node) = self.entity.node.as_ref() else {
            return Err(zenoh::Error::from("service build requires node identity"));
        };
        let qualified_service =
            topic_name::qualify_service_name(&self.entity.topic, &node.namespace, &node.name)
                .map_err(|e| zenoh::Error::from(format!("Failed to qualify service: {}", e)))?;

        self.entity.topic = qualified_service;

        let topic_key_expr = ros_z_protocol::format::topic_key_expr(&self.entity)?;
        let key_expr = (*topic_key_expr).clone();
        tracing::debug!("[SRV] KE: {key_expr}");

        info!("[SRV] Declaring queryable on key expression: {}", key_expr);

        let inner = self
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
            .await?;

        let liveliness_key_expr =
            ros_z_protocol::format::liveliness_key_expr(&self.entity, &self.session.zid())?;
        let lv_token = self
            .session
            .liveliness()
            .declare_token((*liveliness_key_expr).clone())
            .await?;

        Ok(ServiceServer {
            key_expr,
            _inner: inner,
            _lv_token: lv_token,
            clock: self.clock,
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
        let queue_size = match self.entity.qos.history {
            ros_z_protocol::qos::QosHistory::KeepLast(depth) => depth,
            ros_z_protocol::qos::QosHistory::KeepAll => usize::MAX,
        };
        let queue = Arc::new(BoundedQueue::new(queue_size));
        self.build_internal(ServiceQueryHandler::Queue(queue.clone()), Some(queue))
            .await
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
    request_id: Option<RequestId>,
    key_expr: KeyExpr<'static>,
    query: Query,
    clock: crate::time::Clock,
    _phantom_data: PhantomData<T>,
}

impl<T: Service> ServiceReply<T> {
    pub fn id(&self) -> Option<&RequestId> {
        self.request_id.as_ref()
    }

    pub fn reply(self, message: &T::Response) -> Result<()> {
        let mut reply = self.query.reply(&self.key_expr, message.serialize());
        if let Some(request_id) = self.request_id {
            let attachment = Attachment::with_clock(
                request_id.sequence_number,
                request_id.writer_global_id,
                &self.clock,
            );
            reply = reply.attachment(attachment);
        }
        reply.wait()
    }

    pub async fn reply_async(self, message: &T::Response) -> Result<()> {
        let mut reply = self.query.reply(&self.key_expr, message.serialize());
        if let Some(request_id) = self.request_id {
            let attachment = Attachment::with_clock(
                request_id.sequence_number,
                request_id.writer_global_id,
                &self.clock,
            );
            reply = reply.attachment(attachment);
        }
        reply.await
    }
}

pub struct ServiceRequest<T: Service> {
    message: T::Request,
    reply: ServiceReply<T>,
}

impl<T: Service> ServiceRequest<T> {
    pub fn id(&self) -> Option<&RequestId> {
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

    pub fn reply(self, response: &T::Response) -> Result<()> {
        self.reply.reply(response)
    }

    pub async fn reply_async(self, response: &T::Response) -> Result<()> {
        self.reply.reply_async(response).await
    }
}

impl<T> ServiceServer<T, Query>
where
    T: Service,
{
    fn decode_request(&self, query: Query) -> Result<ServiceRequest<T>>
    where
        T::Request: WireMessage + Send + Sync + 'static,
        for<'a> <T::Request as WireMessage>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        let request_id = query
            .attachment()
            .map(Attachment::try_from)
            .transpose()?
            .map(RequestId::from);

        let payload_bytes = query
            .payload()
            .map(|payload| payload.to_bytes())
            .unwrap_or_default();
        let message = <T::Request as WireMessage>::deserialize(&payload_bytes[..])
            .map_err(|e| zenoh::Error::from(e.to_string()))?;

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
        T::Request: WireMessage + Send + Sync + 'static,
        for<'a> <T::Request as WireMessage>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        let queue = self.queue.as_ref().ok_or_else(|| {
            zenoh::Error::from("Server was built with callback, no queue available")
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
        T::Request: WireMessage + Send + Sync + 'static,
        for<'a> <T::Request as WireMessage>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        trace!("[SRV] Waiting for request");

        let queue = self.queue.as_ref().ok_or_else(|| {
            zenoh::Error::from("Server was built with callback, no queue available")
        })?;
        let query = queue.recv();
        self.decode_request(query)
    }

    /// Awaits the next request on the service and then deserializes the payload.
    ///
    /// This method may fail if the message does not deserialize as the requested type.
    pub async fn take_request_async(&mut self) -> Result<ServiceRequest<T>>
    where
        T::Request: WireMessage + Send + Sync + 'static,
        for<'a> <T::Request as WireMessage>::Codec:
            WireDecoder<Output = T::Request, Input<'a> = &'a [u8]>,
    {
        let queue = self.queue.as_ref().ok_or_else(|| {
            zenoh::Error::from("Server was built with callback, no queue available")
        })?;
        let query = queue.recv_async().await;
        self.decode_request(query)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        Message, ServiceTypeInfo,
        context::ContextBuilder,
        dynamic::{FieldType, MessageSchema},
        entity::{SchemaHash, TypeInfo},
        msg::WireMessage,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
    struct AddTwoIntsRequest {
        a: i64,
        b: i64,
    }

    impl Message for AddTwoIntsRequest {
        type Codec = crate::SerdeCdrCodec<Self>;

        fn type_name() -> &'static str {
            "test_msgs::AddTwoIntsRequest"
        }

        fn schema_hash() -> SchemaHash {
            SchemaHash::zero()
        }

        fn schema() -> std::sync::Arc<MessageSchema> {
            MessageSchema::builder("test_msgs::AddTwoIntsRequest")
                .field("a", FieldType::Int64)
                .field("b", FieldType::Int64)
                .build()
                .expect("schema should build")
        }
    }

    impl WireMessage for AddTwoIntsRequest {
        type Codec = crate::msg::SerdeCdrCodec<AddTwoIntsRequest>;
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
    struct AddTwoIntsResponse {
        sum: i64,
    }

    impl Message for AddTwoIntsResponse {
        type Codec = crate::SerdeCdrCodec<Self>;

        fn type_name() -> &'static str {
            "test_msgs::AddTwoIntsResponse"
        }

        fn schema_hash() -> SchemaHash {
            SchemaHash::zero()
        }

        fn schema() -> std::sync::Arc<MessageSchema> {
            MessageSchema::builder("test_msgs::AddTwoIntsResponse")
                .field("sum", FieldType::Int64)
                .build()
                .expect("schema should build")
        }
    }

    impl WireMessage for AddTwoIntsResponse {
        type Codec = crate::msg::SerdeCdrCodec<AddTwoIntsResponse>;
    }

    struct AddTwoInts;

    impl crate::msg::Service for AddTwoInts {
        type Request = AddTwoIntsRequest;
        type Response = AddTwoIntsResponse;
    }

    impl ServiceTypeInfo for AddTwoInts {
        fn service_type_info() -> TypeInfo {
            TypeInfo::new("test_msgs::AddTwoInts", None)
        }
    }

    // -----------------------------------------------------------------------
    // Topic name qualification for service names
    // Service names follow the same rules as topic names
    // -----------------------------------------------------------------------

    #[test]
    fn test_qualify_service_absolute_unchanged() {
        let result = crate::topic_name::qualify_service_name("/add_two_ints", "/", "node").unwrap();
        assert_eq!(result, "/add_two_ints");
    }

    #[test]
    fn test_qualify_service_relative_adds_slash() {
        let result = crate::topic_name::qualify_service_name("add_two_ints", "/", "node").unwrap();
        assert_eq!(result, "/add_two_ints");
    }

    #[test]
    fn test_qualify_service_with_namespace() {
        let result =
            crate::topic_name::qualify_service_name("add_two_ints", "/ns", "node").unwrap();
        assert_eq!(result, "/ns/add_two_ints");
    }

    #[test]
    fn test_qualify_service_multipart_name() {
        let result =
            crate::topic_name::qualify_service_name("/my/service/name", "/", "node").unwrap();
        assert_eq!(result, "/my/service/name");
    }

    // -----------------------------------------------------------------------
    // QoS stored in builder entity reflects the protocol values
    // -----------------------------------------------------------------------

    #[test]
    fn test_protocol_qos_default_is_reliable() {
        let qos = crate::qos::QosProfile::default();
        let proto = qos.to_protocol_qos();
        assert_eq!(
            proto.reliability,
            ros_z_protocol::qos::QosReliability::Reliable
        );
    }

    #[test]
    fn test_protocol_qos_default_is_volatile() {
        let qos = crate::qos::QosProfile::default();
        let proto = qos.to_protocol_qos();
        assert_eq!(
            proto.durability,
            ros_z_protocol::qos::QosDurability::Volatile
        );
    }

    #[test]
    fn test_zclientbuilder_with_qos_sets_reliability() {
        use crate::qos::{QosProfile, QosReliability};
        let qos = QosProfile {
            reliability: QosReliability::BestEffort,
            ..Default::default()
        };
        let proto = qos.to_protocol_qos();
        assert_eq!(
            proto.reliability,
            ros_z_protocol::qos::QosReliability::BestEffort
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn service_request_without_attachment_is_accepted_and_reply_has_no_attachment() {
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
            .create_service_server::<AddTwoInts>("raw_query_add_two_ints")
            .build()
            .await
            .expect("Failed to create service server");
        let key_expr = server.key_expr.clone();

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let reply_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(reply_tx)));
        let request_task = tokio::spawn(async move {
            let request = server
                .take_request_async()
                .await
                .expect("Failed to take request");
            assert!(request.id().is_none());
            assert_eq!(request.message().a, 10);
            assert_eq!(request.message().b, 32);

            request
                .reply_async(&AddTwoIntsResponse { sum: 42 })
                .await
                .expect("Failed to send response");
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        client_node
            .session()
            .get(key_expr)
            .payload(WireMessage::serialize(&AddTwoIntsRequest { a: 10, b: 32 }))
            .callback(move |reply| {
                let sample = reply.into_result().expect("Expected service reply sample");
                let sender = reply_tx
                    .lock()
                    .expect("Reply sender mutex poisoned")
                    .take()
                    .expect("Expected to deliver a single reply sample");
                sender
                    .send(sample)
                    .expect("Expected to deliver a single reply sample");
            })
            .await
            .expect("Failed to send raw service query");

        let reply_sample = reply_rx.await.expect("Failed to receive reply sample");
        assert!(reply_sample.attachment().is_none());

        let payload = reply_sample.payload().to_bytes();
        let response = <AddTwoIntsResponse as WireMessage>::deserialize(&payload)
            .expect("Failed to deserialize raw reply payload");
        assert_eq!(response.sum, 42);

        request_task.await.expect("Service task panicked");
    }
}

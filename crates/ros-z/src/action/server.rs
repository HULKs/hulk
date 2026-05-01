//! Action server implementation for native ros-z actions.
//!
//! This module provides the server-side functionality for native ros-z actions,
//! allowing nodes to accept goals from action clients, execute them,
//! provide feedback, and return results.

use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use zenoh::{Result, Wait, key_expr::KeyExpr, query::Query};

use super::{
    Action, GoalId, GoalInfo, GoalStatus,
    messages::*,
    state::{SafeGoalManager, ServerGoalState},
};
use crate::{
    attachment::Attachment, entity::TypeInfo, msg::WireMessage, topic_name::qualify_topic_name,
};

pub(crate) fn query_attachment(query: &Query) -> Result<Option<Attachment>> {
    query.attachment().map(Attachment::try_from).transpose()
}

fn decode_message_payload<T: WireMessage>(payload: Option<&[u8]>) -> Result<T>
where
    T::Codec: for<'a> crate::msg::WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    let payload = payload.ok_or_else(|| zenoh::Error::from("query payload is missing"))?;
    T::deserialize(payload).map_err(|error| {
        zenoh::Error::from(format!("failed to deserialize query payload: {error}"))
    })
}

pub(crate) fn decode_query_message<T: WireMessage>(query: &Query) -> Result<T>
where
    T::Codec: for<'a> crate::msg::WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    let payload = query.payload().map(|payload| payload.to_bytes());
    decode_message_payload(payload.as_ref().map(|bytes| bytes.as_ref()))
}

fn ensure_blocking_result_receive_runtime_supported() -> Result<()> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::CurrentThread => Err(zenoh::Error::from(
                "Blocking manual result receive cannot run on Tokio current_thread runtimes. Use receive_result_request_async() instead.",
            )),
            tokio::runtime::RuntimeFlavor::MultiThread => Ok(()),
            _ => Err(zenoh::Error::from(
                "Blocking manual result receive requires a supported Tokio runtime. Use receive_result_request_async() from async contexts.",
            )),
        }
    } else {
        Ok(())
    }
}

fn block_on_action_future<F, T>(future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(future))
            }
            tokio::runtime::RuntimeFlavor::CurrentThread => Err(zenoh::Error::from(
                "Blocking action APIs cannot run on Tokio current_thread runtimes. Use async action APIs instead.",
            )),
            _ => Err(zenoh::Error::from(
                "Blocking action APIs require a supported Tokio runtime. Use async action APIs from async contexts.",
            )),
        }
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                zenoh::Error::from(format!(
                    "failed to create Tokio runtime for blocking action call: {error}"
                ))
            })?
            .block_on(future)
    }
}

pub(crate) fn reply_with_attachment<T: WireMessage>(
    query: Query,
    key_expr: KeyExpr<'static>,
    attachment: Option<Attachment>,
    message: &T,
) -> Result<()> {
    let mut reply = query.reply(&key_expr, message.serialize());
    if let Some(attachment) = attachment {
        reply = reply.attachment(attachment);
    }
    reply.wait()
}

pub(crate) async fn reply_with_attachment_async<T: WireMessage>(
    query: Query,
    key_expr: KeyExpr<'static>,
    attachment: Option<Attachment>,
    message: &T,
) -> Result<()> {
    let mut reply = query.reply(&key_expr, message.serialize());
    if let Some(attachment) = attachment {
        reply = reply.attachment(attachment);
    }
    reply.await
}

pub struct CancelReply {
    key_expr: KeyExpr<'static>,
    query: Query,
    attachment: Option<Attachment>,
}

impl CancelReply {
    pub fn reply(self, response: &CancelGoalServiceResponse) -> Result<()> {
        reply_with_attachment(self.query, self.key_expr, self.attachment, response)
    }

    pub async fn reply_async(self, response: &CancelGoalServiceResponse) -> Result<()> {
        reply_with_attachment_async(self.query, self.key_expr, self.attachment, response).await
    }
}

pub struct CancelRequest {
    message: CancelGoalServiceRequest,
    reply: CancelReply,
}

impl CancelRequest {
    pub fn message(&self) -> &CancelGoalServiceRequest {
        &self.message
    }

    pub fn goal_info(&self) -> &GoalInfo {
        &self.message.goal_info
    }

    pub fn into_message(self) -> CancelGoalServiceRequest {
        self.message
    }

    pub fn into_parts(self) -> (CancelGoalServiceRequest, CancelReply) {
        (self.message, self.reply)
    }

    pub fn reply(self, response: &CancelGoalServiceResponse) -> Result<()> {
        self.reply.reply(response)
    }

    pub async fn reply_async(self, response: &CancelGoalServiceResponse) -> Result<()> {
        self.reply.reply_async(response).await
    }
}

pub struct ResultReply<A: Action> {
    key_expr: KeyExpr<'static>,
    query: Query,
    attachment: Option<Attachment>,
    _phantom_data: PhantomData<A>,
}

impl<A: Action> ResultReply<A> {
    pub fn reply(self, response: &GetResultResponse<A>) -> Result<()> {
        reply_with_attachment(self.query, self.key_expr, self.attachment, response)
    }

    pub async fn reply_async(self, response: &GetResultResponse<A>) -> Result<()> {
        reply_with_attachment_async(self.query, self.key_expr, self.attachment, response).await
    }
}

pub struct ResultRequestHandle<A: Action> {
    goal_id: GoalId,
    reply: ResultReply<A>,
}

impl<A: Action> ResultRequestHandle<A> {
    pub fn goal_id(&self) -> &GoalId {
        &self.goal_id
    }

    pub fn into_goal_id(self) -> GoalId {
        self.goal_id
    }

    pub fn into_parts(self) -> (GoalId, ResultReply<A>) {
        (self.goal_id, self.reply)
    }

    pub fn reply(self, response: &GetResultResponse<A>) -> Result<()> {
        self.reply.reply(response)
    }

    pub async fn reply_async(self, response: &GetResultResponse<A>) -> Result<()> {
        self.reply.reply_async(response).await
    }
}

/// Routes cancel requests from the shared cancel service queue to per-goal channels.
///
/// Follows zenoh-python's per-entity queue pattern: each executing goal registers
/// a dedicated channel. `drain()` reads the shared queue and routes by goal ID.
pub(crate) struct CancelDispatcher {
    routes: parking_lot::Mutex<HashMap<GoalId, flume::Sender<zenoh::query::Query>>>,
}

impl CancelDispatcher {
    pub(crate) fn new() -> Self {
        Self {
            routes: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    /// Register a goal; returns the per-goal receiver.
    pub(crate) fn register(&self, goal_id: GoalId) -> flume::Receiver<zenoh::query::Query> {
        let (tx, rx) = flume::bounded(4);
        self.routes.lock().insert(goal_id, tx);
        rx
    }

    /// Deregister a goal (call when goal terminates).
    pub(crate) fn deregister(&self, goal_id: GoalId) {
        self.routes.lock().remove(&goal_id);
    }

    /// Drain the shared cancel queue, routing each request to the appropriate per-goal channel.
    /// Messages for goals with no registered handle are logged and dropped.
    pub(crate) fn drain<A: Action>(
        &self,
        queue: &Arc<crate::queue::BoundedQueue<zenoh::query::Query>>,
        server: &ActionServer<A>,
    ) {
        while let Some(query) = queue.try_recv() {
            let Some(payload) = query.payload() else {
                tracing::warn!("CancelDispatcher: cancel query has no payload");
                let response = malformed_cancel_response();
                reply_to_cancel_query(
                    query,
                    &response,
                    "CancelDispatcher: failed to reply to payload-less cancel",
                );
                continue;
            };
            let goal_info =
                match <CancelGoalServiceRequest as WireMessage>::deserialize(&payload.to_bytes()) {
                    Ok(request) => request.goal_info,
                    Err(e) => {
                        tracing::warn!("CancelDispatcher: failed to parse cancel request: {}", e);
                        let response = malformed_cancel_response();
                        reply_to_cancel_query(
                            query,
                            &response,
                            "CancelDispatcher: failed to reply to malformed cancel",
                        );
                        continue;
                    }
                };
            let goal_id = goal_info.goal_id;
            let mut query = Some(query);

            enum CancelDispatch {
                Sent,
                Full(Query),
                Disconnected(Query),
                NoRoute(Query),
            }

            let dispatch = {
                let routes = self.routes.lock();
                if let Some(tx) = routes.get(&goal_id) {
                    match tx.try_send(query.take().expect("query should be available")) {
                        Ok(()) => CancelDispatch::Sent,
                        Err(flume::TrySendError::Full(query)) => CancelDispatch::Full(query),
                        Err(flume::TrySendError::Disconnected(query)) => {
                            CancelDispatch::Disconnected(query)
                        }
                    }
                } else {
                    CancelDispatch::NoRoute(
                        query
                            .take()
                            .expect("query should not be sent without a route"),
                    )
                }
            };
            match dispatch {
                CancelDispatch::Sent => {}
                CancelDispatch::Full(query) => {
                    tracing::warn!(
                        "CancelDispatcher: per-goal channel full for goal {:?}; replying immediately",
                        goal_id
                    );
                    let cancel_requested = server.request_cancel(goal_id);
                    let response = build_overflow_cancel_response(cancel_requested, goal_info);
                    reply_to_cancel_query(
                        query,
                        &response,
                        "CancelDispatcher: failed to reply to overflow cancel",
                    );
                }
                CancelDispatch::Disconnected(query) => {
                    tracing::warn!(
                        "CancelDispatcher: per-goal channel closed for goal {:?}",
                        goal_id
                    );
                    let response = build_cancel_response(false, goal_info);
                    reply_to_cancel_query(
                        query,
                        &response,
                        "CancelDispatcher: failed to reply to disconnected cancel",
                    );
                }
                CancelDispatch::NoRoute(query) => {
                    tracing::warn!(
                        "CancelDispatcher: no handle registered for goal {:?}",
                        goal_id
                    );
                    let response = build_cancel_response(false, goal_info);
                    reply_to_cancel_query(
                        query,
                        &response,
                        "CancelDispatcher: failed to reply to unrouted cancel",
                    );
                }
            }
        }
    }
}

/// Private implementation holding the actual server state.
/// This is wrapped by the public `ActionServer` handle.
pub(crate) struct InnerServer<A: Action> {
    pub(crate) goal_server: Arc<crate::service::ServiceServer<GoalService<A>>>,
    pub(crate) result_server: Arc<crate::service::ServiceServer<ResultService<A>>>,
    pub(crate) cancel_server: Arc<crate::service::ServiceServer<CancelService<A>>>,
    pub(crate) feedback_pub: Arc<
        crate::pubsub::Publisher<FeedbackMessage<A>, <FeedbackMessage<A> as WireMessage>::Codec>,
    >,
    pub(crate) status_pub:
        Arc<crate::pubsub::Publisher<StatusMessage, <StatusMessage as WireMessage>::Codec>>,
    pub(crate) goal_manager: Arc<SafeGoalManager<A>>,
    /// Token to cancel the default result handler when switching to full driver mode
    pub(crate) result_handler_token: CancellationToken,
    pub(crate) result_handler_stopped: Arc<AtomicBool>,
    pub(crate) result_handler_stopped_notify: Arc<Notify>,
    pub(crate) manual_result_mode: Arc<AtomicBool>,
    pub(crate) cancel_dispatcher: Arc<CancelDispatcher>,
}

/// Drop guard that triggers shutdown when the last server handle is dropped.
pub(crate) struct ShutdownGuard {
    pub(crate) token: CancellationToken,
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        tracing::debug!("ActionServer handle dropped, triggering shutdown");
        self.token.cancel();
    }
}

/// Builder for creating an action server.
///
/// The `ActionServerBuilder` allows you to configure timeouts and QoS settings
/// for different action communication channels before building the server.
///
/// # Examples
///
/// ```ignore
/// # use ros_z::action::*;
/// # use std::time::Duration;
/// # let node = todo!();
/// let server = node.create_action_server::<MyAction>("my_action")
///     .with_result_timeout(Duration::from_secs(30))
///     .build()
///     .await?;
/// # Ok::<(), zenoh::Error>(())
/// ```
pub struct ActionServerBuilder<'a, A: Action> {
    /// The name of the action.
    pub action_name: String,
    /// Reference to the node that will own this server.
    pub node: &'a crate::node::Node,
    /// Timeout for result requests.
    pub result_timeout: Duration,
    /// Optional timeout for goal execution.
    pub goal_timeout: Option<Duration>,
    /// QoS profile for the goal service.
    pub goal_service_qos: Option<crate::qos::QosProfile>,
    /// QoS profile for the result service.
    pub result_service_qos: Option<crate::qos::QosProfile>,
    /// QoS profile for the cancel service.
    pub cancel_service_qos: Option<crate::qos::QosProfile>,
    /// QoS profile for the feedback topic.
    pub feedback_topic_qos: Option<crate::qos::QosProfile>,
    /// QoS profile for the status topic.
    pub status_topic_qos: Option<crate::qos::QosProfile>,
    /// Override for goal (send_goal) type info; uses `A::send_goal_type_info()` if None.
    pub goal_type_info: Option<TypeInfo>,
    /// Override for result (get_result) type info; uses `A::get_result_type_info()` if None.
    pub result_type_info: Option<TypeInfo>,
    /// Override for feedback type info; uses `A::feedback_type_info()` if None.
    pub feedback_type_info: Option<TypeInfo>,
    pub _phantom: std::marker::PhantomData<A>,
}

impl<'a, A: Action> ActionServerBuilder<'a, A> {
    pub fn with_result_timeout(mut self, timeout: Duration) -> Self {
        self.result_timeout = timeout;
        self
    }

    pub fn with_goal_timeout(mut self, timeout: Duration) -> Self {
        self.goal_timeout = Some(timeout);
        self
    }

    pub fn with_goal_service_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.goal_service_qos = Some(qos);
        self
    }

    pub fn with_result_service_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.result_service_qos = Some(qos);
        self
    }

    pub fn with_cancel_service_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.cancel_service_qos = Some(qos);
        self
    }

    pub fn with_feedback_topic_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.feedback_topic_qos = Some(qos);
        self
    }

    pub fn with_status_topic_qos(mut self, qos: crate::qos::QosProfile) -> Self {
        self.status_topic_qos = Some(qos);
        self
    }

    /// Override the goal type info used for graph registration.
    ///
    /// By default `A::send_goal_type_info()` is used. Set this to supply a
    /// runtime-determined schema hash (e.g. from Python message classes).
    pub fn with_goal_type_info(mut self, info: TypeInfo) -> Self {
        self.goal_type_info = Some(info);
        self
    }

    /// Override the result type info used for graph registration.
    pub fn with_result_type_info(mut self, info: TypeInfo) -> Self {
        self.result_type_info = Some(info);
        self
    }

    /// Override the feedback type info used for graph registration.
    pub fn with_feedback_type_info(mut self, info: TypeInfo) -> Self {
        self.feedback_type_info = Some(info);
        self
    }
}

fn decode_cancel_request(query: Query) -> Result<CancelRequest> {
    let message = decode_query_message(&query)?;
    let reply = CancelReply {
        key_expr: query.key_expr().clone(),
        attachment: query_attachment(&query)?,
        query,
    };
    Ok(CancelRequest { message, reply })
}

fn decode_result_request<A: Action>(query: Query) -> Result<ResultRequestHandle<A>> {
    let request = decode_query_message::<GetResultRequest>(&query)?;
    let reply = ResultReply {
        key_expr: query.key_expr().clone(),
        attachment: query_attachment(&query)?,
        query,
        _phantom_data: PhantomData,
    };
    Ok(ResultRequestHandle {
        goal_id: request.goal_id,
        reply,
    })
}

fn should_commit_accepted_goal(reply_result: &Result<()>) -> bool {
    reply_result.is_ok()
}

pub(crate) fn build_cancel_response(
    cancelled: bool,
    goal_info: GoalInfo,
) -> CancelGoalServiceResponse {
    CancelGoalServiceResponse {
        return_code: if cancelled { 0 } else { 1 },
        goals_canceling: if cancelled { vec![goal_info] } else { vec![] },
    }
}

fn build_overflow_cancel_response(
    cancel_requested: bool,
    goal_info: GoalInfo,
) -> CancelGoalServiceResponse {
    build_cancel_response(cancel_requested, goal_info)
}

pub(crate) fn malformed_cancel_response() -> CancelGoalServiceResponse {
    CancelGoalServiceResponse {
        return_code: 1,
        goals_canceling: vec![],
    }
}

pub(crate) fn reply_to_cancel_query(
    query: Query,
    response: &CancelGoalServiceResponse,
    failure_context: &str,
) -> bool {
    let attachment = match query_attachment(&query) {
        Ok(attachment) => attachment,
        Err(error) => {
            tracing::warn!(
                "{}: failed to decode attachment: {}",
                failure_context,
                error
            );
            None
        }
    };
    let key_expr = query.key_expr().clone();
    if let Err(error) = reply_with_attachment(query, key_expr, attachment, response) {
        tracing::warn!("{}: {}", failure_context, error);
        return false;
    }
    true
}

impl<'a, A: Action> ActionServerBuilder<'a, A> {
    pub fn new(action_name: &str, node: &'a crate::node::Node) -> Self {
        Self {
            action_name: action_name.to_string(),
            node,
            result_timeout: Duration::from_secs(10),
            goal_timeout: None,
            goal_service_qos: None,
            result_service_qos: None,
            cancel_service_qos: None,
            feedback_topic_qos: None,
            status_topic_qos: None,
            goal_type_info: None,
            result_type_info: None,
            feedback_type_info: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

// Legacy result handler to preserve original behavior (using InnerServer)
async fn handle_result_requests_legacy_inner<A: Action>(
    inner: &InnerServer<A>,
    query: zenoh::query::Query,
) -> bool {
    tracing::debug!("Received result request");
    let request = match decode_query_message::<GetResultRequest>(&query) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to deserialize result request: {}", e);
            return false;
        }
    };

    // Look up goal result - extract data while holding lock, then release
    let result_data = inner.goal_manager.read(|manager| {
        if let Some(ServerGoalState::Terminated { result, status, .. }) =
            manager.goals.get(&request.goal_id)
        {
            Some((result.clone(), *status))
        } else {
            None
        }
    }); // Lock released here

    let handoff_requested = inner.manual_result_mode.load(Ordering::Acquire)
        || inner.result_handler_token.is_cancelled();

    if let Some((result, status)) = result_data {
        tracing::debug!(
            "Goal {:?} is terminated with status {:?}",
            request.goal_id,
            status
        );

        // Send result response without holding lock
        let response = GetResultResponse::<A> {
            status: status as i8,
            result,
        };
        let attachment = match query_attachment(&query) {
            Ok(attachment) => attachment,
            Err(error) => {
                tracing::warn!("Failed to decode result request attachment: {}", error);
                return false;
            }
        };
        let key_expr = query.key_expr().clone();
        if let Err(error) = reply_with_attachment(query, key_expr, attachment, &response) {
            tracing::warn!("Failed to send result response: {}", error);
        }
        tracing::debug!("Sent result response");
        false
    } else if handoff_requested {
        tracing::debug!(
            "Requeueing result request for goal {:?} during result-handler handoff",
            request.goal_id
        );
        if inner.result_server.queue().push(query) {
            tracing::warn!(
                "Dropped oldest queued result request while requeueing goal {:?} during result-handler handoff",
                request.goal_id
            );
        }
        true
    } else {
        tracing::warn!("Goal {:?} not found or not terminated yet", request.goal_id);
        // Server doesn't reply if goal is not ready yet
        false
    }
}

impl<'a, A: Action> ActionServerBuilder<'a, A> {
    pub async fn build(self) -> Result<ActionServer<A>> {
        // Apply remapping to action name
        let action_name = self.node.remap_rules.apply(&self.action_name);

        // Validate action name
        if action_name.is_empty() {
            return Err(zenoh::Error::from("Action name cannot be empty"));
        }

        // Qualify action name like a topic name
        let qualified_action_name = qualify_topic_name(
            &action_name,
            &self.node.entity.namespace,
            &self.node.entity.name,
        )?;

        tracing::debug!(
            "Action name: '{}', namespace: '{}', qualified: '{}'",
            action_name,
            self.node.entity.namespace,
            qualified_action_name
        );

        const ACTION_CHANNEL_PREFIX: &str = "_ros_z_action";
        let goal_service_name =
            format!("{qualified_action_name}/{ACTION_CHANNEL_PREFIX}/send_goal");
        let result_service_name =
            format!("{qualified_action_name}/{ACTION_CHANNEL_PREFIX}/get_result");
        let cancel_service_name =
            format!("{qualified_action_name}/{ACTION_CHANNEL_PREFIX}/cancel_goal");
        let feedback_topic_name =
            format!("{qualified_action_name}/{ACTION_CHANNEL_PREFIX}/feedback");
        let status_topic_name = format!("{qualified_action_name}/{ACTION_CHANNEL_PREFIX}/status");

        // Create goal server using node API for proper graph registration
        // Use override if provided, otherwise fall back to the action's static type info.
        let goal_type_info = Some(self.goal_type_info.unwrap_or_else(A::send_goal_type_info));
        let mut goal_server_builder = self
            .node
            .create_service_impl::<GoalService<A>>(&goal_service_name, goal_type_info);
        if let Some(qos) = self.goal_service_qos {
            goal_server_builder.entity.qos = qos.to_protocol_qos();
        }
        let goal_server = goal_server_builder.build().await?;

        // Create result server using node API for proper graph registration
        let result_type_info = Some(
            self.result_type_info
                .unwrap_or_else(A::get_result_type_info),
        );
        let mut result_server_builder = self
            .node
            .create_service_impl::<ResultService<A>>(&result_service_name, result_type_info);
        if let Some(qos) = self.result_service_qos {
            result_server_builder.entity.qos = qos.to_protocol_qos();
        }
        let result_server = result_server_builder.build().await?;
        tracing::debug!("Created result server for: {}", result_service_name);

        // Create cancel server using node API for proper graph registration
        // Use the native action cancel control type identity.
        let cancel_type_info = Some(A::cancel_goal_type_info());
        let mut cancel_server_builder = self
            .node
            .create_service_impl::<CancelService<A>>(&cancel_service_name, cancel_type_info);
        if let Some(qos) = self.cancel_service_qos {
            cancel_server_builder.entity.qos = qos.to_protocol_qos();
        }
        let cancel_server = cancel_server_builder.build().await?;

        // Create feedback publisher using node API for proper graph registration
        let feedback_type_info = Some(
            self.feedback_type_info
                .unwrap_or_else(A::feedback_type_info),
        );
        let mut feedback_pub_builder = self.node.publisher_with_type_info::<FeedbackMessage<A>>(
            &feedback_topic_name,
            feedback_type_info,
        );
        if let Some(qos) = self.feedback_topic_qos {
            feedback_pub_builder.entity.qos = qos.to_protocol_qos();
        }
        let feedback_pub = feedback_pub_builder.build().await?;

        // Create status publisher using node API for proper graph registration
        // Use the native action status type identity.
        let status_type_info = Some(A::status_type_info());
        let mut status_pub_builder = self
            .node
            .publisher_with_type_info::<StatusMessage>(&status_topic_name, status_type_info);
        if let Some(qos) = self.status_topic_qos {
            status_pub_builder.entity.qos = qos.to_protocol_qos();
        }
        let status_pub = status_pub_builder.build().await?;

        let goal_manager = Arc::new(SafeGoalManager::new(self.result_timeout, self.goal_timeout));

        let cancellation_token = CancellationToken::new();
        let result_handler_token = CancellationToken::new();
        let result_handler_stopped = Arc::new(AtomicBool::new(false));
        let result_handler_stopped_notify = Arc::new(Notify::new());
        let manual_result_mode = Arc::new(AtomicBool::new(false));

        // Create the inner server
        let inner = Arc::new(InnerServer {
            goal_server: Arc::new(goal_server),
            result_server: Arc::new(result_server),
            cancel_server: Arc::new(cancel_server),
            feedback_pub: Arc::new(feedback_pub),
            status_pub: Arc::new(status_pub),
            goal_manager,
            result_handler_token: result_handler_token.clone(),
            result_handler_stopped: result_handler_stopped.clone(),
            result_handler_stopped_notify: result_handler_stopped_notify.clone(),
            manual_result_mode: manual_result_mode.clone(),
            cancel_dispatcher: Arc::new(CancelDispatcher::new()),
        });

        // Spawn background task to handle result requests (default mode for manual goal handling)
        // This task will be cancelled if with_handler() is called
        let weak_inner = Arc::downgrade(&inner);
        let global_shutdown = cancellation_token.clone();
        let handler_token = result_handler_token.clone();
        let stopped_flag = result_handler_stopped.clone();
        let stopped_notify = result_handler_stopped_notify.clone();

        tokio::spawn(async move {
            while let Some(inner) = weak_inner.upgrade() {
                let query = tokio::select! {
                    _ = global_shutdown.cancelled() => {
                        tracing::debug!("Result handler stopping due to global shutdown");
                        break;
                    },
                    _ = handler_token.cancelled() => {
                        tracing::debug!("Result handler stopping - switching to full driver mode");
                        break;
                    },
                    query = inner.result_server.queue().recv_async() => query,
                };

                if handle_result_requests_legacy_inner(&inner, query).await {
                    break;
                }
            }

            stopped_flag.store(true, Ordering::Release);
            stopped_notify.notify_waiters();
        });

        // Note: cancel requests are NOT handled by a background task in polling mode.
        // In polling mode (Python), cancel requests are processed on-demand via
        // GoalHandle::try_process_cancel(), called from the is_cancel_requested getter.
        // This avoids competing with explicit receive_cancel_async() calls in Rust code.
        // In driver mode (with_handler), the driver loop handles cancel requests.

        Ok(ActionServer {
            inner,
            _shutdown: Arc::new(ShutdownGuard {
                token: cancellation_token,
            }),
            _runtime: None,
        })
    }
}

/// Action server handle using the Handle Pattern.
///
/// This is a lightweight, cloneable handle that wraps the actual server implementation.
/// When all handles are dropped, the server automatically shuts down.
pub struct ActionServer<A: Action> {
    inner: Arc<InnerServer<A>>,
    /// Drop guard that triggers shutdown when the last handle is dropped
    _shutdown: Arc<ShutdownGuard>,
    _runtime: Option<Arc<tokio::runtime::Runtime>>,
}

impl<A: Action> std::fmt::Debug for ActionServer<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionServer")
            .field("goal_server", &self.inner.goal_server)
            .finish_non_exhaustive()
    }
}

impl<A: Action> Clone for ActionServer<A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _shutdown: self._shutdown.clone(),
            _runtime: self._runtime.clone(),
        }
    }
}

// Internal helper for driver to create server handles and access inner fields
impl<A: Action> ActionServer<A> {
    pub(crate) fn from_inner(inner: Arc<InnerServer<A>>) -> Self {
        // Create a dummy shutdown guard that doesn't do anything
        // The driver doesn't control the server lifetime
        let dummy_token = CancellationToken::new();
        Self {
            inner,
            _shutdown: Arc::new(ShutdownGuard { token: dummy_token }),
            _runtime: None,
        }
    }
}

// Provide convenient access to inner fields via getter methods
impl<A: Action> ActionServer<A> {
    fn goal_server(&self) -> &Arc<crate::service::ServiceServer<GoalService<A>>> {
        &self.inner.goal_server
    }

    fn result_server(&self) -> &Arc<crate::service::ServiceServer<ResultService<A>>> {
        &self.inner.result_server
    }

    pub(crate) fn cancel_server(&self) -> &Arc<crate::service::ServiceServer<CancelService<A>>> {
        &self.inner.cancel_server
    }

    pub(crate) fn cancel_dispatcher(&self) -> &Arc<CancelDispatcher> {
        &self.inner.cancel_dispatcher
    }

    fn feedback_pub(
        &self,
    ) -> &Arc<
        crate::pubsub::Publisher<FeedbackMessage<A>, <FeedbackMessage<A> as WireMessage>::Codec>,
    > {
        &self.inner.feedback_pub
    }

    fn status_pub(
        &self,
    ) -> &Arc<crate::pubsub::Publisher<StatusMessage, <StatusMessage as WireMessage>::Codec>> {
        &self.inner.status_pub
    }

    /// Access the goal manager for advanced use cases and testing.
    ///
    /// # Warning
    ///
    /// This is a low-level API that gives direct access to the goal state.
    /// Use with caution as it bypasses the normal goal handle abstractions.
    pub fn goal_manager(&self) -> &Arc<SafeGoalManager<A>> {
        &self.inner.goal_manager
    }

    fn result_handler_token(&self) -> &CancellationToken {
        &self.inner.result_handler_token
    }

    fn result_handler_stopped(&self) -> &Arc<AtomicBool> {
        &self.inner.result_handler_stopped
    }

    fn result_handler_stopped_notify(&self) -> &Arc<Notify> {
        &self.inner.result_handler_stopped_notify
    }

    fn manual_result_mode(&self) -> &Arc<AtomicBool> {
        &self.inner.manual_result_mode
    }

    fn stop_result_handler(&self) {
        self.result_handler_token().cancel();
        while !self.result_handler_stopped().load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    async fn stop_result_handler_async(&self) {
        self.result_handler_token().cancel();
        while !self.result_handler_stopped().load(Ordering::Acquire) {
            let notified = self.result_handler_stopped_notify().notified();
            if self.result_handler_stopped().load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

impl<A: Action> ActionServer<A> {
    pub fn try_publish_status(&self) -> Result<()> {
        block_on_action_future(self.try_publish_status_async())
    }

    pub async fn try_publish_status_async(&self) -> Result<()> {
        // Build status list while holding lock, then release before publishing
        let status_list: Vec<GoalStatusInfo> = self.goal_manager().read(|manager| {
            manager
                .goals
                .iter()
                .map(|(goal_id, state)| {
                    let status = match state {
                        ServerGoalState::Accepted { .. } => GoalStatus::Accepted,
                        ServerGoalState::Executing { .. } => GoalStatus::Executing,
                        ServerGoalState::Canceling { .. } => GoalStatus::Canceling,
                        ServerGoalState::Terminated { status, .. } => *status,
                    };
                    GoalStatusInfo {
                        goal_info: GoalInfo::new(*goal_id),
                        status,
                    }
                })
                .collect()
        }); // Lock released here

        // Publish without holding lock
        let message = StatusMessage { status_list };
        self.status_pub().publish(&message).await
    }

    fn publish_status(&self) {
        if let Err(error) = self.try_publish_status() {
            tracing::warn!("Failed to publish action status: {}", error);
        }
    }

    pub fn receive_goal(&self) -> Result<GoalHandle<A, Requested>> {
        let query = self.goal_server().queue().recv();
        let request = decode_query_message::<SendGoalRequest<A>>(&query)?;
        let reply_attachment = query_attachment(&query)?;

        Ok(GoalHandle {
            goal: request.goal,
            info: GoalInfo::new(request.goal_id),
            server: self.clone(),
            query: Some(query),
            reply_attachment,
            cancel_flag: None,
            cancel_rx: None,
            _state: PhantomData,
        })
    }

    pub async fn receive_goal_async(&self) -> Result<GoalHandle<A, Requested>> {
        let query = self.goal_server().queue().recv_async().await;
        let request = decode_query_message::<SendGoalRequest<A>>(&query)?;
        let reply_attachment = query_attachment(&query)?;

        Ok(GoalHandle {
            goal: request.goal,
            info: GoalInfo::new(request.goal_id),
            server: self.clone(),
            query: Some(query),
            reply_attachment,
            cancel_flag: None,
            cancel_rx: None,
            _state: PhantomData,
        })
    }

    pub fn receive_cancel(&self) -> Result<CancelRequest> {
        let query = self.cancel_server().queue().recv();
        decode_cancel_request(query)
    }

    pub async fn receive_cancel_async(&self) -> Result<CancelRequest> {
        let query = self.cancel_server().queue().recv_async().await;
        decode_cancel_request(query)
    }

    pub fn is_cancel_request_ready(&self) -> bool {
        !self.cancel_server().queue().is_empty()
    }

    /// Marks a goal as canceling by setting its atomic cancel flag.
    /// This is a lock-free operation that can be called from any thread.
    pub fn request_cancel(&self, goal_id: GoalId) -> bool {
        self.goal_manager().read(|manager| {
            if let Some(ServerGoalState::Executing { cancel_flag, .. }) =
                manager.goals.get(&goal_id)
            {
                cancel_flag.store(true, Ordering::Relaxed);
                true
            } else {
                false
            }
        })
    }

    /// Receives the next manual result request using the blocking API.
    ///
    /// This call disables the legacy background result handler before reading from the
    /// shared result queue so manual result handling has exclusive ownership.
    ///
    /// Do not call this from Tokio `current_thread` runtimes. That runtime cannot safely
    /// block while the background result handler shuts down, so this method returns a
    /// normal [`zenoh::Error`] with guidance to use
    /// [`receive_result_request_async`](Self::receive_result_request_async) instead.
    pub fn receive_result_request(&self) -> Result<ResultRequestHandle<A>> {
        ensure_blocking_result_receive_runtime_supported()?;
        // Manual result receives must own the queue so the legacy background responder
        // does not race and consume the same request first.
        self.manual_result_mode().store(true, Ordering::Release);
        self.stop_result_handler();
        let query = self.result_server().queue().recv();
        decode_result_request(query)
    }

    pub async fn receive_result_request_async(&self) -> Result<ResultRequestHandle<A>> {
        // Manual result receives must own the queue so the legacy background responder
        // does not race and consume the same request first.
        self.manual_result_mode().store(true, Ordering::Release);
        self.stop_result_handler_async().await;
        let query = self.result_server().queue().recv_async().await;
        decode_result_request(query)
    }

    /// Attaches an automatic goal handler to the server.
    ///
    /// This method transitions the server from "manual mode" (where you call `receive_goal_async()`)
    /// to "automatic mode" (where goals are handled by the provided callback).
    ///
    /// **Important**: This method cancels the default result-only handler and starts a full
    /// driver loop that handles all protocol events (goals, cancels, results) automatically.
    ///
    /// # Arguments
    ///
    /// * `handler` - Callback function that will be invoked for each accepted goal
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use ros_z::action::*;
    /// # let server = todo!();
    /// let server = server.with_handler(|executing| async move {
    ///     // Process the goal
    ///     executing.succeed(result).unwrap();
    /// });
    /// ```
    pub fn with_handler<F, Fut>(self, handler: F) -> Self
    where
        F: Fn(GoalHandle<A, Executing>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // 1. Stop the default result-only handler to avoid competing for result_server.rx
        tracing::debug!("Cancelling default result handler to switch to full driver mode");
        self.result_handler_token().cancel();

        // 2. Start the full driver loop that handles all protocol events
        let weak_inner = Arc::downgrade(&self.inner);
        let shutdown_token = self._shutdown.token.clone();
        tokio::spawn(async move {
            crate::action::driver::run_driver_loop(weak_inner, shutdown_token, handler).await;
        });

        self
    }

    /// Expires goals that have passed their expiration time.
    ///
    /// This method checks all goals with `expires_at` timestamps and removes:
    /// - Accepted/Executing goals that have timed out (goal timeout)
    /// - Terminated goals whose results have expired (result timeout)
    ///
    /// Goals without expiration times (when timeouts are not configured) are never expired.
    ///
    /// # Returns
    ///
    /// Returns a vector of `GoalId`s that were expired.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use ros_z::action::*;
    /// # let server: ros_z::action::server::ActionServer<MyAction> = todo!();
    /// // Check and expire any goals that have passed their expiration time
    /// let expired = server.expire_goals();
    /// println!("Expired {} goals", expired.len());
    /// ```
    pub fn expire_goals(&self) -> Vec<GoalId> {
        let expired = self.goal_manager().modify(|manager| {
            let now = Instant::now();
            let mut expired = Vec::new();

            // Find goals that have passed their expiration time
            manager.goals.retain(|goal_id, state| {
                let should_expire = match state {
                    ServerGoalState::Accepted { expires_at, .. }
                    | ServerGoalState::Executing { expires_at, .. }
                    | ServerGoalState::Terminated { expires_at, .. } => {
                        expires_at.is_some_and(|exp| now >= exp)
                    }
                    ServerGoalState::Canceling { .. } => false,
                };

                if should_expire {
                    expired.push(*goal_id);
                    false // Remove this goal
                } else {
                    true // Keep this goal
                }
            });

            expired
        }); // Lock released here

        // Publish updated status if any goals were expired
        if !expired.is_empty() {
            self.publish_status();
        }

        expired
    }

    /// Sets the result timeout for this server.
    ///
    /// This configures how long the server will keep terminated goals
    /// before they can be expired. Note: This does not automatically
    /// expire goals - you must call `expire_goals()` periodically.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The result timeout duration
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use ros_z::action::*;
    /// # use std::time::Duration;
    /// # let mut server: ros_z::action::server::ActionServer<MyAction> = todo!();
    /// server.set_result_timeout(Duration::from_secs(30));
    /// ```
    pub fn set_result_timeout(&self, timeout: Duration) {
        self.goal_manager().modify(|manager| {
            manager.result_timeout = timeout;
        });
    }

    /// Gets the current result timeout for this server.
    ///
    /// # Returns
    ///
    /// The result timeout duration
    pub fn result_timeout(&self) -> Duration {
        self.goal_manager().read(|manager| manager.result_timeout)
    }
}

// --- State Markers for Type-State Pattern ---
/// Marker type representing a goal that has been requested but not yet accepted or rejected.
pub struct Requested;

/// Marker type representing a goal that has been accepted but not yet executing.
pub struct Accepted;

/// Marker type representing a goal that is currently executing.
pub struct Executing;

// Type aliases for convenience
/// A goal handle in the "Requested" state.
pub type RequestedGoal<A> = GoalHandle<A, Requested>;

/// A goal handle in the "Accepted" state.
pub type AcceptedGoal<A> = GoalHandle<A, Accepted>;

/// A goal handle in the "Executing" state.
pub type ExecutingGoal<A> = GoalHandle<A, Executing>;

// Type-state pattern for goal lifecycle with PhantomData markers
/// A type-safe goal handle that uses compile-time state tracking.
///
/// The `GoalHandle` is generic over the action type `A` and the state `State`.
/// Different methods are available depending on the current state, enforced at compile time.
///
/// # Type States
///
/// - `GoalHandle<A, Requested>`: Can be accepted or rejected
/// - `GoalHandle<A, Accepted>`: Can be executed
/// - `GoalHandle<A, Executing>`: Can publish feedback and be terminated
///
/// # Examples
///
/// ```ignore
/// # use ros_z::action::*;
/// # let server: std::sync::Arc<server::ActionServer<MyAction>> = todo!();
/// # async {
/// let requested = server.receive_goal_async().await?;
/// let accepted = requested.accept();
/// let executing = accepted.execute();
/// executing.succeed(result)?;
/// # Ok::<(), zenoh::Error>(())
/// # };
/// ```
pub struct GoalHandle<A: Action, State> {
    /// The goal data.
    pub goal: A::Goal,
    /// The goal metadata.
    pub info: GoalInfo,
    pub(crate) server: ActionServer<A>,
    pub(crate) query: Option<zenoh::query::Query>,
    pub(crate) reply_attachment: Option<Attachment>,
    pub(crate) cancel_flag: Option<Arc<AtomicBool>>,
    /// Per-goal cancel channel registered with the CancelDispatcher (Some only in Executing state).
    pub(crate) cancel_rx: Option<flume::Receiver<zenoh::query::Query>>,
    pub(crate) _state: PhantomData<State>,
}

// --- State-specific implementations ---

/// Methods available only for goals in the "Requested" state.
impl<A: Action> GoalHandle<A, Requested> {
    /// Access the goal data.
    pub fn goal(&self) -> &A::Goal {
        &self.goal
    }

    /// Access the goal info.
    pub fn info(&self) -> &GoalInfo {
        &self.info
    }

    /// Accept this goal and transition to the "Accepted" state.
    ///
    /// This sends an acceptance response to the client and updates the server state.
    fn send_accept_reply(&mut self) -> Result<()> {
        let response = SendGoalResponse {
            accepted: true,
            stamp_sec: self.info.stamp.sec,
            stamp_nanosec: self.info.stamp.nanosec,
        };
        if let Some(query) = self.query.take() {
            let key_expr = query.key_expr().clone();
            let attachment = self.reply_attachment.take();
            reply_with_attachment(query, key_expr, attachment, &response)?;
        }
        Ok(())
    }

    fn insert_accepted_goal(&self) {
        self.server.goal_manager().modify(|manager| {
            let expires_at = manager.goal_timeout.map(|timeout| Instant::now() + timeout);
            manager.goals.insert(
                self.info.goal_id,
                ServerGoalState::Accepted {
                    goal: self.goal.clone(),
                    timestamp: Instant::now(),
                    expires_at,
                },
            );
        });
    }

    fn into_accepted_goal(self) -> GoalHandle<A, Accepted> {
        GoalHandle {
            goal: self.goal,
            info: self.info,
            server: self.server,
            query: None,
            reply_attachment: None,
            cancel_flag: None,
            cancel_rx: None,
            _state: PhantomData,
        }
    }

    pub fn try_accept(mut self) -> Result<GoalHandle<A, Accepted>> {
        let reply_result = self.send_accept_reply();
        if !should_commit_accepted_goal(&reply_result) {
            return match reply_result {
                Ok(()) => unreachable!("accepted goal reply success should allow commit"),
                Err(error) => Err(error),
            };
        }

        self.insert_accepted_goal();
        if let Err(error) = self.server.try_publish_status() {
            tracing::warn!("Failed to publish action status after accept: {}", error);
        }
        Ok(self.into_accepted_goal())
    }

    pub fn accept(mut self) -> GoalHandle<A, Accepted> {
        if let Err(error) = self.send_accept_reply() {
            tracing::warn!("Failed to accept action goal cleanly: {}", error);
        }
        self.insert_accepted_goal();
        self.server.publish_status();
        self.into_accepted_goal()
    }

    /// Reject this goal.
    ///
    /// This sends a rejection response to the client. The goal will not be executed.
    pub fn try_reject(mut self) -> Result<()> {
        // Send rejection response
        let response = GoalResponse {
            accepted: false,
            stamp_sec: 0,
            stamp_nanosec: 0,
        };
        if let Some(query) = self.query.take() {
            let key_expr = query.key_expr().clone();
            let attachment = self.reply_attachment.take();
            reply_with_attachment(query, key_expr, attachment, &response)?;
        }
        Ok(())
    }

    pub fn reject(self) -> Result<()> {
        self.try_reject().inspect_err(|error| {
            tracing::warn!("Failed to send goal rejection response: {}", error);
        })
    }
}

/// Methods available only for goals in the "Accepted" state.
impl<A: Action> GoalHandle<A, Accepted> {
    /// Access the goal data.
    pub fn goal(&self) -> &A::Goal {
        &self.goal
    }

    /// Access the goal info.
    pub fn info(&self) -> &GoalInfo {
        &self.info
    }

    /// Begin executing this goal and transition to the "Executing" state.
    ///
    /// This updates the server state to executing and publishes a status update.
    pub fn execute(self) -> GoalHandle<A, Executing> {
        self.execute_with_cancel_dispatch(true)
    }

    pub(crate) fn execute_driver(self) -> GoalHandle<A, Executing> {
        self.execute_with_cancel_dispatch(false)
    }

    fn execute_with_cancel_dispatch(self, register_cancel_rx: bool) -> GoalHandle<A, Executing> {
        // Create cancel flag
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let cancel_rx =
            register_cancel_rx.then(|| self.server.cancel_dispatcher().register(self.info.goal_id));

        // Transition to EXECUTING
        self.server.goal_manager().modify(|manager| {
            let expires_at = manager.goal_timeout.map(|timeout| Instant::now() + timeout);
            manager.goals.insert(
                self.info.goal_id,
                ServerGoalState::Executing {
                    goal: self.goal.clone(),
                    cancel_flag: cancel_flag.clone(),
                    expires_at,
                },
            );
        });

        self.server.publish_status();

        GoalHandle {
            goal: self.goal,
            info: self.info,
            server: self.server,
            query: None,
            reply_attachment: None,
            cancel_flag: Some(cancel_flag),
            cancel_rx,
            _state: PhantomData,
        }
    }
}

/// Methods available only for goals in the "Executing" state.
impl<A: Action> GoalHandle<A, Executing> {
    /// Access the goal data.
    pub fn goal(&self) -> &A::Goal {
        &self.goal
    }

    /// Access the goal info.
    pub fn info(&self) -> &GoalInfo {
        &self.info
    }

    /// Publish feedback for this goal.
    ///
    /// Feedback can be published multiple times during goal execution to inform
    /// the client of progress.
    pub fn publish_feedback(&self, feedback: A::Feedback) -> Result<()> {
        block_on_action_future(self.publish_feedback_async(feedback))
    }

    pub async fn publish_feedback_async(&self, feedback: A::Feedback) -> Result<()> {
        let message = FeedbackMessage {
            goal_id: self.info.goal_id,
            feedback,
        };
        self.server.feedback_pub().publish(&message).await
    }

    /// Check if cancellation has been requested for this goal.
    ///
    /// In polling mode this also processes pending cancel requests for this goal.
    ///
    /// # Returns
    ///
    /// `true` if a cancel request has been received, `false` otherwise.
    pub fn is_cancel_requested(&self) -> bool {
        self.try_process_cancel()
    }

    fn cancel_flag_requested(&self) -> bool {
        self.cancel_flag
            .as_ref()
            .map(|flag| flag.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    /// Check for and process any pending cancel request for this goal (polling mode).
    ///
    /// This is a non-blocking operation that drains the shared cancel queue via the
    /// `CancelDispatcher`, routing each message to the appropriate per-goal channel.
    /// Returns `true` if a cancel was requested for this goal (either via the flag
    /// already set, or a newly routed request processed here).
    ///
    /// Fixes the silent-drop bug where a cancel for goal B would be lost if goal A's
    /// handle polled first and found a goal ID mismatch. Each goal now has its own
    /// dedicated channel; `drain()` routes all pending messages before we check ours.
    pub fn try_process_cancel(&self) -> bool {
        let flag_requested = self.cancel_flag_requested();

        let Some(cancel_rx) = &self.cancel_rx else {
            return flag_requested;
        };

        // Drain shared cancel queue into per-goal channels
        self.server
            .cancel_dispatcher()
            .drain(self.server.cancel_server().queue(), &self.server);

        let mut processed = false;
        while let Ok(query) = cancel_rx.try_recv() {
            let payload = match query.payload() {
                Some(payload) => payload.to_bytes(),
                None => {
                    tracing::warn!("try_process_cancel: cancel query has no payload");
                    let response = malformed_cancel_response();
                    reply_to_cancel_query(
                        query,
                        &response,
                        "try_process_cancel: failed to reply to payload-less cancel",
                    );
                    continue;
                }
            };
            let request = match <CancelGoalServiceRequest as WireMessage>::deserialize(&payload) {
                Ok(request) => request,
                Err(error) => {
                    tracing::error!("try_process_cancel: deserialize error: {}", error);
                    let response = malformed_cancel_response();
                    reply_to_cancel_query(
                        query,
                        &response,
                        "try_process_cancel: failed to reply to malformed cancel",
                    );
                    continue;
                }
            };
            let response = build_cancel_response(true, request.goal_info);
            self.server.request_cancel(self.info.goal_id);
            if !reply_to_cancel_query(
                query,
                &response,
                "try_process_cancel: failed to send cancel response",
            ) {
                continue;
            }
            processed = true;
        }
        flag_requested || processed
    }

    /// Mark this goal as succeeded with the given result.
    ///
    /// This transitions the goal to a terminal state and consumes the handle.
    pub fn succeed(self, result: A::Result) -> Result<()> {
        self.terminate(result, GoalStatus::Succeeded)
    }

    /// Mark this goal as aborted with the given result.
    ///
    /// This transitions the goal to a terminal state and consumes the handle.
    pub fn abort(self, result: A::Result) -> Result<()> {
        self.terminate(result, GoalStatus::Aborted)
    }

    /// Mark this goal as canceled with the given result.
    ///
    /// This transitions the goal to a terminal state and consumes the handle.
    pub fn canceled(self, result: A::Result) -> Result<()> {
        self.terminate(result, GoalStatus::Canceled)
    }

    fn reply_to_pending_routed_cancels(&self, accepted: bool) {
        let Some(cancel_rx) = &self.cancel_rx else {
            return;
        };

        while let Ok(query) = cancel_rx.try_recv() {
            let payload = match query.payload() {
                Some(payload) => payload.to_bytes(),
                None => {
                    tracing::warn!("terminate: pending cancel query has no payload");
                    let response = malformed_cancel_response();
                    reply_to_cancel_query(
                        query,
                        &response,
                        "terminate: failed to reply to payload-less pending cancel",
                    );
                    continue;
                }
            };
            let request = match <CancelGoalServiceRequest as WireMessage>::deserialize(&payload) {
                Ok(request) => request,
                Err(error) => {
                    tracing::warn!("terminate: failed to deserialize pending cancel: {}", error);
                    let response = malformed_cancel_response();
                    reply_to_cancel_query(
                        query,
                        &response,
                        "terminate: failed to reply to malformed pending cancel",
                    );
                    continue;
                }
            };
            let response = build_cancel_response(accepted, request.goal_info);
            reply_to_cancel_query(
                query,
                &response,
                "terminate: failed to reply to pending cancel",
            );
        }
    }

    fn terminate(self, result: A::Result, status: GoalStatus) -> Result<()> {
        let has_cancel_rx = self.cancel_rx.is_some();

        // Deregister from the cancel dispatcher so no more cancel messages are routed here
        self.server
            .cancel_dispatcher()
            .deregister(self.info.goal_id);

        self.reply_to_pending_routed_cancels(status == GoalStatus::Canceled);

        if has_cancel_rx {
            self.server
                .cancel_dispatcher()
                .drain(self.server.cancel_server().queue(), &self.server);
        }

        // Notify any waiting result futures
        let futures_to_notify = self.server.goal_manager().modify(|manager| {
            let now = Instant::now();
            let expires_at = Some(now + manager.result_timeout);
            manager.goals.insert(
                self.info.goal_id,
                ServerGoalState::Terminated {
                    result: result.clone(),
                    status,
                    timestamp: now,
                    expires_at,
                },
            );

            // Take all waiting result futures for this goal
            manager
                .result_futures
                .remove(&self.info.goal_id)
                .unwrap_or_default()
        }); // Drop the lock before notifying futures and publishing status

        // Notify all waiting result futures
        for tx in futures_to_notify {
            let _ = tx.send((result.clone(), status));
        }

        self.server.publish_status();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{
        Accepted, ActionServer, GoalHandle, Requested, build_cancel_response,
        build_overflow_cancel_response, decode_message_payload,
        ensure_blocking_result_receive_runtime_supported, should_commit_accepted_goal,
    };
    use crate::action::messages::CancelGoalServiceRequest;
    use crate::{Result, define_action};

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct TestGoal;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct TestResult;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct TestFeedback;

    struct TestAction;

    define_action! {
        TestAction,
        action_name: "test_action",
        Goal: TestGoal,
        Result: TestResult,
        Feedback: TestFeedback,
    }

    #[test]
    fn blocking_result_receive_runtime_guard_rejects_current_thread() {
        let error = std::thread::spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create current_thread runtime for regression test")
                .block_on(async { ensure_blocking_result_receive_runtime_supported() })
        })
        .join()
        .expect("current_thread regression test should not panic")
        .expect_err("current_thread runtimes must be rejected");

        assert!(
            error.to_string().contains("receive_result_request_async()"),
            "expected async receive guidance, got: {error}"
        );
    }

    #[test]
    fn decode_message_payload_reports_missing_payload() {
        let error = decode_message_payload::<CancelGoalServiceRequest>(None)
            .expect_err("missing payload should be rejected");

        assert!(
            error.to_string().contains("payload"),
            "expected payload error, got: {error}"
        );
    }

    #[test]
    fn decode_message_payload_reports_invalid_cdr() {
        let error = decode_message_payload::<CancelGoalServiceRequest>(Some(&[0xFF, 0x00]))
            .expect_err("invalid CDR should be rejected");

        assert!(
            error.to_string().contains("deserialize") || error.to_string().contains("cdr"),
            "expected deserialize error, got: {error}"
        );
    }

    #[test]
    fn accept_commit_is_gated_only_by_reply_delivery() {
        assert!(should_commit_accepted_goal(&Ok(())));

        let reply_error = Err(zenoh::Error::from("reply failed"));
        assert!(!should_commit_accepted_goal(&reply_error));
    }

    #[test]
    fn cancel_response_uses_success_return_code_when_cancel_is_accepted() {
        let goal_info = crate::action::GoalInfo::new(crate::action::GoalId::new());
        let response = build_cancel_response(true, goal_info);

        assert_eq!(response.return_code, 0);
        assert_eq!(response.goals_canceling.len(), 1);
    }

    #[test]
    fn overflow_cancel_response_reflects_request_cancel_failure() {
        let goal_info = crate::action::GoalInfo::new(crate::action::GoalId::new());
        let response = build_overflow_cancel_response(false, goal_info);

        assert_ne!(response.return_code, 0);
        assert!(response.goals_canceling.is_empty());
    }

    #[test]
    fn requested_goal_exposes_fallible_accept_and_reject() {
        type RequestedTestGoal = GoalHandle<TestAction, Requested>;
        type AcceptedTestGoal = GoalHandle<TestAction, Accepted>;

        let _accept: fn(RequestedTestGoal) -> Result<AcceptedTestGoal> =
            GoalHandle::<TestAction, Requested>::try_accept;
        let _reject: fn(RequestedTestGoal) -> Result<()> =
            GoalHandle::<TestAction, Requested>::try_reject;
    }

    #[test]
    fn action_server_exposes_fallible_status_publish() {
        let _publish_status: fn(&ActionServer<TestAction>) -> Result<()> =
            ActionServer::<TestAction>::try_publish_status;
    }
}

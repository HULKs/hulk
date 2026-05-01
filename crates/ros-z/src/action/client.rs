//! Action client implementation for native ros-z actions.
//!
//! This module provides the client-side functionality for native ros-z actions,
//! allowing nodes to send goals to action servers, receive feedback,
//! monitor goal status, and retrieve results.

use std::{collections::HashMap, future::Future, marker::PhantomData, sync::Arc, time::Duration};

use parking_lot::Mutex;
use tokio::sync::{mpsc, watch};
use zenoh::Result;

use super::{Action, GoalId, GoalInfo, GoalStatus, Time, messages::*};
use crate::{entity::TypeInfo, msg::WireMessage, qos::QosProfile, topic_name::qualify_topic_name};

/// Type states for goal handles.
pub mod goal_state {
    /// The goal is active and can be monitored or canceled.
    pub struct Active;
    /// The goal has been terminated and cannot be used further.
    pub struct Terminated;
}

/// Builder for creating an action client.
///
/// The `ActionClientBuilder` allows you to configure QoS settings for different
/// action communication channels before building the client.
///
/// # Examples
///
/// ```ignore
/// # use ros_z::action::*;
/// # use ros_z::qos::QosProfile;
/// # let node = todo!();
/// let client = node.create_action_client::<MyAction>("my_action")
///     .with_goal_service_qos(QosProfile::default())
///     .build()
///     .await?;
/// # Ok::<(), zenoh::Error>(())
/// ```
pub struct ActionClientBuilder<'a, A: Action> {
    /// The name of the action.
    pub action_name: String,
    /// Reference to the node that will own this client.
    pub node: &'a crate::node::Node,
    /// QoS profile for the goal service.
    pub goal_service_qos: Option<QosProfile>,
    /// QoS profile for the result service.
    pub result_service_qos: Option<QosProfile>,
    /// QoS profile for the cancel service.
    pub cancel_service_qos: Option<QosProfile>,
    /// QoS profile for the feedback topic.
    pub feedback_topic_qos: Option<QosProfile>,
    /// QoS profile for the status topic.
    pub status_topic_qos: Option<QosProfile>,
    /// Override for goal (send_goal) type info; uses `A::send_goal_type_info()` if None.
    pub goal_type_info: Option<TypeInfo>,
    /// Override for result (get_result) type info; uses `A::get_result_type_info()` if None.
    pub result_type_info: Option<TypeInfo>,
    /// Override for feedback type info; uses `A::feedback_type_info()` if None.
    pub feedback_type_info: Option<TypeInfo>,
    /// Phantom data for the action type and backend.
    pub _phantom: std::marker::PhantomData<A>,
}

impl<'a, A: Action> ActionClientBuilder<'a, A> {
    pub fn with_goal_service_qos(mut self, qos: QosProfile) -> Self {
        self.goal_service_qos = Some(qos);
        self
    }

    pub fn with_result_service_qos(mut self, qos: QosProfile) -> Self {
        self.result_service_qos = Some(qos);
        self
    }

    pub fn with_cancel_service_qos(mut self, qos: QosProfile) -> Self {
        self.cancel_service_qos = Some(qos);
        self
    }

    pub fn with_feedback_topic_qos(mut self, qos: QosProfile) -> Self {
        self.feedback_topic_qos = Some(qos);
        self
    }

    pub fn with_status_topic_qos(mut self, qos: QosProfile) -> Self {
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

impl<'a, A: Action> ActionClientBuilder<'a, A> {
    pub fn new(action_name: &str, node: &'a crate::node::Node) -> Self {
        Self {
            action_name: action_name.to_string(),
            node,
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

impl<'a, A: Action> ActionClientBuilder<'a, A> {
    pub async fn build(self) -> Result<ActionClient<A>> {
        // Apply remapping to action name
        let action_name = self.node.remap_rules.apply(&self.action_name);

        // Validate action name is not empty
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

        // Create goal client using node API for proper graph registration
        // Use override if provided, otherwise fall back to the action's static type info.
        let goal_type_info = Some(self.goal_type_info.unwrap_or_else(A::send_goal_type_info));
        let mut goal_client_builder = self
            .node
            .create_client_impl::<GoalService<A>>(&goal_service_name, goal_type_info);
        if let Some(qos) = self.goal_service_qos {
            goal_client_builder.entity.qos = qos.to_protocol_qos();
        }
        let goal_client = goal_client_builder.build().await?;

        // Create result client using node API for proper graph registration
        let result_type_info = Some(
            self.result_type_info
                .unwrap_or_else(A::get_result_type_info),
        );
        let mut result_client_builder = self
            .node
            .create_client_impl::<ResultService<A>>(&result_service_name, result_type_info);
        if let Some(qos) = self.result_service_qos {
            result_client_builder.entity.qos = qos.to_protocol_qos();
        }
        let result_client = result_client_builder.build().await?;
        tracing::debug!("Created result client for: {}", result_service_name);

        // Create cancel client using node API for proper graph registration
        // Use the native action cancel control type identity.
        let cancel_type_info = Some(A::cancel_goal_type_info());
        let mut cancel_client_builder = self
            .node
            .create_client_impl::<CancelService<A>>(&cancel_service_name, cancel_type_info);
        if let Some(qos) = self.cancel_service_qos {
            cancel_client_builder.entity.qos = qos.to_protocol_qos();
        }
        let cancel_client = cancel_client_builder.build().await?;

        let goal_board = Arc::new(GoalBoard {
            active_goals: Mutex::new(HashMap::new()),
        });

        // Create feedback subscriber for proper graph registration
        let feedback_type_info = Some(
            self.feedback_type_info
                .unwrap_or_else(A::feedback_type_info),
        );
        let mut feedback_sub_builder = self.node.subscriber_with_type_info::<FeedbackMessage<A>>(
            &feedback_topic_name,
            feedback_type_info,
        );
        if let Some(qos) = self.feedback_topic_qos {
            feedback_sub_builder.entity.qos = qos.to_protocol_qos();
        }
        tracing::debug!("Creating feedback subscriber for {}", feedback_topic_name);
        let feedback_sub = Arc::new(feedback_sub_builder.build().await?);
        let feedback_task = Arc::new(SubscriptionTask::new(tokio::spawn({
            let goal_board_feedback = goal_board.clone();
            let feedback_sub = feedback_sub.clone();
            async move {
                loop {
                    let message: FeedbackMessage<A> = match feedback_sub.recv().await {
                        Ok(message) => message,
                        Err(error) => {
                            tracing::warn!("Feedback subscriber stopped: {}", error);
                            break;
                        }
                    };
                    tracing::trace!("Feedback callback received for goal {:?}", message.goal_id);
                    let feedback_tx = goal_board_feedback
                        .active_goals
                        .lock()
                        .get(&message.goal_id)
                        .map(|channels| channels.feedback_tx.clone());
                    if let Some(feedback_tx) = feedback_tx {
                        tracing::trace!("Routing feedback to goal {:?}", message.goal_id);
                        let _ = feedback_tx.send(message.feedback);
                    } else {
                        tracing::warn!("No active goal found for feedback {:?}", message.goal_id);
                    }
                }
            }
        })));
        tracing::debug!("Feedback subscriber created successfully");

        // Create status subscriber with background routing
        // Use the native action status type identity.
        let status_type_info = Some(A::status_type_info());
        let mut status_sub_builder = self
            .node
            .subscriber_with_type_info::<StatusMessage>(&status_topic_name, status_type_info);
        if let Some(qos) = self.status_topic_qos {
            status_sub_builder.entity.qos = qos.to_protocol_qos();
        }
        let status_sub = Arc::new(status_sub_builder.build().await?);
        let status_task = Arc::new(SubscriptionTask::new(tokio::spawn({
            let goal_board_status = goal_board.clone();
            let status_sub = status_sub.clone();
            async move {
                loop {
                    let message = match status_sub.recv().await {
                        Ok(message) => message,
                        Err(error) => {
                            tracing::warn!("Status subscriber stopped: {}", error);
                            break;
                        }
                    };
                    tracing::trace!(
                        "Status callback received with {} statuses",
                        message.status_list.len()
                    );
                    for status_info in message.status_list {
                        let status_tx = goal_board_status
                            .active_goals
                            .lock()
                            .get(&status_info.goal_info.goal_id)
                            .map(|channels| channels.status_tx.clone());
                        if let Some(status_tx) = status_tx {
                            let goal_id = status_info.goal_info.goal_id;
                            let status = status_info.status;
                            tracing::trace!("Routing status {:?} to goal {:?}", status, goal_id);
                            let _ = status_tx.send(status);
                        } else {
                            tracing::trace!(
                                "No active goal found for status {:?}",
                                status_info.goal_info.goal_id
                            );
                        }
                    }
                }
            }
        })));

        Ok(ActionClient {
            action_name: qualified_action_name,
            graph: self.node.graph.clone(),
            goal_client: Arc::new(goal_client),
            result_client: Arc::new(result_client),
            cancel_client: Arc::new(cancel_client),
            feedback_sub,
            status_sub,
            _feedback_task: feedback_task,
            _status_task: status_task,
            goal_board,
            _runtime: None,
        })
    }
}
/// An action client for sending goals to an action server.
///
/// The `ActionClient` allows you to send goals, receive feedback,
/// monitor status, and request results from an action server.
///
/// # Simple Goal Send/Receive
///
/// ```ignore
/// # use ros_z::action::*;
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let node = todo!();
/// // Create a client for an action
/// let client = node.create_action_client::<MyAction>("my_action").build().await?;
///
/// // Send a goal
/// let goal_handle = client.send_goal_async(MyGoal { value: 42 }).await?;
///
/// // Wait for the result
/// let result = goal_handle.result_async().await?;
/// println!("Result: {:?}", result);
/// # Ok(())
/// # }
/// ```
///
/// # Feedback Streaming
///
/// ```ignore
/// # use ros_z::action::*;
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let node = todo!();
/// # let client = todo!();
/// # let goal_handle = todo!();
/// // Get feedback stream
/// let mut feedback_rx = goal_handle.feedback().unwrap();
///
/// // Process feedback in a separate task
/// tokio::spawn(async move {
///     while let Some(feedback) = feedback_rx.recv().await {
///         println!("Progress: {:.1}%", feedback.progress * 100.0);
///     }
/// });
/// # Ok(())
/// # }
/// ```
///
/// # Cancellation
///
/// ```ignore
/// # use ros_z::action::*;
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let node = todo!();
/// # let client = todo!();
/// # let goal_handle = todo!();
/// // Cancel a specific goal
/// client.cancel_goal_async(goal_handle.id()).await?;
///
/// // Or cancel all goals
/// client.cancel_all_goals_async().await?;
/// # Ok(())
/// # }
/// ```
pub struct ActionClient<A: Action> {
    action_name: String,
    graph: Arc<crate::graph::Graph>,
    goal_client: Arc<crate::service::ServiceClient<GoalService<A>>>,
    result_client: Arc<crate::service::ServiceClient<ResultService<A>>>,
    cancel_client: Arc<crate::service::ServiceClient<CancelService<A>>>,
    feedback_sub: Arc<
        crate::pubsub::Subscriber<FeedbackMessage<A>, <FeedbackMessage<A> as WireMessage>::Codec>,
    >,
    status_sub:
        Arc<crate::pubsub::Subscriber<StatusMessage, <StatusMessage as WireMessage>::Codec>>,
    _feedback_task: Arc<SubscriptionTask>,
    _status_task: Arc<SubscriptionTask>,
    goal_board: Arc<GoalBoard<A>>,
    _runtime: Option<Arc<tokio::runtime::Runtime>>,
}

struct SubscriptionTask {
    handle: tokio::task::JoinHandle<()>,
}

impl SubscriptionTask {
    fn new(handle: tokio::task::JoinHandle<()>) -> Self {
        Self { handle }
    }
}

impl Drop for SubscriptionTask {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl<A: Action> std::fmt::Debug for ActionClient<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionClient")
            .field("goal_client", &self.goal_client)
            .finish_non_exhaustive()
    }
}

// This saves the condition A: Clone if using #[derive(Clone)]
impl<A: Action> Clone for ActionClient<A> {
    fn clone(&self) -> Self {
        Self {
            action_name: self.action_name.clone(),
            graph: self.graph.clone(),
            goal_client: self.goal_client.clone(),
            result_client: self.result_client.clone(),
            cancel_client: self.cancel_client.clone(),
            feedback_sub: self.feedback_sub.clone(),
            status_sub: self.status_sub.clone(),
            _feedback_task: self._feedback_task.clone(),
            _status_task: self._status_task.clone(),
            goal_board: self.goal_board.clone(),
            _runtime: self._runtime.clone(),
        }
    }
}

impl<A: Action> ActionClient<A> {
    fn wait_for_terminal_status_blocking(
        status_rx: &mut watch::Receiver<GoalStatus>,
    ) -> Result<()> {
        loop {
            let status = *status_rx.borrow_and_update();
            if status.is_terminal() {
                return Ok(());
            }

            match Self::block_on(async { Ok(status_rx.changed().await) }) {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    tracing::warn!("status channel closed before reaching terminal action state");
                    return Ok(());
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn wait_for_terminal_status_async(status_rx: &mut watch::Receiver<GoalStatus>) {
        loop {
            let status = *status_rx.borrow_and_update();
            if status.is_terminal() {
                break;
            }

            if status_rx.changed().await.is_err() {
                tracing::warn!("Status channel closed before reaching terminal state");
                break;
            }
        }
    }

    fn wait_for_terminal_result_request_blocking(&self, goal_id: GoalId) -> Result<()> {
        if let Some(mut status_rx) = self.status_watch(goal_id) {
            Self::wait_for_terminal_status_blocking(&mut status_rx)?;
        }
        Ok(())
    }

    fn block_on<F, T>(future: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            match handle.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::MultiThread => {
                    tokio::task::block_in_place(|| handle.block_on(future))
                }
                tokio::runtime::RuntimeFlavor::CurrentThread => Err(zenoh::Error::from(
                    "Blocking action APIs cannot run on Tokio current_thread runtimes. Do not call them from async contexts; use the async action APIs there.",
                )),
                _ => Err(zenoh::Error::from(
                    "Blocking action APIs require a supported Tokio runtime. Use the async action APIs from async contexts.",
                )),
            }
        } else {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create Tokio runtime for blocking action call")
                .block_on(future)
        }
    }

    /// Wait until the action server is fully available.
    pub async fn wait_for_server_async(&self, timeout: std::time::Duration) -> bool {
        self.graph
            .wait_for_action_server(self.action_name.as_str(), timeout)
            .await
    }

    /// Sends a goal to the action server.
    ///
    /// This method sends a goal to the action server and returns a `GoalHandle`
    /// that can be used to monitor the goal's progress, receive feedback,
    /// and retrieve the result.
    ///
    /// # Arguments
    ///
    /// * `goal` - The goal to send to the server.
    ///
    /// # Returns
    ///
    /// Returns a `GoalHandle` if the goal is accepted, or an error if rejected.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`send_goal_async`](Self::send_goal_async) there.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use ros_z::action::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let node = todo!();
    /// // Create an action client
    /// let client = node.create_action_client::<MyAction>("my_action").build().await?;
    ///
    /// // Send a goal and get a handle to monitor it
    /// let goal_handle = client.send_goal_async(MyGoal { target: 42.0 }).await?;
    ///
    /// // The handle can be used to monitor progress, get feedback, and retrieve results
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_goal(&self, goal: A::Goal) -> Result<GoalHandle<A, goal_state::Active>> {
        let goal_id = GoalId::new();

        // 1. Create channels for this goal
        let (feedback_tx, feedback_rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = watch::channel(GoalStatus::Unknown);

        // 2. Insert into board
        self.goal_board.active_goals.lock().insert(
            goal_id,
            GoalChannels {
                feedback_tx,
                status_tx,
            },
        );

        // 3. Send goal request via service client
        let request = SendGoalRequest { goal_id, goal };
        tracing::debug!("Sending goal request for goal_id: {:?}", goal_id);
        let response = match self.goal_client.call(&request) {
            Ok(response) => response,
            Err(error) => {
                self.goal_board.active_goals.lock().remove(&goal_id);
                return Err(error);
            }
        };

        // 5. Check if accepted
        if !response.accepted {
            // Cleanup on rejection
            self.goal_board.active_goals.lock().remove(&goal_id);
            return Err(zenoh::Error::from("Goal rejected".to_string()));
        }

        // 6. Return typed handle in Active state
        Ok(GoalHandle {
            id: goal_id,
            client: Arc::new(self.clone()),
            feedback_rx: Some(feedback_rx),
            status_rx: Some(status_rx),
            _state: PhantomData,
        })
    }

    /// Sends a goal to the action server, failing if no response arrives before `timeout`.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`send_goal_with_timeout_async`](Self::send_goal_with_timeout_async) there.
    pub fn send_goal_with_timeout(
        &self,
        goal: A::Goal,
        timeout: Duration,
    ) -> Result<GoalHandle<A, goal_state::Active>> {
        let goal_id = GoalId::new();

        let (feedback_tx, feedback_rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = watch::channel(GoalStatus::Unknown);

        self.goal_board.active_goals.lock().insert(
            goal_id,
            GoalChannels {
                feedback_tx,
                status_tx,
            },
        );

        let request = SendGoalRequest { goal_id, goal };
        tracing::debug!("Sending goal request for goal_id: {:?}", goal_id);
        let response = match self.goal_client.call_with_timeout(&request, timeout) {
            Ok(response) => response,
            Err(error) => {
                self.goal_board.active_goals.lock().remove(&goal_id);
                return Err(error);
            }
        };

        if !response.accepted {
            self.goal_board.active_goals.lock().remove(&goal_id);
            return Err(zenoh::Error::from("Goal rejected".to_string()));
        }

        Ok(GoalHandle {
            id: goal_id,
            client: Arc::new(self.clone()),
            feedback_rx: Some(feedback_rx),
            status_rx: Some(status_rx),
            _state: PhantomData,
        })
    }

    pub async fn send_goal_async(
        &self,
        goal: A::Goal,
    ) -> Result<GoalHandle<A, goal_state::Active>> {
        let goal_id = GoalId::new();

        // 1. Create channels for this goal
        let (feedback_tx, feedback_rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = watch::channel(GoalStatus::Unknown);

        // 2. Insert into board
        self.goal_board.active_goals.lock().insert(
            goal_id,
            GoalChannels {
                feedback_tx,
                status_tx,
            },
        );

        // 3. Send goal request via service client
        let request = SendGoalRequest { goal_id, goal };
        tracing::debug!("Sending goal request for goal_id: {:?}", goal_id);
        let response = match self.goal_client.call_async(&request).await {
            Ok(response) => response,
            Err(error) => {
                self.goal_board.active_goals.lock().remove(&goal_id);
                return Err(error);
            }
        };

        // 5. Check if accepted
        if !response.accepted {
            // Cleanup on rejection
            self.goal_board.active_goals.lock().remove(&goal_id);
            return Err(zenoh::Error::from("Goal rejected".to_string()));
        }

        // 6. Return typed handle in Active state
        Ok(GoalHandle {
            id: goal_id,
            client: Arc::new(self.clone()),
            feedback_rx: Some(feedback_rx),
            status_rx: Some(status_rx),
            _state: PhantomData,
        })
    }

    pub async fn send_goal_with_timeout_async(
        &self,
        goal: A::Goal,
        timeout: Duration,
    ) -> Result<GoalHandle<A, goal_state::Active>> {
        let goal_id = GoalId::new();

        let (feedback_tx, feedback_rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = watch::channel(GoalStatus::Unknown);

        self.goal_board.active_goals.lock().insert(
            goal_id,
            GoalChannels {
                feedback_tx,
                status_tx,
            },
        );

        let request = SendGoalRequest { goal_id, goal };
        tracing::debug!("Sending goal request for goal_id: {:?}", goal_id);
        let response = match self
            .goal_client
            .call_with_timeout_async(&request, timeout)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.goal_board.active_goals.lock().remove(&goal_id);
                return Err(error);
            }
        };

        if !response.accepted {
            self.goal_board.active_goals.lock().remove(&goal_id);
            return Err(zenoh::Error::from("Goal rejected".to_string()));
        }

        Ok(GoalHandle {
            id: goal_id,
            client: Arc::new(self.clone()),
            feedback_rx: Some(feedback_rx),
            status_rx: Some(status_rx),
            _state: PhantomData,
        })
    }

    /// This is a blocking API. Do not call it from async contexts; use
    /// [`cancel_goal_async`](Self::cancel_goal_async) there.
    pub fn cancel_goal(&self, goal_id: GoalId) -> Result<CancelGoalServiceResponse> {
        let goal_info = GoalInfo::new(goal_id);
        let request = CancelGoalServiceRequest { goal_info };

        self.cancel_client.call(&request)
    }

    pub async fn cancel_goal_async(&self, goal_id: GoalId) -> Result<CancelGoalServiceResponse> {
        let goal_info = GoalInfo::new(goal_id);
        let request = CancelGoalServiceRequest { goal_info };

        self.cancel_client.call_async(&request).await
    }

    /// This is a blocking API. Do not call it from async contexts; use
    /// [`cancel_all_goals_async`](Self::cancel_all_goals_async) there.
    pub fn cancel_all_goals(&self) -> Result<CancelGoalServiceResponse> {
        // NOTE: zero UUID + zero timestamp means "cancel all" for ros-z actions.
        let zero_goal_id = GoalId([0u8; 16]);
        let goal_info = GoalInfo {
            goal_id: zero_goal_id,
            stamp: Time::zero(),
        };
        let request = CancelGoalServiceRequest { goal_info };

        self.cancel_client.call(&request)
    }

    pub async fn cancel_all_goals_async(&self) -> Result<CancelGoalServiceResponse> {
        // NOTE: zero UUID + zero timestamp means "cancel all" for ros-z actions.
        let zero_goal_id = GoalId([0u8; 16]);
        let goal_info = GoalInfo {
            goal_id: zero_goal_id,
            stamp: Time::zero(),
        };
        let request = CancelGoalServiceRequest { goal_info };

        self.cancel_client.call_async(&request).await
    }

    pub fn feedback_stream(&self, goal_id: GoalId) -> Option<mpsc::UnboundedReceiver<A::Feedback>> {
        self.goal_board
            .active_goals
            .lock()
            .get_mut(&goal_id)
            .map(|channels| {
                // Create new receiver (old one already taken via GoalHandle)
                let (tx, rx) = mpsc::unbounded_channel();
                channels.feedback_tx = tx;
                rx
            })
    }

    pub fn status_watch(&self, goal_id: GoalId) -> Option<watch::Receiver<GoalStatus>> {
        self.goal_board
            .active_goals
            .lock()
            .get(&goal_id)
            .map(|channels| channels.status_tx.subscribe())
    }

    /// This is a blocking API. Do not call it from async contexts; use
    /// [`get_result_async`](Self::get_result_async) there.
    pub fn get_result(&self, goal_id: GoalId) -> Result<A::Result> {
        self.wait_for_terminal_result_request_blocking(goal_id)?;

        let request = GetResultRequest { goal_id };

        let response: GetResultResponse<A> = self.result_client.call(&request)?;

        Ok(response.result)
    }

    pub async fn get_result_async(&self, goal_id: GoalId) -> Result<A::Result> {
        let request = GetResultRequest { goal_id };

        let response: GetResultResponse<A> = self.result_client.call_async(&request).await?;

        Ok(response.result)
    }
}

/// Routes action feedback and status messages to active goal handles.
struct GoalBoard<A: Action> {
    active_goals: Mutex<HashMap<GoalId, GoalChannels<A>>>,
}

struct GoalChannels<A: Action> {
    feedback_tx: mpsc::UnboundedSender<A::Feedback>,
    status_tx: watch::Sender<GoalStatus>,
}

/// Handle for monitoring and managing an active goal.
///
/// A `GoalHandle` is returned when a goal is successfully sent to an action server.
/// It provides methods to monitor the goal's status, receive feedback, retrieve results,
/// and cancel the goal.
///
/// The handle uses a type-state pattern to ensure goals cannot be misused:
/// - `GoalHandle<A, goal_state::Active>` - Can be monitored, cancelled, or consumed for result
/// - `GoalHandle<A, goal_state::Terminated>` - Read-only access after completion
///
/// # Examples
///
/// ```ignore
/// # use ros_z::action::*;
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let mut goal_handle = todo!();
/// // Monitor status
/// let mut status_watch = goal_handle.status_watch().unwrap();
/// while let Ok(()) = status_watch.changed().await {
///     println!("Status: {:?}", *status_watch.borrow());
/// }
///
/// // Get result (consumes the handle)
/// let result = goal_handle.result_async().await?;
/// # Ok(())
/// # }
/// ```
pub struct GoalHandle<A: Action, State = goal_state::Active> {
    /// Unique identifier for this goal.
    id: GoalId,
    /// Reference to the client that sent this goal.
    client: Arc<ActionClient<A>>,
    /// Receiver for feedback messages.
    feedback_rx: Option<mpsc::UnboundedReceiver<A::Feedback>>,
    /// Receiver for status updates.
    status_rx: Option<watch::Receiver<GoalStatus>>,
    /// Type-state marker
    _state: PhantomData<State>,
}

// --- Active State Methods ---
impl<A: Action> GoalHandle<A, goal_state::Active> {
    /// Returns the unique identifier for this goal.
    ///
    /// # Returns
    ///
    /// The `GoalId` assigned to this goal when it was sent.
    pub fn id(&self) -> GoalId {
        self.id
    }

    /// Takes ownership of the feedback receiver.
    ///
    /// Returns `Some` the first time it's called, `None` afterwards.
    pub fn feedback(&mut self) -> Option<mpsc::UnboundedReceiver<A::Feedback>> {
        self.feedback_rx.take()
    }

    /// Takes ownership of the status watcher.
    ///
    /// Returns `Some` the first time it's called, `None` afterwards.
    pub fn status_watch(&mut self) -> Option<watch::Receiver<GoalStatus>> {
        self.status_rx.take()
    }

    /// Requests cancellation of this goal.
    ///
    /// # Returns
    ///
    /// The cancellation response from the server.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`cancel_async`](Self::cancel_async) there.
    pub fn cancel(&self) -> Result<CancelGoalServiceResponse> {
        self.client.cancel_goal(self.id)
    }

    /// Requests cancellation of this goal asynchronously.
    pub async fn cancel_async(&self) -> Result<CancelGoalServiceResponse> {
        self.client.cancel_goal_async(self.id).await
    }

    /// Consumes the Active handle to prevent reuse.
    ///
    /// Waits for the handle's status stream to report a terminal state, then
    /// sends the blocking result request and cleans up the goal from the board.
    /// This is crucial for memory safety.
    ///
    /// # Returns
    ///
    /// The result of the action once it completes.
    ///
    /// This is a blocking API. Do not call it from async contexts; use
    /// [`result_async`](Self::result_async) there.
    pub fn result(mut self) -> Result<A::Result> {
        if let Some(mut rx) = self.status_rx.take()
            && let Err(error) = ActionClient::<A>::wait_for_terminal_status_blocking(&mut rx)
        {
            self.client.goal_board.active_goals.lock().remove(&self.id);
            return Err(error);
        }

        let res = self.client.get_result(self.id);

        self.client.goal_board.active_goals.lock().remove(&self.id);

        res
    }

    /// Consumes the Active handle to prevent reuse.
    ///
    /// Waits for the goal to reach a terminal state, fetches the result,
    /// and cleans up the goal from the board. This is crucial for memory safety.
    ///
    /// # Returns
    ///
    /// The result of the action once it completes.
    pub async fn result_async(mut self) -> Result<A::Result> {
        // 1. Wait for Terminal Status
        if let Some(mut rx) = self.status_rx.take() {
            ActionClient::<A>::wait_for_terminal_status_async(&mut rx).await;
        }

        // 2. Fetch Result
        // The server's get_result handler will either:
        // - Return immediately if the goal is already terminated
        // - Register a future and wait for termination
        // This eliminates the need for the sleep workaround
        let res = self.client.get_result_async(self.id).await;

        // 3. Cleanup Board (Crucial for Memory Safety)
        self.client.goal_board.active_goals.lock().remove(&self.id);

        res
    }
}

#[cfg(test)]
mod tests {
    use std::{marker::PhantomData, sync::Arc};

    use serde::{Deserialize, Serialize};
    use tokio::sync::{mpsc, watch};

    use super::{ActionClient, GoalChannels, GoalHandle, goal_state};
    use crate::{action::GoalStatus, context::ContextBuilder, define_action};

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
    fn wait_for_terminal_status_blocking_wakes_on_status_change() {
        let (tx, mut rx) = tokio::sync::watch::channel(GoalStatus::Executing);
        let handle = std::thread::spawn(move || {
            ActionClient::<TestAction>::wait_for_terminal_status_blocking(&mut rx).unwrap();
        });

        tx.send(GoalStatus::Succeeded).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn wait_for_terminal_status_blocking_rejects_current_thread_runtime() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let error = runtime.block_on(async {
            let (_tx, mut rx) = tokio::sync::watch::channel(GoalStatus::Executing);
            ActionClient::<TestAction>::wait_for_terminal_status_blocking(&mut rx)
                .expect_err("current-thread runtime should be rejected")
        });

        assert!(error.to_string().contains("current_thread"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn goal_handle_result_cleans_up_on_status_wait_runtime_guard_error() -> zenoh::Result<()>
    {
        let context = ContextBuilder::default().build().await?;
        let node = context
            .create_node("result_guard_cleanup_test")
            .build()
            .await?;
        let client = Arc::new(
            node.create_action_client::<TestAction>("result_guard_cleanup_action")
                .build()
                .await?,
        );
        let goal_id = crate::action::GoalId::new();
        let (feedback_tx, feedback_rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = watch::channel(GoalStatus::Executing);
        client.goal_board.active_goals.lock().insert(
            goal_id,
            GoalChannels {
                feedback_tx,
                status_tx,
            },
        );
        let handle = GoalHandle {
            id: goal_id,
            client: client.clone(),
            feedback_rx: Some(feedback_rx),
            status_rx: Some(status_rx),
            _state: PhantomData::<goal_state::Active>,
        };

        let error = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                handle
                    .result()
                    .expect_err("current-thread runtime should be rejected")
            })
        })
        .join()
        .expect("current-thread runtime guard test panicked");

        assert!(error.to_string().contains("current_thread"));
        assert!(!client.goal_board.active_goals.lock().contains_key(&goal_id));
        Ok(())
    }
}

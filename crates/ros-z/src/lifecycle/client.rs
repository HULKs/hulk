//! Client for driving remote lifecycle nodes through their state transitions.
//!
//! `LifecycleClient` connects to a lifecycle node's management services and
//! allows you to orchestrate its state machine from Rust code — the building
//! block for bringup managers and system orchestrators.
//!
//! # Example
//!
//! ```rust,ignore
//! let context = ContextBuilder::default().build().await?;
//! let mgr = context.create_node("lifecycle_manager").build().await?;
//!
//! let client = LifecycleClient::new(&mgr, "/camera_node").await?;
//!
//! client.configure(Duration::from_secs(5)).await?;
//! client.activate(Duration::from_secs(5)).await?;
//!
//! // … node is now Active and publishing …
//!
//! client.deactivate(Duration::from_secs(5)).await?;
//! client.shutdown(Duration::from_secs(5)).await?;
//! ```

use std::time::Duration;

use crate::{
    Result,
    lifecycle::{
        LifecycleState, TransitionId,
        msgs::{
            ChangeState, ChangeStateRequest, GetAvailableStates, GetAvailableStatesRequest,
            GetAvailableTransitions, GetAvailableTransitionsRequest, GetState, GetStateRequest,
            LcState, LcTransitionDescription,
        },
    },
    node::Node,
    service::ServiceClient,
    topic_name::qualify_remote_private_service_name,
};

/// Client for managing a remote lifecycle node's state machine.
///
/// Each method maps to one of the native lifecycle management services.
/// The target passed to [`LifecycleClient::new`] must be the lifecycle node's
/// absolute fully-qualified name, including namespace.
pub struct LifecycleClient {
    change_state: ServiceClient<ChangeState>,
    get_state: ServiceClient<GetState>,
    get_available_states: ServiceClient<GetAvailableStates>,
    get_available_transitions: ServiceClient<GetAvailableTransitions>,
}

impl LifecycleClient {
    /// Create a new lifecycle client targeting `node_name`.
    ///
    /// `node_name` must be an absolute node FQN like `/camera_node` or
    /// `/tools/camera_node`.
    pub async fn new(node: &Node, node_name: &str) -> Result<Self> {
        let (target_namespace, target_node_name) = parse_absolute_node_fqn(node_name)?;
        let change_state = node
            .create_service_client::<ChangeState>(&qualify_remote_private_service_name(
                "_ros_z_lifecycle/change_state",
                &target_namespace,
                &target_node_name,
            )?)
            .build()
            .await?;
        let get_state = node
            .create_service_client::<GetState>(&qualify_remote_private_service_name(
                "_ros_z_lifecycle/get_state",
                &target_namespace,
                &target_node_name,
            )?)
            .build()
            .await?;
        let get_available_states = node
            .create_service_client::<GetAvailableStates>(&qualify_remote_private_service_name(
                "_ros_z_lifecycle/get_available_states",
                &target_namespace,
                &target_node_name,
            )?)
            .build()
            .await?;
        let get_available_transitions = node
            .create_service_client::<GetAvailableTransitions>(&qualify_remote_private_service_name(
                "_ros_z_lifecycle/get_available_transitions",
                &target_namespace,
                &target_node_name,
            )?)
            .build()
            .await?;
        Ok(Self {
            change_state,
            get_state,
            get_available_states,
            get_available_transitions,
        })
    }

    // -----------------------------------------------------------------------
    // High-level transition helpers
    // -----------------------------------------------------------------------

    /// Trigger the `configure` transition (Unconfigured → Inactive).
    pub async fn configure(&self, timeout: Duration) -> Result<bool> {
        self.trigger(TransitionId::Configure, timeout).await
    }

    /// Trigger the `activate` transition (Inactive → Active).
    pub async fn activate(&self, timeout: Duration) -> Result<bool> {
        self.trigger(TransitionId::Activate, timeout).await
    }

    /// Trigger the `deactivate` transition (Active → Inactive).
    pub async fn deactivate(&self, timeout: Duration) -> Result<bool> {
        self.trigger(TransitionId::Deactivate, timeout).await
    }

    /// Trigger the `cleanup` transition (Inactive → Unconfigured).
    pub async fn cleanup(&self, timeout: Duration) -> Result<bool> {
        self.trigger(TransitionId::Cleanup, timeout).await
    }

    /// Trigger `shutdown` from any primary state (→ Finalized).
    ///
    /// Sends the appropriate shutdown transition ID for the node's current
    /// state. Returns `Ok(true)` if the node acknowledged the transition.
    pub async fn shutdown(&self, timeout: Duration) -> Result<bool> {
        let transition_id = match self.get_state(timeout).await? {
            LifecycleState::Unconfigured => TransitionId::UnconfiguredShutdown,
            LifecycleState::Active => TransitionId::ActiveShutdown,
            _ => TransitionId::InactiveShutdown,
        };
        self.trigger(transition_id, timeout).await
    }

    // -----------------------------------------------------------------------
    // Low-level transition trigger
    // -----------------------------------------------------------------------

    /// Trigger an arbitrary transition by [`TransitionId`].
    ///
    /// Returns `Ok(true)` when the lifecycle node accepted the transition.
    pub async fn trigger(&self, transition: TransitionId, timeout: Duration) -> Result<bool> {
        let req = ChangeStateRequest {
            transition: crate::lifecycle::msgs::LcTransition {
                id: transition as u8,
                label: String::new(),
            },
        };
        let resp = self
            .change_state
            .call_with_timeout_async(&req, timeout)
            .await?;
        Ok(resp.success)
    }

    // -----------------------------------------------------------------------
    // Query services
    // -----------------------------------------------------------------------

    /// Query the current state of the remote lifecycle node.
    pub async fn get_state(&self, timeout: Duration) -> Result<LifecycleState> {
        let resp = self
            .get_state
            .call_with_timeout_async(&GetStateRequest {}, timeout)
            .await?;
        Ok(state_from_lc(&resp.current_state))
    }

    /// List all states in the lifecycle state machine.
    pub async fn get_available_states(&self, timeout: Duration) -> Result<Vec<LcState>> {
        let resp = self
            .get_available_states
            .call_with_timeout_async(&GetAvailableStatesRequest {}, timeout)
            .await?;
        Ok(resp.available_states)
    }

    /// List transitions valid from the node's current state.
    pub async fn get_available_transitions(
        &self,
        timeout: Duration,
    ) -> Result<Vec<LcTransitionDescription>> {
        let resp = self
            .get_available_transitions
            .call_with_timeout_async(&GetAvailableTransitionsRequest {}, timeout)
            .await?;
        Ok(resp.available_transitions)
    }
}

fn parse_absolute_node_fqn(node_name: &str) -> Result<(String, String)> {
    let node_name = node_name.trim();
    if !node_name.starts_with('/') {
        return Err(zenoh::Error::from(format!(
            "Lifecycle client target must be an absolute node FQN, got '{node_name}'"
        )));
    }

    let mut parts = node_name.rsplitn(2, '/');
    let name = parts.next().unwrap_or_default();
    let namespace = parts.next().unwrap_or_default();
    if name.is_empty() {
        return Err(zenoh::Error::from(format!(
            "Lifecycle client target must include a node name, got '{node_name}'"
        )));
    }

    let namespace = if namespace.is_empty() {
        "/".to_string()
    } else {
        namespace.to_string()
    };

    qualify_remote_private_service_name("", &namespace, name)
        .map_err(|e| zenoh::Error::from(format!("Invalid lifecycle client target: {e}")))?;

    Ok((namespace, name.to_string()))
}

fn state_from_lc(s: &LcState) -> LifecycleState {
    match s.id {
        1 => LifecycleState::Unconfigured,
        2 => LifecycleState::Inactive,
        3 => LifecycleState::Active,
        4 => LifecycleState::Finalized,
        _ => LifecycleState::Unconfigured,
    }
}

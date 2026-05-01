use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicBool, Ordering},
};

use parking_lot::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use zenoh::Result;

use crate::{
    Message, ServiceTypeInfo,
    context::Context,
    lifecycle::{
        msgs::{
            ChangeState, ChangeStateResponse, GetAvailableStates, GetAvailableStatesResponse,
            GetAvailableTransitions, GetAvailableTransitionsResponse, GetState, GetStateResponse,
            LcState, LcTime, LcTransition, LcTransitionDescription, LcTransitionEvent,
        },
        publisher::{LifecyclePublisher, ManagedEntity},
        state_machine::{CallbackReturn, State, StateMachine, TransitionId, TransitionResult},
    },
    msg::{GeneratedCdrCodec, WireMessage},
    node::Node,
};

/// A ros-z lifecycle-aware node.
///
/// Wraps a [`Node`] and adds the full lifecycle state machine:
/// - lifecycle management services under `~/_ros_z_lifecycle/*`
/// - `~/_ros_z_lifecycle/transition_event` publisher
/// - Lifecycle publisher factory ([`LifecycleNode::create_publisher`]) whose `publish()` calls
///   are gated on the node's activation state
///
/// # Setting lifecycle callbacks
///
/// Set callbacks after building the node:
///
/// ```no_run
/// use ros_z::lifecycle::CallbackReturn;
/// use ros_z::prelude::*;
///
/// # #[tokio::main]
/// # async fn main() -> zenoh::Result<()> {
/// let context = ContextBuilder::default().build().await?;
/// let mut node = context.create_lifecycle_node("talker").build().await?;
/// node.set_on_configure(|_prev| {
///     println!("configuring!");
///     CallbackReturn::Success
/// });
/// node.configure().await?;
/// # Ok(())
/// # }
/// ```
pub struct LifecycleNode {
    pub inner: Node,
    state_machine: Arc<Mutex<StateMachine>>,
    managed_entities: Arc<Mutex<Vec<Arc<dyn ManagedEntity>>>>,
    callbacks: LifecycleCallbacks,
    transition_in_progress: Arc<AtomicBool>,

    service_tasks: Vec<JoinHandle<()>>,

    // Transition-event publisher (Arc so trigger_transition can publish)
    te_pub: Arc<crate::pubsub::Publisher<LcTransitionEvent, GeneratedCdrCodec<LcTransitionEvent>>>,
}

type LifecycleCallback = Arc<dyn Fn(State) -> CallbackReturn + Send + Sync>;
type LifecycleCallbackSlot = Arc<RwLock<LifecycleCallback>>;

#[derive(Clone)]
struct LifecycleCallbacks {
    on_configure: LifecycleCallbackSlot,
    on_activate: LifecycleCallbackSlot,
    on_deactivate: LifecycleCallbackSlot,
    on_cleanup: LifecycleCallbackSlot,
    on_shutdown: LifecycleCallbackSlot,
    on_error: LifecycleCallbackSlot,
}

impl LifecycleCallbacks {
    fn new() -> Self {
        Self {
            on_configure: Arc::new(RwLock::new(Arc::new(|_| CallbackReturn::Success))),
            on_activate: Arc::new(RwLock::new(Arc::new(|_| CallbackReturn::Success))),
            on_deactivate: Arc::new(RwLock::new(Arc::new(|_| CallbackReturn::Success))),
            on_cleanup: Arc::new(RwLock::new(Arc::new(|_| CallbackReturn::Success))),
            on_shutdown: Arc::new(RwLock::new(Arc::new(|_| CallbackReturn::Success))),
            on_error: Arc::new(RwLock::new(Arc::new(|_| CallbackReturn::Failure))),
        }
    }

    fn callback(&self, transition: TransitionId) -> LifecycleCallback {
        match transition {
            TransitionId::Configure => &self.on_configure,
            TransitionId::Activate => &self.on_activate,
            TransitionId::Deactivate => &self.on_deactivate,
            TransitionId::Cleanup => &self.on_cleanup,
            TransitionId::UnconfiguredShutdown
            | TransitionId::InactiveShutdown
            | TransitionId::ActiveShutdown => &self.on_shutdown,
        }
        .read()
        .clone()
    }

    fn error_callback(&self) -> LifecycleCallback {
        self.on_error.read().clone()
    }
}

enum TransitionSelection {
    Direct(TransitionId),
    Remote { id: u8, label: String },
}

struct TransitionInProgressGuard {
    in_progress: Arc<AtomicBool>,
}

impl TransitionInProgressGuard {
    fn try_new(in_progress: &Arc<AtomicBool>) -> Option<Self> {
        in_progress
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()?;
        Some(Self {
            in_progress: in_progress.clone(),
        })
    }
}

impl Drop for TransitionInProgressGuard {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::Release);
    }
}

struct LifecycleTransitionOutcome {
    final_state: State,
    success: bool,
    invalid_transition: Option<(TransitionId, State)>,
    transition_in_progress: bool,
}

struct LifecycleTransitionContext<'a> {
    state_machine: &'a Arc<Mutex<StateMachine>>,
    managed_entities: &'a Arc<Mutex<Vec<Arc<dyn ManagedEntity>>>>,
    callbacks: &'a LifecycleCallbacks,
    transition_in_progress: &'a Arc<AtomicBool>,
    te_pub:
        &'a Arc<crate::pubsub::Publisher<LcTransitionEvent, GeneratedCdrCodec<LcTransitionEvent>>>,
    clock: &'a crate::time::Clock,
    node_name: &'a str,
}

struct ServiceTaskGuard {
    tasks: Vec<JoinHandle<()>>,
}

impl ServiceTaskGuard {
    fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    fn push(&mut self, task: JoinHandle<()>) {
        self.tasks.push(task);
    }

    fn into_tasks(mut self) -> Vec<JoinHandle<()>> {
        std::mem::take(&mut self.tasks)
    }
}

impl Drop for ServiceTaskGuard {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

impl std::fmt::Debug for LifecycleNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LifecycleNode")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl Drop for LifecycleNode {
    fn drop(&mut self) {
        for task in &self.service_tasks {
            task.abort();
        }
    }
}

impl LifecycleNode {
    /// The current lifecycle state.
    pub fn get_current_state(&self) -> State {
        lock_state_machine(&self.state_machine).current_state()
    }

    /// Set the callback invoked when the node transitions to Inactive from Unconfigured.
    pub fn set_on_configure<F>(&self, callback: F)
    where
        F: Fn(State) -> CallbackReturn + Send + Sync + 'static,
    {
        *self.callbacks.on_configure.write() = Arc::new(callback);
    }

    /// Set the callback invoked when the node transitions to Active.
    pub fn set_on_activate<F>(&self, callback: F)
    where
        F: Fn(State) -> CallbackReturn + Send + Sync + 'static,
    {
        *self.callbacks.on_activate.write() = Arc::new(callback);
    }

    /// Set the callback invoked when the node transitions from Active to Inactive.
    pub fn set_on_deactivate<F>(&self, callback: F)
    where
        F: Fn(State) -> CallbackReturn + Send + Sync + 'static,
    {
        *self.callbacks.on_deactivate.write() = Arc::new(callback);
    }

    /// Set the callback invoked when the node transitions from Inactive to Unconfigured.
    pub fn set_on_cleanup<F>(&self, callback: F)
    where
        F: Fn(State) -> CallbackReturn + Send + Sync + 'static,
    {
        *self.callbacks.on_cleanup.write() = Arc::new(callback);
    }

    /// Set the callback invoked when the node shuts down from any primary state.
    pub fn set_on_shutdown<F>(&self, callback: F)
    where
        F: Fn(State) -> CallbackReturn + Send + Sync + 'static,
    {
        *self.callbacks.on_shutdown.write() = Arc::new(callback);
    }

    /// Set the callback invoked when a lifecycle transition returns `Error`.
    pub fn set_on_error<F>(&self, callback: F)
    where
        F: Fn(State) -> CallbackReturn + Send + Sync + 'static,
    {
        *self.callbacks.on_error.write() = Arc::new(callback);
    }

    /// Trigger the `configure` transition.
    pub async fn configure(&mut self) -> Result<State> {
        self.trigger_transition(TransitionId::Configure).await
    }

    /// Trigger the `activate` transition.
    pub async fn activate(&mut self) -> Result<State> {
        self.trigger_transition(TransitionId::Activate).await
    }

    /// Trigger the `deactivate` transition.
    pub async fn deactivate(&mut self) -> Result<State> {
        self.trigger_transition(TransitionId::Deactivate).await
    }

    /// Trigger the `cleanup` transition.
    pub async fn cleanup(&mut self) -> Result<State> {
        self.trigger_transition(TransitionId::Cleanup).await
    }

    /// Trigger the `shutdown` transition from the current primary state.
    pub async fn shutdown(&mut self) -> Result<State> {
        let current = self.get_current_state();
        match TransitionId::shutdown_for(current) {
            Some(t) => self.trigger_transition(t).await,
            None => Err(zenoh::Error::from(format!(
                "invalid lifecycle transition Shutdown from {current:?}"
            ))),
        }
    }

    /// Trigger a specific lifecycle transition.
    pub async fn trigger_transition(&mut self, transition: TransitionId) -> Result<State> {
        let outcome = execute_lifecycle_transition(
            LifecycleTransitionContext {
                state_machine: &self.state_machine,
                managed_entities: &self.managed_entities,
                callbacks: &self.callbacks,
                transition_in_progress: &self.transition_in_progress,
                te_pub: &self.te_pub,
                clock: self.inner.clock(),
                node_name: &self.inner.entity.name,
            },
            TransitionSelection::Direct(transition),
        )
        .await;
        if let Some((transition, from)) = outcome.invalid_transition {
            return Err(zenoh::Error::from(format!(
                "invalid lifecycle transition {transition:?} from {from:?}"
            )));
        }
        if outcome.transition_in_progress {
            return Err(zenoh::Error::from(
                "lifecycle transition already in progress",
            ));
        }
        Ok(outcome.final_state)
    }

    /// Create a lifecycle-gated publisher registered as a managed entity.
    pub async fn create_publisher<T>(
        &self,
        topic: &str,
    ) -> Result<Arc<LifecyclePublisher<T, <T as crate::Message>::Codec>>>
    where
        T: crate::Message + WireMessage + Message + serde::Serialize,
        <T as crate::Message>::Codec: for<'a> crate::msg::WireEncoder<Input<'a> = &'a T>
            + Send
            + Sync
            + crate::msg::MessageCodec<T>,
    {
        let inner = self.inner.publisher::<T>(topic).build().await?;
        let lc_pub = LifecyclePublisher::new(inner);
        let mut managed_entities = lock_managed_entities(&self.managed_entities);
        if self.get_current_state() == State::Active {
            lc_pub.on_activate();
        }
        managed_entities.push(lc_pub.clone() as Arc<dyn ManagedEntity>);
        Ok(lc_pub)
    }
}

async fn execute_lifecycle_transition(
    context: LifecycleTransitionContext<'_>,
    selection: TransitionSelection,
) -> LifecycleTransitionOutcome {
    let Some(_guard) = TransitionInProgressGuard::try_new(context.transition_in_progress) else {
        return LifecycleTransitionOutcome {
            final_state: lock_state_machine(context.state_machine).current_state(),
            success: false,
            invalid_transition: None,
            transition_in_progress: true,
        };
    };

    let transition_result = {
        let mut state_machine = lock_state_machine(context.state_machine);
        let start = state_machine.current_state();
        let transition = match &selection {
            TransitionSelection::Direct(transition) => Some(*transition),
            TransitionSelection::Remote { id, label } => {
                if !label.is_empty() {
                    TransitionId::from_label_and_state(label, start)
                } else {
                    TransitionId::from_id_and_state(*id, start)
                }
            }
        };
        let transition_result = match transition {
            Some(transition) => match state_machine.begin_transition(transition) {
                Some(start) => TransitionResult::Complete(start),
                None => TransitionResult::Invalid {
                    transition,
                    from: start,
                },
            },
            None => TransitionResult::Invalid {
                transition: match selection {
                    TransitionSelection::Direct(transition) => transition,
                    TransitionSelection::Remote { id, .. } => {
                        TransitionId::from_id_and_state(id, start)
                            .unwrap_or(TransitionId::Configure)
                    }
                },
                from: start,
            },
        };
        (transition_result, transition)
    };

    let (transition_result, selected_transition) = transition_result;
    let start = match transition_result {
        TransitionResult::Complete(start) => start,
        TransitionResult::Invalid { transition, from } => {
            if selected_transition.is_some() {
                warn!("invalid lifecycle transition {transition:?} from {from:?}");
            } else {
                warn!("invalid lifecycle transition request from {from:?}");
            }
            return LifecycleTransitionOutcome {
                final_state: from,
                success: false,
                invalid_transition: selected_transition.map(|_| (transition, from)),
                transition_in_progress: false,
            };
        }
    };
    let transition = selected_transition.expect("valid lifecycle transition selected");

    debug!(node=%context.node_name, ?transition, ?start, "triggering lifecycle transition");

    let callback = context.callbacks.callback(transition);
    let callback_result = callback(start);
    let callback_state = lock_state_machine(context.state_machine).complete_transition(
        transition,
        start,
        callback_result,
    );

    let final_state = if callback_state == State::ErrorProcessing {
        let error_callback = context.callbacks.error_callback();
        let error_result = error_callback(State::ErrorProcessing);
        lock_state_machine(context.state_machine).trigger_error_processing(|_| error_result)
    } else {
        callback_state
    };

    update_managed_entities(context.managed_entities, start, final_state);

    let te = make_transition_event(
        transition,
        start,
        final_state,
        to_lc_time(context.clock.now()),
    );
    let te_pub = context.te_pub.clone();
    let publish_task = tokio::spawn(async move { te_pub.publish(&te).await });
    match publish_task.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!("failed to publish transition_event: {error}"),
        Err(error) => warn!("transition_event publish task failed: {error}"),
    }

    info!(node=%context.node_name, ?final_state, "lifecycle transition complete");
    LifecycleTransitionOutcome {
        final_state,
        success: callback_result == CallbackReturn::Success,
        invalid_transition: None,
        transition_in_progress: false,
    }
}

fn update_managed_entities(
    managed_entities: &Arc<Mutex<Vec<Arc<dyn ManagedEntity>>>>,
    start: State,
    final_state: State,
) {
    match (start, final_state) {
        (_, State::Active) if start != State::Active => {
            for entity in lock_managed_entities(managed_entities).iter() {
                entity.on_activate();
            }
        }
        (State::Active, _) if final_state != State::Active => {
            for entity in lock_managed_entities(managed_entities).iter() {
                entity.on_deactivate();
            }
        }
        _ => {}
    }
}

fn lock_state_machine(state_machine: &Arc<Mutex<StateMachine>>) -> MutexGuard<'_, StateMachine> {
    state_machine.lock().unwrap_or_else(|poisoned| {
        warn!("lifecycle state machine lock poisoned; recovering inner state");
        poisoned.into_inner()
    })
}

fn lock_managed_entities(
    managed_entities: &Arc<Mutex<Vec<Arc<dyn ManagedEntity>>>>,
) -> MutexGuard<'_, Vec<Arc<dyn ManagedEntity>>> {
    managed_entities.lock().unwrap_or_else(|poisoned| {
        warn!("lifecycle managed entities lock poisoned; recovering inner state");
        poisoned.into_inner()
    })
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

pub struct LifecycleNodeBuilder {
    pub(crate) context: Context,
    pub(crate) name: String,
    pub(crate) namespace: Option<String>,
    pub enable_communication_interface: bool,
    pub disable_schema_service: bool,
}

impl LifecycleNodeBuilder {
    pub fn with_namespace<S: Into<String>>(mut self, ns: S) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn disable_communication_interface(mut self) -> Self {
        self.enable_communication_interface = false;
        self
    }

    pub fn without_schema_service(mut self) -> Self {
        self.disable_schema_service = true;
        self
    }
}

impl LifecycleNodeBuilder {
    pub async fn build(self) -> Result<LifecycleNode> {
        let mut node_builder = self.context.create_node(&self.name);
        if let Some(ns) = self.namespace {
            node_builder = node_builder.with_namespace(ns);
        }
        if self.disable_schema_service {
            node_builder = node_builder.without_schema_service();
        }
        let inner = node_builder.build().await?;

        // Shared state machine for service closures
        let sm = Arc::new(Mutex::new(StateMachine::new()));
        let managed_entities = Arc::new(Mutex::new(Vec::new()));
        let callbacks = LifecycleCallbacks::new();
        let transition_in_progress = Arc::new(AtomicBool::new(false));

        // Transition-event publisher
        let te_pub = Arc::new(
            inner
                .publisher::<LcTransitionEvent>("~/_ros_z_lifecycle/transition_event")
                .codec::<GeneratedCdrCodec<LcTransitionEvent>>()
                .build()
                .await?,
        );

        // --- change_state ---
        let sm_cs = sm.clone();
        let te_cs = te_pub.clone();
        let callbacks_cs = callbacks.clone();
        let managed_entities_cs = managed_entities.clone();
        let transition_in_progress_cs = transition_in_progress.clone();
        let clock_cs = inner.clock().clone();
        let node_name_cs = inner.entity.name.clone();
        let srv_change_state = inner
            .create_service_impl::<ChangeState>(
                "~/_ros_z_lifecycle/change_state",
                Some(ChangeState::service_type_info()),
            )
            .build()
            .await?;
        let mut service_task_guard = ServiceTaskGuard::new();
        service_task_guard.push(tokio::spawn(async move {
            let mut srv_change_state = srv_change_state;
            loop {
                let request = match srv_change_state.take_request_async().await {
                    Ok(request) => request,
                    Err(error) => {
                        warn!("failed to receive lifecycle change_state request: {error}");
                        continue;
                    }
                };
                let (req, reply) = request.into_parts();
                let label = req.transition.label.clone();
                let tid = req.transition.id;
                let success = execute_lifecycle_transition(
                    LifecycleTransitionContext {
                        state_machine: &sm_cs,
                        managed_entities: &managed_entities_cs,
                        callbacks: &callbacks_cs,
                        transition_in_progress: &transition_in_progress_cs,
                        te_pub: &te_cs,
                        clock: &clock_cs,
                        node_name: &node_name_cs,
                    },
                    TransitionSelection::Remote { id: tid, label },
                )
                .await
                .success;
                if let Err(error) = reply.reply_async(&ChangeStateResponse { success }).await {
                    warn!("failed to send lifecycle change_state reply: {error}");
                }
            }
        }));

        // --- get_state ---
        let sm_gs = sm.clone();
        let srv_get_state = inner
            .create_service_impl::<GetState>(
                "~/_ros_z_lifecycle/get_state",
                Some(GetState::service_type_info()),
            )
            .build()
            .await?;
        service_task_guard.push(tokio::spawn(async move {
            let mut srv_get_state = srv_get_state;
            loop {
                let request = match srv_get_state.take_request_async().await {
                    Ok(request) => request,
                    Err(error) => {
                        warn!("failed to receive lifecycle get_state request: {error}");
                        continue;
                    }
                };
                let s = lock_state_machine(&sm_gs).current_state();
                if let Err(error) = request
                    .reply_async(&GetStateResponse {
                        current_state: to_lc_state(s),
                    })
                    .await
                {
                    warn!("failed to send lifecycle get_state reply: {error}");
                }
            }
        }));

        // --- get_available_states ---
        let srv_get_available_states = inner
            .create_service_impl::<GetAvailableStates>(
                "~/_ros_z_lifecycle/get_available_states",
                Some(GetAvailableStates::service_type_info()),
            )
            .build()
            .await?;
        service_task_guard.push(tokio::spawn(async move {
            let mut srv_get_available_states = srv_get_available_states;
            loop {
                let request = match srv_get_available_states.take_request_async().await {
                    Ok(request) => request,
                    Err(error) => {
                        warn!("failed to receive lifecycle get_available_states request: {error}");
                        continue;
                    }
                };
                let available_states = StateMachine::all_states()
                    .iter()
                    .map(|(id, lbl)| LcState {
                        id: *id,
                        label: lbl.to_string(),
                    })
                    .collect();
                if let Err(error) = request
                    .reply_async(&GetAvailableStatesResponse { available_states })
                    .await
                {
                    warn!("failed to send lifecycle get_available_states reply: {error}");
                }
            }
        }));

        // --- get_available_transitions ---
        let sm_gat = sm.clone();
        let srv_get_available_transitions = inner
            .create_service_impl::<GetAvailableTransitions>(
                "~/_ros_z_lifecycle/get_available_transitions",
                Some(GetAvailableTransitions::service_type_info()),
            )
            .build()
            .await?;
        service_task_guard.push(tokio::spawn(async move {
            let mut srv_get_available_transitions = srv_get_available_transitions;
            loop {
                let request = match srv_get_available_transitions.take_request_async().await {
                    Ok(request) => request,
                    Err(error) => {
                        warn!(
                            "failed to receive lifecycle get_available_transitions request: {error}"
                        );
                        continue;
                    }
                };
                let available_transitions = lock_state_machine(&sm_gat)
                    .available_transitions()
                    .into_iter()
                    .map(|(t, s, g)| to_lc_td(t, s, g))
                    .collect();
                if let Err(error) = request
                    .reply_async(&GetAvailableTransitionsResponse {
                        available_transitions,
                    })
                    .await
                {
                    warn!("failed to send lifecycle get_available_transitions reply: {error}");
                }
            }
        }));

        // --- get_transition_graph ---
        let srv_get_transition_graph = inner
            .create_service_impl::<GetAvailableTransitions>(
                "~/_ros_z_lifecycle/get_transition_graph",
                Some(GetAvailableTransitions::service_type_info()),
            )
            .build()
            .await?;
        service_task_guard.push(tokio::spawn(async move {
            let mut srv_get_transition_graph = srv_get_transition_graph;
            loop {
                let request = match srv_get_transition_graph.take_request_async().await {
                    Ok(request) => request,
                    Err(error) => {
                        warn!("failed to receive lifecycle get_transition_graph request: {error}");
                        continue;
                    }
                };
                let available_transitions = StateMachine::all_transitions()
                    .into_iter()
                    .map(|(t, s, g)| to_lc_td(t, s, g))
                    .collect();
                if let Err(error) = request
                    .reply_async(&GetAvailableTransitionsResponse {
                        available_transitions,
                    })
                    .await
                {
                    warn!("failed to send lifecycle get_transition_graph reply: {error}");
                }
            }
        }));

        Ok(LifecycleNode {
            inner,
            state_machine: sm,
            managed_entities,
            callbacks,
            transition_in_progress,
            service_tasks: service_task_guard.into_tasks(),
            te_pub,
        })
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

fn to_lc_state(s: State) -> LcState {
    LcState {
        id: s.id(),
        label: s.label().to_string(),
    }
}

fn to_lc_transition(t: TransitionId) -> LcTransition {
    LcTransition {
        id: t.id(),
        label: t.label().to_string(),
    }
}

fn to_lc_td(t: TransitionId, start: State, goal: State) -> LcTransitionDescription {
    LcTransitionDescription {
        transition: to_lc_transition(t),
        start_state: to_lc_state(start),
        goal_state: to_lc_state(goal),
    }
}

fn to_lc_time(time: crate::time::Time) -> LcTime {
    let max_nanos = i32::MAX as i64 * 1_000_000_000 + 999_999_999;
    let nanos = time.as_nanos().clamp(0, max_nanos);
    LcTime {
        sec: (nanos / 1_000_000_000) as i32,
        nanosec: (nanos % 1_000_000_000) as u32,
    }
}

fn make_transition_event(
    t: TransitionId,
    start: State,
    goal: State,
    timestamp: LcTime,
) -> LcTransitionEvent {
    LcTransitionEvent {
        timestamp,
        transition: to_lc_transition(t),
        start_state: to_lc_state(start),
        goal_state: to_lc_state(goal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn state_machine_lock_recovers_from_poison() {
        let state_machine = Arc::new(Mutex::new(StateMachine::new()));
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = state_machine.lock().unwrap();
            panic!("poison state machine");
        }));

        assert_eq!(
            lock_state_machine(&state_machine).current_state(),
            State::Unconfigured
        );
    }

    #[test]
    fn managed_entities_lock_recovers_from_poison() {
        let managed_entities = Arc::new(Mutex::new(Vec::<Arc<dyn ManagedEntity>>::new()));
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = managed_entities.lock().unwrap();
            panic!("poison managed entities");
        }));

        assert_eq!(lock_managed_entities(&managed_entities).len(), 0);
    }

    #[test]
    fn lifecycle_time_conversion_clamps_to_coherent_maximum() {
        let time = to_lc_time(crate::time::Time::from_nanos(i64::MAX));

        assert_eq!(time.sec, i32::MAX);
        assert_eq!(time.nanosec, 999_999_999);
    }
}

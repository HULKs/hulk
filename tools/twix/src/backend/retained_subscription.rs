use std::{sync::Arc, time::Duration};

use eframe::egui::Context as EguiContext;
use parking_lot::Mutex;
use ros_z::{dynamic::DynamicPayload, graph::GraphChangeSubscription, node::Node, time::Time};
use ros_z_debug::{
    JsonRenderPolicy, RetentionPolicy, SampleRecord, SubscriptionHandle,
    SubscriptionStatusSnapshot, SubscriptionUpdate, SubscriptionUpdateReceiver,
    dynamic_payload_to_json,
};
use serde_json::Value;
use tokio::{runtime::Runtime, sync::watch, time};

use super::subscription;

const SUBSCRIBE_RETRY_DELAY: Duration = Duration::from_secs(1);

type SharedState<T> = Arc<Mutex<RetainedSubscriptionState<T>>>;
type WeakState<T> = std::sync::Weak<Mutex<RetainedSubscriptionState<T>>>;

pub struct RetainedSubscription<T> {
    state: SharedState<T>,
    retention_sender: watch::Sender<RetentionPolicy>,
}

impl<T> Clone for RetainedSubscription<T> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            retention_sender: self.retention_sender.clone(),
        }
    }
}

impl<T> RetainedSubscription<T> {
    fn new(retention: RetentionPolicy) -> Self {
        let (retention_sender, _) = watch::channel(retention);

        Self {
            state: Arc::new(Mutex::new(RetainedSubscriptionState::default())),
            retention_sender,
        }
    }

    pub fn latest(&self) -> Option<Arc<SampleRecord<T>>> {
        self.handle().and_then(|handle| handle.latest())
    }

    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<SampleRecord<T>>> {
        self.handle()
            .map_or_else(Vec::new, |handle| handle.window(start, end))
    }

    pub fn diagnostic_message(&self) -> Option<String> {
        self.state.lock().diagnostic.clone()
    }

    pub fn set_retention(&self, retention: RetentionPolicy) {
        let current = *self.retention_sender.borrow();
        if current != retention {
            self.retention_sender.send_replace(retention);
        }
    }

    fn handle(&self) -> Option<SubscriptionHandle<T>> {
        self.state.lock().handle.clone()
    }
}

struct RetainedSubscriptionState<T> {
    handle: Option<SubscriptionHandle<T>>,
    status: Option<SubscriptionStatusSnapshot>,
    diagnostic: Option<String>,
}

impl<T> Default for RetainedSubscriptionState<T> {
    fn default() -> Self {
        Self {
            handle: None,
            status: None,
            diagnostic: None,
        }
    }
}

pub struct DynamicSubscription {
    retained: RetainedSubscription<DynamicPayload>,
}

impl Clone for DynamicSubscription {
    fn clone(&self) -> Self {
        Self {
            retained: self.retained.clone(),
        }
    }
}

impl DynamicSubscription {
    pub fn latest_json(&self) -> Option<Value> {
        self.latest_json_with_timestamp()
            .map(|(_timestamp, value)| value)
    }

    pub fn latest_json_with_timestamp(&self) -> Option<(Time, Value)> {
        self.retained.latest().map(|record| {
            (
                record.source_time,
                dynamic_payload_to_json(&record.value, JsonRenderPolicy::default()),
            )
        })
    }

    pub fn window_json(&self, start: Time, end: Time) -> Vec<(Time, Value)> {
        self.retained
            .window(start, end)
            .iter()
            .map(|record| {
                (
                    record.source_time,
                    dynamic_payload_to_json(&record.value, JsonRenderPolicy::default()),
                )
            })
            .collect()
    }

    pub fn diagnostic_message(&self) -> Option<String> {
        self.retained.diagnostic_message()
    }

    pub fn set_retention(&self, retention: RetentionPolicy) {
        self.retained.set_retention(retention);
    }
}

pub fn subscribe_dynamic(
    runtime: &Runtime,
    node: Arc<Node>,
    target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: impl Into<String>,
    retention: RetentionPolicy,
) -> DynamicSubscription {
    let retained = RetainedSubscription::new(retention);
    let state = Arc::downgrade(&retained.state);
    let retention_receiver = retained.retention_sender.subscribe();

    runtime.spawn(run_dynamic_subscription(
        node,
        target_namespace,
        egui_context,
        selector.into(),
        retention_receiver,
        state,
    ));

    DynamicSubscription { retained }
}

async fn run_dynamic_subscription(
    node: Arc<Node>,
    mut target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: String,
    mut retention: watch::Receiver<RetentionPolicy>,
    state: WeakState<DynamicPayload>,
) {
    let mut graph_changes = node.graph().subscribe_changes();
    let mut subscribe_error_handle_policy = SubscribeErrorHandlePolicy::ClearExisting;

    loop {
        if state.strong_count() == 0 {
            break;
        }

        let namespace = target_namespace.borrow_and_update().clone();
        let retention_policy = *retention.borrow_and_update();
        let subscribe = subscription::subscribe_dynamic(
            node.clone(),
            namespace,
            selector.clone(),
            retention_policy,
        );
        tokio::pin!(subscribe);

        let active_subscription = tokio::select! {
            result = &mut subscribe => result,
            changed = target_namespace.changed() => {
                if changed.is_err() {
                    break;
                }
                subscribe_error_handle_policy = SubscribeErrorHandlePolicy::ClearExisting;
                if !clear_handle(&state) {
                    break;
                }
                continue;
            }
            changed = retention.changed() => {
                if changed.is_err() {
                    break;
                }
                subscribe_error_handle_policy = SubscribeErrorHandlePolicy::KeepExisting;
                continue;
            }
            changed = graph_changes.changed() => {
                if changed.is_none() {
                    break;
                }
                subscribe_error_handle_policy = SubscribeErrorHandlePolicy::ClearExisting;
                if !clear_handle(&state) {
                    break;
                }
                continue;
            }
        };

        match active_subscription {
            Ok(mut active_subscription) => {
                if !install_handle(&state, active_subscription.handle.clone()) {
                    break;
                }
                egui_context.request_repaint();

                let forward_exit = forward_subscription_updates(
                    &mut active_subscription.updates,
                    &mut target_namespace,
                    &mut retention,
                    &mut graph_changes,
                    &state,
                    &egui_context,
                )
                .await;

                if matches!(
                    forward_exit,
                    ForwardSubscriptionExit::OwnerDropped
                        | ForwardSubscriptionExit::ControlChannelClosed
                ) {
                    break;
                }

                if should_clear_handle_after_forward_exit(forward_exit) && !clear_handle(&state) {
                    break;
                }
                subscribe_error_handle_policy =
                    subscribe_error_handle_policy_after_forward_exit(forward_exit);
            }
            Err(error) => {
                if !set_subscribe_error(&state, &error, subscribe_error_handle_policy) {
                    break;
                }
                egui_context.request_repaint();

                let Some(rebuild_signal) = wait_for_rebuild_signal(
                    &mut target_namespace,
                    &mut retention,
                    &mut graph_changes,
                )
                .await
                else {
                    break;
                };

                match rebuild_signal {
                    RebuildSignal::Retry => {}
                    RebuildSignal::RetentionChanged => {
                        subscribe_error_handle_policy = SubscribeErrorHandlePolicy::KeepExisting;
                    }
                    RebuildSignal::TargetNamespaceChanged | RebuildSignal::GraphChanged => {
                        subscribe_error_handle_policy = SubscribeErrorHandlePolicy::ClearExisting;
                        if !clear_handle(&state) {
                            break;
                        }
                    }
                }
            }
        }
    }

    let _ = clear_handle(&state);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForwardSubscriptionExit {
    TargetNamespaceChanged,
    GraphChanged,
    RetentionChanged,
    UpdateReceiverClosed,
    ControlChannelClosed,
    OwnerDropped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DrainUpdatesOutcome {
    Drained,
    ReceiverClosed,
    OwnerDropped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubscribeErrorHandlePolicy {
    ClearExisting,
    KeepExisting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RebuildSignal {
    Retry,
    TargetNamespaceChanged,
    RetentionChanged,
    GraphChanged,
}

async fn forward_subscription_updates(
    updates: &mut SubscriptionUpdateReceiver,
    target_namespace: &mut watch::Receiver<String>,
    retention: &mut watch::Receiver<RetentionPolicy>,
    graph_changes: &mut GraphChangeSubscription,
    state: &WeakState<DynamicPayload>,
    egui_context: &EguiContext,
) -> ForwardSubscriptionExit {
    loop {
        tokio::select! {
            outcome = drain_updates(updates, state, egui_context) => {
                match outcome {
                    DrainUpdatesOutcome::Drained => {}
                    DrainUpdatesOutcome::ReceiverClosed => {
                        return ForwardSubscriptionExit::UpdateReceiverClosed;
                    }
                    DrainUpdatesOutcome::OwnerDropped => {
                        return ForwardSubscriptionExit::OwnerDropped;
                    }
                }
            }
            changed = target_namespace.changed() => return match changed {
                Ok(()) => ForwardSubscriptionExit::TargetNamespaceChanged,
                Err(_) => control_channel_closed_exit(state),
            },
            changed = retention.changed() => return match changed {
                Ok(()) => ForwardSubscriptionExit::RetentionChanged,
                Err(_) => control_channel_closed_exit(state),
            },
            changed = graph_changes.changed() => return match changed {
                Some(_) => ForwardSubscriptionExit::GraphChanged,
                None => control_channel_closed_exit(state),
            },
        }
    }
}

fn control_channel_closed_exit<T>(state: &WeakState<T>) -> ForwardSubscriptionExit {
    if state.strong_count() == 0 {
        ForwardSubscriptionExit::OwnerDropped
    } else {
        ForwardSubscriptionExit::ControlChannelClosed
    }
}

fn should_clear_handle_after_forward_exit(exit: ForwardSubscriptionExit) -> bool {
    matches!(
        exit,
        ForwardSubscriptionExit::TargetNamespaceChanged
            | ForwardSubscriptionExit::GraphChanged
            | ForwardSubscriptionExit::UpdateReceiverClosed
    )
}

fn subscribe_error_handle_policy_after_forward_exit(
    exit: ForwardSubscriptionExit,
) -> SubscribeErrorHandlePolicy {
    match exit {
        ForwardSubscriptionExit::RetentionChanged => SubscribeErrorHandlePolicy::KeepExisting,
        ForwardSubscriptionExit::TargetNamespaceChanged
        | ForwardSubscriptionExit::GraphChanged
        | ForwardSubscriptionExit::UpdateReceiverClosed
        | ForwardSubscriptionExit::ControlChannelClosed
        | ForwardSubscriptionExit::OwnerDropped => SubscribeErrorHandlePolicy::ClearExisting,
    }
}

async fn drain_updates(
    updates: &mut SubscriptionUpdateReceiver,
    state: &WeakState<DynamicPayload>,
    egui_context: &EguiContext,
) -> DrainUpdatesOutcome {
    let update = match updates.recv().await {
        Ok(update) => update,
        Err(_) => return DrainUpdatesOutcome::ReceiverClosed,
    };

    let mut request_repaint = false;
    if !handle_update(update, state, &mut request_repaint) {
        return DrainUpdatesOutcome::OwnerDropped;
    }

    let mut budget = subscription::UpdateDrainBudget::new();
    while budget.can_process() {
        let update = match updates.try_recv() {
            Ok(Some(update)) => update,
            Ok(None) => break,
            Err(_) => return DrainUpdatesOutcome::ReceiverClosed,
        };
        budget.record_processed();

        if !handle_update(update, state, &mut request_repaint) {
            return DrainUpdatesOutcome::OwnerDropped;
        }
    }

    if request_repaint || budget.may_have_more() {
        egui_context.request_repaint();
    }
    if budget.may_have_more() {
        tokio::task::yield_now().await;
    }

    DrainUpdatesOutcome::Drained
}

fn handle_update(
    update: SubscriptionUpdate,
    state: &WeakState<DynamicPayload>,
    request_repaint: &mut bool,
) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };

    match update {
        SubscriptionUpdate::DataChanged => {
            state.lock().diagnostic = None;
        }
        SubscriptionUpdate::StatusChanged(snapshot) => {
            let diagnostic = snapshot.message().map(str::to_owned);
            let mut state = state.lock();
            state.status = Some(snapshot);
            state.diagnostic = diagnostic;
        }
        SubscriptionUpdate::Lagged { dropped } => {
            state.lock().diagnostic = Some(format!(
                "subscription update receiver lagged; dropped {dropped} updates"
            ));
        }
        _ => {}
    }

    *request_repaint = true;
    true
}

async fn wait_for_rebuild_signal(
    target_namespace: &mut watch::Receiver<String>,
    retention: &mut watch::Receiver<RetentionPolicy>,
    graph_changes: &mut GraphChangeSubscription,
) -> Option<RebuildSignal> {
    let retry = time::sleep(SUBSCRIBE_RETRY_DELAY);
    tokio::pin!(retry);

    tokio::select! {
        _ = &mut retry => Some(RebuildSignal::Retry),
        changed = target_namespace.changed() => {
            changed.ok().map(|()| RebuildSignal::TargetNamespaceChanged)
        }
        changed = retention.changed() => changed.ok().map(|()| RebuildSignal::RetentionChanged),
        changed = graph_changes.changed() => changed.map(|_| RebuildSignal::GraphChanged),
    }
}

fn install_handle<T>(state: &WeakState<T>, handle: SubscriptionHandle<T>) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };

    let status = handle.status();
    let diagnostic = status.message().map(str::to_owned);
    let mut state = state.lock();
    state.handle = Some(handle);
    state.status = Some(status);
    state.diagnostic = diagnostic;
    true
}

fn clear_handle<T>(state: &WeakState<T>) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };

    state.lock().handle = None;
    true
}

fn set_subscribe_error<T>(
    state: &WeakState<T>,
    error: &color_eyre::Report,
    handle_policy: SubscribeErrorHandlePolicy,
) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };

    let mut state = state.lock();
    if matches!(handle_policy, SubscribeErrorHandlePolicy::ClearExisting) {
        state.handle = None;
        state.status = None;
    }
    state.diagnostic = Some(format!("{error:#}"));
    true
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use color_eyre::eyre::eyre;
    use ros_z::{dynamic::DynamicPayload, prelude::*};
    use ros_z_debug::{ManagerOptions, RetentionPolicy, SubscriptionManager, SubscriptionUpdate};

    use super::{
        ForwardSubscriptionExit, RetainedSubscription, SubscribeErrorHandlePolicy, handle_update,
        set_subscribe_error, should_clear_handle_after_forward_exit,
    };

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ros_z::Message)]
    struct SubscribeErrorStateMessage {
        value: String,
    }

    #[test]
    fn data_update_clears_lagged_diagnostic() {
        let retained = RetainedSubscription::<DynamicPayload>::new(RetentionPolicy::LatestOnly);
        let state = Arc::downgrade(&retained.state);
        let mut request_repaint = false;

        assert!(handle_update(
            SubscriptionUpdate::Lagged { dropped: 7 },
            &state,
            &mut request_repaint,
        ));
        assert!(
            retained
                .diagnostic_message()
                .is_some_and(|message| message.contains("dropped 7 updates"))
        );

        request_repaint = false;
        assert!(handle_update(
            SubscriptionUpdate::DataChanged,
            &state,
            &mut request_repaint,
        ));

        assert_eq!(retained.diagnostic_message(), None);
        assert!(request_repaint);
    }

    #[test]
    fn forward_exit_clears_handle_for_retarget_or_graph_but_not_retention() {
        assert!(should_clear_handle_after_forward_exit(
            ForwardSubscriptionExit::TargetNamespaceChanged,
        ));
        assert!(should_clear_handle_after_forward_exit(
            ForwardSubscriptionExit::GraphChanged,
        ));
        assert!(!should_clear_handle_after_forward_exit(
            ForwardSubscriptionExit::RetentionChanged,
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_error_with_keep_existing_handle_preserves_handle() {
        let retained =
            RetainedSubscription::<SubscribeErrorStateMessage>::new(RetentionPolicy::LatestOnly);
        let state = Arc::downgrade(&retained.state);
        retained.state.lock().handle = Some(typed_handle("twix_keep_existing_handle").await);

        let error = eyre!("subscribe failed");
        assert!(set_subscribe_error(
            &state,
            &error,
            SubscribeErrorHandlePolicy::KeepExisting,
        ));

        assert!(retained.handle().is_some());
        assert_eq!(
            retained.diagnostic_message().as_deref(),
            Some("subscribe failed")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_error_with_clear_handle_policy_clears_handle() {
        let retained =
            RetainedSubscription::<SubscribeErrorStateMessage>::new(RetentionPolicy::LatestOnly);
        let state = Arc::downgrade(&retained.state);
        retained.state.lock().handle = Some(typed_handle("twix_clear_existing_handle").await);

        let error = eyre!("subscribe failed");
        assert!(set_subscribe_error(
            &state,
            &error,
            SubscribeErrorHandlePolicy::ClearExisting,
        ));

        assert!(retained.handle().is_none());
        assert_eq!(
            retained.diagnostic_message().as_deref(),
            Some("subscribe failed")
        );
    }

    async fn typed_handle(
        topic: &str,
    ) -> ros_z_debug::SubscriptionHandle<SubscribeErrorStateMessage> {
        let context = ContextBuilder::default()
            .disable_multicast_scouting()
            .with_json("connect/endpoints", serde_json::json!([]))
            .build()
            .await
            .expect("context should build");
        let node = Arc::new(
            context
                .create_node(format!("{topic}_sub"))
                .build()
                .await
                .expect("subscriber node"),
        );
        let manager = SubscriptionManager::new(node, ManagerOptions::default());

        manager
            .subscribe_typed::<SubscribeErrorStateMessage>(topic)
            .retention(RetentionPolicy::LatestOnly)
            .build()
            .await
            .expect("subscription should build")
    }
}

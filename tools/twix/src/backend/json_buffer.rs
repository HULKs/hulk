use std::{future::Future, sync::Arc, time::Duration};

use color_eyre::eyre::eyre;
use color_eyre::{Result, eyre::Report};
use eframe::egui::Context as EguiContext;
use ros_z::{dynamic::DynamicPayload, node::Node, pubsub::PublicationId, time::Time};
use ros_z_debug::{
    DebugEvent, JsonRenderPolicy, ManagerOptions, RetentionPolicy, SampleRecord,
    SubscriptionManager, dynamic_payload_to_json,
};
use serde_json::Value;
use tokio::{
    runtime::Runtime,
    sync::watch,
    time::{self, MissedTickBehavior},
};

use crate::value_buffer::{Buffer, BufferHandle, Datum};

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const SUBSCRIBE_RETRY_DELAY: Duration = Duration::from_secs(1);

type JsonBuffer = Buffer<Value, Report>;

struct ActiveSubscription {
    _manager: SubscriptionManager,
    handle: ros_z_debug::SubscriptionHandle<DynamicPayload>,
    retention: RetentionPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RebuildReason {
    Retarget,
    RetentionChanged,
    Retry,
}

pub fn subscribe_json(
    runtime: &Runtime,
    node: Arc<Node>,
    target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: impl Into<String>,
    history: Duration,
) -> BufferHandle<Value> {
    let (buffer, handle) = Buffer::new(history);
    runtime.spawn(run_json_buffer(
        node,
        target_namespace,
        egui_context,
        selector.into(),
        buffer,
    ));
    handle
}

async fn run_json_buffer(
    node: Arc<Node>,
    mut target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: String,
    buffer: JsonBuffer,
) {
    let mut clear_on_rebuild = true;

    loop {
        if buffer.is_closed() {
            break;
        }

        if clear_on_rebuild {
            buffer.replace(Vec::new());
            egui_context.request_repaint();
        }

        let namespace = target_namespace.borrow_and_update().clone();
        let retention = retention_policy(buffer.history().await);
        let subscription = subscribe_dynamic(node.clone(), namespace, selector.clone(), retention);
        tokio::pin!(subscription);

        let active_subscription = tokio::select! {
            result = &mut subscription => result,
            changed = target_namespace.changed() => {
                if changed.is_err() {
                    break;
                }
                clear_on_rebuild = true;
                continue;
            }
            _ = buffer.closed() => break,
        };

        let rebuild_reason = match active_subscription {
            Ok(active_subscription) => {
                if buffer.clear_error() {
                    egui_context.request_repaint();
                }
                let Some(rebuild_reason) = forward_subscription(
                    active_subscription,
                    &mut target_namespace,
                    &buffer,
                    &egui_context,
                )
                .await
                else {
                    break;
                };
                rebuild_reason
            }
            Err(error) => {
                buffer.send_error(error);
                egui_context.request_repaint();
                let Some(rebuild_reason) =
                    wait_for_retry_or_retarget(&mut target_namespace, &buffer).await
                else {
                    break;
                };
                rebuild_reason
            }
        };

        clear_on_rebuild = should_clear_on_rebuild(rebuild_reason);
    }
}

async fn subscribe_dynamic(
    node: Arc<Node>,
    target_namespace: String,
    selector: String,
    retention: RetentionPolicy,
) -> Result<ActiveSubscription> {
    let manager = SubscriptionManager::new(
        node,
        ManagerOptions::with_target_namespace(target_namespace)?,
    );
    let handle = manager
        .subscribe_dynamic(selector)
        .retention(retention)
        .build()
        .await?;

    Ok(ActiveSubscription {
        _manager: manager,
        handle,
        retention,
    })
}

async fn forward_subscription(
    active_subscription: ActiveSubscription,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonBuffer,
    egui_context: &EguiContext,
) -> Option<RebuildReason> {
    let mut poll = time::interval(EVENT_POLL_INTERVAL);
    poll.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = poll.tick() => {
                if let Some(rebuild_reason) = poll_tick_rebuild_reason(
                    active_subscription.retention,
                    drain_events(
                        &active_subscription,
                        buffer,
                        egui_context,
                    ),
                    async { retention_policy(buffer.history().await) },
                ).await {
                    return Some(rebuild_reason);
                }
            }
            changed = target_namespace.changed() => return changed.ok().map(|()| RebuildReason::Retarget),
            _ = buffer.closed() => return None,
        }
    }
}

async fn drain_events(
    active_subscription: &ActiveSubscription,
    buffer: &JsonBuffer,
    egui_context: &EguiContext,
) {
    let events = active_subscription.handle.drain_events();
    if events.is_empty() {
        return;
    }

    let has_identity_events = events
        .iter()
        .any(|event| matches!(event, DebugEvent::ValueRetained { .. }));
    let mut requested_repaint = false;

    for event in events {
        match event {
            DebugEvent::ValueUpdated => {
                if !has_identity_events && let Some(record) = active_subscription.handle.latest() {
                    forward_record(record, buffer).await;
                    requested_repaint = true;
                }
            }
            DebugEvent::ValueRetained {
                source_time,
                publication_id,
            } => {
                if let Some(record) = retained_record(
                    &active_subscription.handle,
                    active_subscription.retention,
                    source_time,
                    publication_id,
                ) {
                    forward_record(record, buffer).await;
                    requested_repaint = true;
                }
            }
            DebugEvent::Diagnostic(message) => {
                buffer.send_error(eyre!(message));
                requested_repaint = true;
            }
            DebugEvent::StatusChanged => {}
            _ => {}
        }
    }

    if requested_repaint {
        egui_context.request_repaint();
    }
}

async fn poll_tick_rebuild_reason(
    active_retention: RetentionPolicy,
    drain_events: impl Future<Output = ()>,
    current_retention: impl Future<Output = RetentionPolicy>,
) -> Option<RebuildReason> {
    drain_events.await;
    (current_retention.await != active_retention).then_some(RebuildReason::RetentionChanged)
}

fn should_clear_on_rebuild(rebuild_reason: RebuildReason) -> bool {
    matches!(rebuild_reason, RebuildReason::Retarget)
}

async fn forward_record(record: Arc<SampleRecord<DynamicPayload>>, buffer: &JsonBuffer) {
    buffer
        .push(Datum {
            timestamp: record.source_time.to_wallclock(),
            value: dynamic_payload_to_json(&record.value, JsonRenderPolicy::default()),
        })
        .await;
}

fn retained_record(
    handle: &ros_z_debug::SubscriptionHandle<DynamicPayload>,
    retention: RetentionPolicy,
    source_time: Time,
    publication_id: PublicationId,
) -> Option<Arc<SampleRecord<DynamicPayload>>> {
    match retention {
        RetentionPolicy::TimeWindow(_) => handle
            .window(source_time, source_time)
            .into_iter()
            .find(|record| record.publication_id == publication_id),
        RetentionPolicy::LatestOnly => handle.latest().filter(|record| {
            record.source_time == source_time && record.publication_id == publication_id
        }),
        _ => handle.latest().filter(|record| {
            record.source_time == source_time && record.publication_id == publication_id
        }),
    }
}

async fn wait_for_retry_or_retarget(
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonBuffer,
) -> Option<RebuildReason> {
    let retry = time::sleep(SUBSCRIBE_RETRY_DELAY);
    tokio::pin!(retry);

    tokio::select! {
        _ = &mut retry => Some(RebuildReason::Retry),
        changed = target_namespace.changed() => changed.ok().map(|()| RebuildReason::Retarget),
        _ = buffer.closed() => None,
    }
}

fn retention_policy(history: Duration) -> RetentionPolicy {
    if history.is_zero() {
        return RetentionPolicy::LatestOnly;
    }

    match RetentionPolicy::time_window(history) {
        Ok(retention) => retention,
        Err(_) => RetentionPolicy::LatestOnly,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    #[tokio::test]
    async fn poll_tick_drains_events_before_returning_retention_changed() {
        let order = Rc::new(RefCell::new(Vec::new()));
        let drain_order = order.clone();
        let retention_order = order.clone();

        let reason = poll_tick_rebuild_reason(
            RetentionPolicy::LatestOnly,
            async move {
                drain_order.borrow_mut().push("drain");
            },
            async move {
                retention_order.borrow_mut().push("retention");
                RetentionPolicy::time_window(Duration::from_secs(1)).unwrap()
            },
        )
        .await;

        assert_eq!(reason, Some(RebuildReason::RetentionChanged));
        assert_eq!(order.borrow().as_slice(), ["drain", "retention"]);
    }

    #[test]
    fn rebuild_clears_only_after_retarget() {
        assert!(should_clear_on_rebuild(RebuildReason::Retarget));
        assert!(!should_clear_on_rebuild(RebuildReason::RetentionChanged));
        assert!(!should_clear_on_rebuild(RebuildReason::Retry));
    }
}

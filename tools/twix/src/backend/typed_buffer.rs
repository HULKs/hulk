use std::{sync::Arc, time::Duration};

use color_eyre::eyre::{Report, eyre};
use eframe::egui::Context as EguiContext;
use ros_z::{Message, qos::QosProfile};
use ros_z_debug::{RetentionPolicy, SampleRecord};
use tokio::{runtime::Runtime, sync::watch, time};

use crate::{
    backend::{connection::ConnectionState, latency::trace_forward_latency, subscription},
    value_buffer::{Buffer, BufferHandle, BufferHistory, Datum},
};

const SUBSCRIBE_RETRY_DELAY: Duration = Duration::from_secs(1);

type TypedBuffer<T> = Buffer<T, Report>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RebuildReason {
    ConnectionChanged,
    Retarget,
    RetentionChanged,
    Retry,
}

pub fn subscribe_value<T>(
    runtime: &Runtime,
    connection_state: watch::Receiver<ConnectionState>,
    target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: impl Into<String>,
    history: BufferHistory,
    qos: Option<QosProfile>,
) -> BufferHandle<T>
where
    T: Message + Clone,
    T::Codec: Send + Sync,
{
    let (buffer, handle) = Buffer::new(history);
    runtime.spawn(run_typed_buffer(
        connection_state,
        target_namespace,
        egui_context,
        selector.into(),
        qos,
        buffer,
    ));
    handle
}

async fn run_typed_buffer<T>(
    mut connection_state: watch::Receiver<ConnectionState>,
    mut target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: String,
    qos: Option<QosProfile>,
    buffer: TypedBuffer<T>,
) where
    T: Message + Clone,
    T::Codec: Send + Sync,
{
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
        let state = connection_state.borrow_and_update().clone();
        let Some(node) = state.node() else {
            if let Some(message) = state.unavailable_message() {
                buffer.send_error(eyre!(message.to_string()));
                egui_context.request_repaint();
            }
            let Some(rebuild_reason) = wait_for_connection_or_retarget(
                &mut connection_state,
                &mut target_namespace,
                &buffer,
            )
            .await
            else {
                break;
            };
            clear_on_rebuild = should_clear_on_rebuild(rebuild_reason);
            continue;
        };

        let retention = retention_policy(buffer.history().await);
        let subscription =
            subscription::subscribe_typed::<T>(node, namespace, selector.clone(), retention, qos);
        tokio::pin!(subscription);

        let active_subscription = tokio::select! {
            result = &mut subscription => result,
            changed = connection_state.changed() => {
                if changed.is_err() {
                    break;
                }
                clear_on_rebuild = true;
                continue;
            }
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
                    &mut connection_state,
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
                let Some(rebuild_reason) = wait_for_retry_or_retarget(
                    &mut connection_state,
                    &mut target_namespace,
                    &buffer,
                )
                .await
                else {
                    break;
                };
                rebuild_reason
            }
        };

        clear_on_rebuild = should_clear_on_rebuild(rebuild_reason);
    }
}

async fn forward_subscription<T>(
    mut active_subscription: subscription::ActiveSubscription<T>,
    connection_state: &mut watch::Receiver<ConnectionState>,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &TypedBuffer<T>,
    egui_context: &EguiContext,
) -> Option<RebuildReason>
where
    T: Message + Clone,
{
    let mut history_changes = buffer.subscribe_history();

    if let Some(rebuild_reason) = subscription_event_rebuild_reason(
        active_subscription.retention,
        subscription::drain_events(
            &active_subscription,
            egui_context,
            |error| buffer.send_error(error),
            |record| forward_record(record, buffer),
        ),
        async { retention_policy(buffer.history().await) },
    )
    .await
    {
        return Some(rebuild_reason);
    }

    loop {
        tokio::select! {
            changed = active_subscription.handle.changed() => {
                if changed.is_err() {
                    return None;
                }
                if let Some(rebuild_reason) = subscription_event_rebuild_reason(
                    active_subscription.retention,
                    subscription::drain_events(
                        &active_subscription,
                        egui_context,
                        |error| buffer.send_error(error),
                        |record| forward_record(record, buffer),
                    ),
                    async { retention_policy(buffer.history().await) },
                ).await {
                    return Some(rebuild_reason);
                }
            }
            changed = history_changes.changed() => {
                if changed.is_err() {
                    return None;
                }
                if let Some(rebuild_reason) = subscription_event_rebuild_reason(
                    active_subscription.retention,
                    subscription::drain_events(
                        &active_subscription,
                        egui_context,
                        |error| buffer.send_error(error),
                        |record| forward_record(record, buffer),
                    ),
                    async { retention_policy(buffer.history().await) },
                ).await {
                    return Some(rebuild_reason);
                }
            }
            changed = connection_state.changed() => return changed.ok().map(|()| RebuildReason::ConnectionChanged),
            changed = target_namespace.changed() => return changed.ok().map(|()| RebuildReason::Retarget),
            _ = buffer.closed() => return None,
        }
    }
}

async fn subscription_event_rebuild_reason(
    active_retention: RetentionPolicy,
    drain_events: impl Future<Output = ()>,
    current_retention: impl Future<Output = RetentionPolicy>,
) -> Option<RebuildReason> {
    drain_events.await;
    (current_retention.await != active_retention).then_some(RebuildReason::RetentionChanged)
}

fn should_clear_on_rebuild(rebuild_reason: RebuildReason) -> bool {
    matches!(
        rebuild_reason,
        RebuildReason::ConnectionChanged | RebuildReason::Retarget
    )
}

async fn forward_record<T>(record: Arc<SampleRecord<T>>, buffer: &TypedBuffer<T>)
where
    T: Message + Clone,
{
    trace_forward_latency("typed", &record);
    buffer
        .push(Datum {
            timestamp: record.source_time.to_wallclock(),
            value: record.value.clone(),
        })
        .await;
}

async fn wait_for_retry_or_retarget<T>(
    connection_state: &mut watch::Receiver<ConnectionState>,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &TypedBuffer<T>,
) -> Option<RebuildReason> {
    let retry = time::sleep(SUBSCRIBE_RETRY_DELAY);
    tokio::pin!(retry);

    tokio::select! {
        _ = &mut retry => Some(RebuildReason::Retry),
        changed = connection_state.changed() => changed.ok().map(|()| RebuildReason::ConnectionChanged),
        changed = target_namespace.changed() => changed.ok().map(|()| RebuildReason::Retarget),
        _ = buffer.closed() => None,
    }
}

async fn wait_for_connection_or_retarget<T>(
    connection_state: &mut watch::Receiver<ConnectionState>,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &TypedBuffer<T>,
) -> Option<RebuildReason> {
    tokio::select! {
        changed = connection_state.changed() => changed.ok().map(|()| RebuildReason::ConnectionChanged),
        changed = target_namespace.changed() => changed.ok().map(|()| RebuildReason::Retarget),
        _ = buffer.closed() => None,
    }
}

fn retention_policy(history: BufferHistory) -> RetentionPolicy {
    let BufferHistory::TimeWindow(history) = history else {
        return RetentionPolicy::LatestOnly;
    };

    match RetentionPolicy::time_window(history) {
        Ok(retention) => retention,
        Err(_) => RetentionPolicy::LatestOnly,
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use eframe::egui::Context as EguiContext;
    use ros_z::context::ContextBuilder;
    use tokio::{runtime::Builder, sync::watch};

    use crate::backend::connection::ConnectionState;

    use super::*;

    #[test]
    fn typed_buffer_forwards_published_sample() {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let context = ContextBuilder::default()
                .disable_multicast_scouting()
                .with_json("connect/endpoints", serde_json::json!([]))
                .build()
                .await
                .unwrap();
            let publisher_node = context.create_node("twix_typed_pub").build().await.unwrap();
            let subscriber_node =
                Arc::new(context.create_node("twix_typed_sub").build().await.unwrap());
            let publisher = publisher_node
                .publisher::<String>("twix_debug_text")
                .unwrap()
                .build()
                .await
                .unwrap();
            let (_connection_sender, connection_receiver) =
                watch::channel(ConnectionState::connected(subscriber_node));
            let (_namespace_sender, namespace_receiver) = watch::channel("/".to_string());
            let buffer = subscribe_value::<String>(
                &runtime,
                connection_receiver,
                namespace_receiver,
                EguiContext::default(),
                "twix_debug_text",
                BufferHistory::LatestOnly,
                None,
            );

            assert!(
                publisher
                    .wait_for_subscribers(1, Duration::from_secs(1))
                    .await
            );
            publisher.publish(&"hello".to_string()).await.unwrap();

            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            loop {
                if buffer.get_last_value().unwrap() == Some("hello".to_string()) {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for Twix typed buffer"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }

    #[test]
    fn typed_buffer_reports_disconnected_connection_state() {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let (_connection_sender, connection_receiver) =
                watch::channel(ConnectionState::disconnected());
            let (_namespace_sender, namespace_receiver) = watch::channel("/".to_string());
            let buffer = subscribe_value::<String>(
                &runtime,
                connection_receiver,
                namespace_receiver,
                EguiContext::default(),
                "twix_debug_text",
                BufferHistory::LatestOnly,
                None,
            );

            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            loop {
                match buffer.get_last() {
                    Err(error) if error.to_string().contains("Twix is disconnected") => break,
                    _ => {}
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for disconnected Twix typed buffer error"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }
}

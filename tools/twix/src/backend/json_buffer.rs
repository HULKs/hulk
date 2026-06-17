use std::{future::Future, sync::Arc, time::Duration};

use color_eyre::eyre::{Report, eyre};
use eframe::egui::Context as EguiContext;
use ros_z::{dynamic::DynamicPayload, qos::QosProfile};
use ros_z_debug::{JsonRenderPolicy, RetentionPolicy, SampleRecord, dynamic_payload_to_json};
use serde_json::Value;
use tokio::{runtime::Runtime, sync::watch};

use crate::{
    backend::{
        connection::ConnectionState,
        latency::trace_forward_latency,
        subscription::{self, ActiveSubscription},
    },
    value_buffer::{Buffer, BufferHandle, BufferHistory, Datum},
};

type JsonBuffer = Buffer<Value, Report>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RebuildReason {
    ConnectionChanged,
    Retarget,
    RetentionChanged,
    Retry,
}

pub fn subscribe_json(
    runtime: &Runtime,
    connection_state: watch::Receiver<ConnectionState>,
    target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: impl Into<String>,
    history: BufferHistory,
    qos: Option<QosProfile>,
) -> BufferHandle<Value> {
    let (buffer, handle) = Buffer::new(history);
    runtime.spawn(run_json_buffer(
        connection_state,
        target_namespace,
        egui_context,
        selector.into(),
        qos,
        buffer,
    ));
    handle
}

async fn run_json_buffer(
    mut connection_state: watch::Receiver<ConnectionState>,
    mut target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: String,
    qos: Option<QosProfile>,
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
        let subscription = subscription::subscribe_dynamic(
            node.clone(),
            namespace,
            selector.clone(),
            retention,
            qos,
        );
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

async fn forward_subscription(
    mut active_subscription: ActiveSubscription,
    connection_state: &mut watch::Receiver<ConnectionState>,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonBuffer,
    egui_context: &EguiContext,
) -> Option<RebuildReason> {
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

async fn forward_record(record: Arc<SampleRecord<DynamicPayload>>, buffer: &JsonBuffer) {
    trace_forward_latency("json", &record);
    buffer
        .push(Datum {
            timestamp: record.source_time.to_wallclock(),
            value: dynamic_payload_to_json(&record.value, JsonRenderPolicy::default()),
        })
        .await;
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

async fn wait_for_retry_or_retarget(
    connection_state: &mut watch::Receiver<ConnectionState>,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonBuffer,
) -> Option<RebuildReason> {
    let retry = tokio::time::sleep(Duration::from_secs(1));
    tokio::pin!(retry);

    tokio::select! {
        _ = &mut retry => Some(RebuildReason::Retry),
        changed = connection_state.changed() => changed.ok().map(|()| RebuildReason::ConnectionChanged),
        changed = target_namespace.changed() => changed.ok().map(|()| RebuildReason::Retarget),
        _ = buffer.closed() => None,
    }
}

async fn wait_for_connection_or_retarget(
    connection_state: &mut watch::Receiver<ConnectionState>,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonBuffer,
) -> Option<RebuildReason> {
    tokio::select! {
        changed = connection_state.changed() => changed.ok().map(|()| RebuildReason::ConnectionChanged),
        changed = target_namespace.changed() => changed.ok().map(|()| RebuildReason::Retarget),
        _ = buffer.closed() => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, time::Duration};

    use eframe::egui::Context as EguiContext;
    use ros_z::{
        context::ContextBuilder,
        dynamic::{DynamicPayload, DynamicStruct},
    };
    use tokio::{runtime::Builder, sync::watch};

    use crate::backend::connection::ConnectionState;

    use super::*;

    fn string_message_schema() -> ros_z::dynamic::Schema {
        use ros_z_schema::{
            FieldDef, SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName,
        };

        let name = TypeName::new("test_msgs::StringMessage").expect("valid type name");
        Arc::new(SchemaBundle {
            root: TypeDef::Named(name.clone()),
            definitions: TypeDefinitions::from([(
                name,
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new("data", TypeDef::String)],
                }),
            )]),
        })
    }

    #[test]
    fn json_buffer_forwards_published_dynamic_sample() {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let context = ContextBuilder::default()
                .disable_multicast_scouting()
                .with_json("connect/endpoints", serde_json::json!([]))
                .build()
                .await
                .unwrap();
            let publisher_node = context.create_node("twix_json_pub").build().await.unwrap();
            let subscriber_node =
                Arc::new(context.create_node("twix_json_sub").build().await.unwrap());
            let schema = string_message_schema();
            let type_info = ros_z::TypeInfo::new(
                "test_msgs::StringMessage",
                ros_z_schema::compute_hash(schema.as_ref()).unwrap(),
            );
            let publisher = publisher_node
                .dynamic_publisher("twix_debug_dynamic", type_info, schema.clone())
                .unwrap()
                .build()
                .await
                .unwrap();
            let (_connection_sender, connection_receiver) =
                watch::channel(ConnectionState::connected(subscriber_node));
            let (_namespace_sender, namespace_receiver) = watch::channel("/".to_string());
            let buffer = subscribe_json(
                &runtime,
                connection_receiver,
                namespace_receiver,
                EguiContext::default(),
                "twix_debug_dynamic",
                BufferHistory::LatestOnly,
                None,
            );

            assert!(
                publisher
                    .wait_for_subscribers(1, Duration::from_secs(1))
                    .await
            );
            let mut message = DynamicStruct::default_for_schema(&schema).unwrap();
            message.set("data", "hello").unwrap();
            let payload = DynamicPayload::from_struct(message).unwrap();
            publisher.publish(&payload).await.unwrap();

            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            loop {
                if buffer.get_last_value().unwrap() == Some(serde_json::json!({ "data": "hello" }))
                {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for Twix JSON buffer"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }

    #[test]
    fn json_buffer_reports_disconnected_connection_state() {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let (_connection_sender, connection_receiver) =
                watch::channel(ConnectionState::disconnected());
            let (_namespace_sender, namespace_receiver) = watch::channel("/".to_string());
            let buffer = subscribe_json(
                &runtime,
                connection_receiver,
                namespace_receiver,
                EguiContext::default(),
                "twix_debug_dynamic",
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
                    "timed out waiting for disconnected Twix JSON buffer error"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }

    #[tokio::test]
    async fn subscription_event_drains_events_before_returning_retention_changed() {
        let order = Rc::new(RefCell::new(Vec::new()));
        let drain_order = order.clone();
        let retention_order = order.clone();

        let reason = subscription_event_rebuild_reason(
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

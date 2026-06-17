use std::{future::Future, sync::Arc, time::Duration};

use color_eyre::{Report, Result, eyre::eyre};
use eframe::egui::Context as EguiContext;
use ros_z::{dynamic::DynamicPayload, node::Node, qos::QosProfile};
use ros_z_debug::{DebugEvent, ManagerOptions, RetentionPolicy, SampleRecord, SubscriptionManager};
use tokio::{sync::watch, time};

const SUBSCRIBE_RETRY_DELAY: Duration = Duration::from_secs(1);

pub struct ActiveSubscription {
    _manager: SubscriptionManager,
    pub handle: ros_z_debug::SubscriptionHandle<DynamicPayload>,
    pub retention: RetentionPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebuildReason {
    Retarget,
    RetentionChanged,
    Retry,
}

pub async fn subscribe_dynamic(
    node: Arc<Node>,
    target_namespace: String,
    selector: String,
    retention: RetentionPolicy,
    qos: Option<QosProfile>,
) -> Result<ActiveSubscription> {
    let manager = SubscriptionManager::new(
        node,
        ManagerOptions::with_target_namespace(target_namespace)?,
    );
    let mut builder = manager.subscribe_dynamic(selector).retention(retention);
    if let Some(qos) = qos {
        builder = builder.qos(qos);
    }
    let handle = builder.build().await?;

    Ok(ActiveSubscription {
        _manager: manager,
        handle,
        retention,
    })
}

pub async fn drain_events<SendError, ForwardRecord, ForwardFuture>(
    active_subscription: &ActiveSubscription,
    egui_context: &EguiContext,
    mut send_error: SendError,
    mut forward_record: ForwardRecord,
) where
    SendError: FnMut(Report),
    ForwardRecord: FnMut(Arc<SampleRecord<DynamicPayload>>) -> ForwardFuture,
    ForwardFuture: Future<Output = ()>,
{
    let events = active_subscription.handle.drain_events();
    if events.is_empty() {
        return;
    }

    let mut requested_repaint = false;

    for event in events {
        match event {
            DebugEvent::ValueUpdated {
                source_time,
                publication_id,
            } => {
                if let Some(record) = active_subscription
                    .handle
                    .record(source_time, publication_id)
                {
                    forward_record(record).await;
                    requested_repaint = true;
                }
            }
            DebugEvent::Diagnostic(message) => {
                send_error(eyre!(message));
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

pub async fn wait_for_retry_or_retarget(
    target_namespace: &mut watch::Receiver<String>,
    closed: impl Future<Output = ()>,
) -> Option<RebuildReason> {
    let retry = time::sleep(SUBSCRIBE_RETRY_DELAY);
    tokio::pin!(retry);
    tokio::pin!(closed);

    tokio::select! {
        _ = &mut retry => Some(RebuildReason::Retry),
        changed = target_namespace.changed() => changed.ok().map(|()| RebuildReason::Retarget),
        _ = &mut closed => None,
    }
}

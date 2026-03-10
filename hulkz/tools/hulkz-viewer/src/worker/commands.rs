use std::sync::{LazyLock, Mutex};

use color_eyre::{eyre::eyre, Result};
use tokio::sync::mpsc::Sender;
use tracing::warn;

use crate::protocol::{WorkerEvent, WorkerEventEnvelope, WorkerWakeNotifier};

static WAKE_NOTIFIER: LazyLock<Mutex<Option<WorkerWakeNotifier>>> =
    LazyLock::new(|| Mutex::new(None));

pub(super) fn install_wake_notifier(notifier: Option<WorkerWakeNotifier>) {
    if let Ok(mut slot) = WAKE_NOTIFIER.lock() {
        *slot = notifier;
    }
}

fn notify_repaint() {
    if let Ok(guard) = WAKE_NOTIFIER.lock() {
        if let Some(notifier) = guard.as_ref() {
            notifier.notify();
        }
    }
}

pub(super) async fn send_event(
    event_tx: &Sender<WorkerEventEnvelope>,
    event: WorkerEvent,
) -> Result<()> {
    event_tx
        .send(WorkerEventEnvelope::new(event))
        .await
        .map_err(|_| eyre!("failed to send worker event: channel closed"))?;
    notify_repaint();
    Ok(())
}

pub(super) async fn send_error(event_tx: &Sender<WorkerEventEnvelope>, message: impl Into<String>) {
    let message = message.into();
    warn!(%message, "worker error");
    if event_tx
        .send(WorkerEventEnvelope::new(WorkerEvent::Error(message)))
        .await
        .is_ok()
    {
        notify_repaint();
    }
}

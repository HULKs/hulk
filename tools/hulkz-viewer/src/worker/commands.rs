use color_eyre::{eyre::eyre, Result};
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

use crate::model::WorkerEvent;

pub(super) fn send_event(
    event_tx: &UnboundedSender<WorkerEvent>,
    event: WorkerEvent,
) -> Result<()> {
    event_tx
        .send(event)
        .map_err(|_| eyre!("failed to send worker event: channel closed"))
}

pub(super) fn send_error(event_tx: &UnboundedSender<WorkerEvent>, message: impl Into<String>) {
    let message = message.into();
    warn!(%message, "worker error");
    let _ = event_tx.send(WorkerEvent::Error(message));
}

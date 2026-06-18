use tokio::sync::broadcast::{
    Receiver,
    error::{RecvError, TryRecvError},
};

use crate::SubscriptionStatusSnapshot;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SubscriptionUpdate {
    /// A new sample was retained as the latest value.
    DataChanged,
    /// The subscription status snapshot changed.
    StatusChanged(SubscriptionStatusSnapshot),
    /// A non-terminal diagnostic message was recorded.
    Diagnostic(String),
    /// This receiver fell behind and missed updates.
    Lagged { dropped: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubscriptionUpdateClosed;

pub struct SubscriptionUpdateReceiver {
    receiver: Receiver<SubscriptionUpdate>,
}

impl SubscriptionUpdateReceiver {
    pub(crate) fn new(receiver: Receiver<SubscriptionUpdate>) -> Self {
        Self { receiver }
    }

    /// Wait for the next subscription update.
    ///
    /// A terminal [`SubscriptionUpdate::StatusChanged`] carrying
    /// [`crate::SubscriptionStatus::Closed`] is a normal update and does not end
    /// the stream while handles keep the subscription state alive. Callers that
    /// want to stop at terminal close should break when they observe that
    /// status. [`SubscriptionUpdateClosed`] means the subscription state was
    /// dropped and no future updates can arrive.
    pub async fn recv(&mut self) -> Result<SubscriptionUpdate, SubscriptionUpdateClosed> {
        match self.receiver.recv().await {
            Ok(update) => Ok(update),
            Err(RecvError::Lagged(dropped)) => Ok(lagged_update(dropped)),
            Err(RecvError::Closed) => Err(SubscriptionUpdateClosed),
        }
    }

    /// Return the next update if one is immediately available.
    ///
    /// `Ok(None)` means no update is currently queued. `Err` means the
    /// subscription state was dropped and no future updates can arrive.
    pub fn try_recv(&mut self) -> Result<Option<SubscriptionUpdate>, SubscriptionUpdateClosed> {
        match self.receiver.try_recv() {
            Ok(update) => Ok(Some(update)),
            Err(TryRecvError::Lagged(dropped)) => Ok(Some(lagged_update(dropped))),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Closed) => Err(SubscriptionUpdateClosed),
        }
    }
}

fn lagged_update(dropped: u64) -> SubscriptionUpdate {
    SubscriptionUpdate::Lagged {
        dropped: usize::try_from(dropped).unwrap_or(usize::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::{SubscriptionUpdate, SubscriptionUpdateClosed, SubscriptionUpdateReceiver};

    #[test]
    fn try_recv_returns_none_when_no_update_is_available() {
        let (_sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = SubscriptionUpdateReceiver::new(receiver);

        assert!(matches!(receiver.try_recv(), Ok(None)));
    }

    #[test]
    fn try_recv_returns_closed_when_sender_is_dropped() {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = SubscriptionUpdateReceiver::new(receiver);
        drop(sender);

        assert!(matches!(receiver.try_recv(), Err(SubscriptionUpdateClosed)));
    }

    #[test]
    fn try_recv_reports_lagged_update_when_receiver_falls_behind() {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = SubscriptionUpdateReceiver::new(receiver);

        sender.send(SubscriptionUpdate::DataChanged).unwrap();
        sender.send(SubscriptionUpdate::DataChanged).unwrap();

        assert!(matches!(
            receiver.try_recv(),
            Ok(Some(SubscriptionUpdate::Lagged { dropped: 1 }))
        ));
        assert!(matches!(
            receiver.try_recv(),
            Ok(Some(SubscriptionUpdate::DataChanged))
        ));
    }

    #[tokio::test]
    async fn recv_returns_closed_after_sender_closes() {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = SubscriptionUpdateReceiver::new(receiver);
        drop(sender);

        assert!(matches!(
            receiver.recv().await,
            Err(SubscriptionUpdateClosed)
        ));
    }
}

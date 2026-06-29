use tokio::sync::broadcast::{
    Receiver,
    error::{RecvError, TryRecvError},
};

use crate::CachedSubscriptionStatusSnapshot;

/// Live notification emitted by a retained debug subscription.
///
/// Updates are only delivered to receivers that were already subscribed when the
/// event happened. Terminal closure is represented by `CachedSubscriptionUpdateClosed`,
/// not by a `CachedSubscriptionUpdate` variant.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CachedSubscriptionUpdate {
    /// A new sample was retained as the latest value.
    DataChanged,
    /// A non-terminal subscription status snapshot changed.
    ///
    /// Terminal close is reported by `CachedSubscriptionUpdateClosed`; use the
    /// subscription handle's `status()` to inspect retained status.
    StatusChanged(CachedSubscriptionStatusSnapshot),
    /// This receiver fell behind and missed updates.
    Lagged { dropped: u64 },
}

/// Error returned when a subscription's live update stream has ended.
///
/// After this error, no future updates can arrive on that receiver. Use the
/// subscription handle's retained state methods to inspect final status and data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("cached subscription update stream closed")]
pub struct CachedSubscriptionUpdateClosed;

/// Receiver for future live updates from a retained debug subscription.
///
/// Create receivers with `CachedSubscription::subscribe_updates` or
/// `CachedJsonSubscription::subscribe_updates`.
pub struct CachedSubscriptionUpdateReceiver {
    receiver: Receiver<CachedSubscriptionUpdate>,
}

impl CachedSubscriptionUpdateReceiver {
    pub(crate) fn new(receiver: Receiver<CachedSubscriptionUpdate>) -> Self {
        Self { receiver }
    }

    /// Wait for the next live subscription update.
    ///
    /// Receivers observe updates sent after they subscribed; old updates are not
    /// replayed. `Err(CachedSubscriptionUpdateClosed)` means the subscription update
    /// stream ended and no future updates can arrive. Use the owning handle's
    /// `status()`, `latest()`, or `window()` methods to inspect retained state.
    pub async fn recv(
        &mut self,
    ) -> Result<CachedSubscriptionUpdate, CachedSubscriptionUpdateClosed> {
        match self.receiver.recv().await {
            Ok(update) => Ok(update),
            Err(RecvError::Lagged(dropped)) => Ok(lagged_update(dropped)),
            Err(RecvError::Closed) => Err(CachedSubscriptionUpdateClosed),
        }
    }

    /// Return the next live update if one is immediately available.
    ///
    /// Receivers observe updates sent after they subscribed; old updates are not
    /// replayed. `Ok(None)` means no update is currently queued.
    /// `Err(CachedSubscriptionUpdateClosed)` means the live update stream ended and no
    /// future updates can arrive.
    pub fn try_recv(
        &mut self,
    ) -> Result<Option<CachedSubscriptionUpdate>, CachedSubscriptionUpdateClosed> {
        match self.receiver.try_recv() {
            Ok(update) => Ok(Some(update)),
            Err(TryRecvError::Lagged(dropped)) => Ok(Some(lagged_update(dropped))),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Closed) => Err(CachedSubscriptionUpdateClosed),
        }
    }
}

fn lagged_update(dropped: u64) -> CachedSubscriptionUpdate {
    CachedSubscriptionUpdate::Lagged { dropped }
}

#[cfg(test)]
mod tests {
    use super::{
        CachedSubscriptionUpdate, CachedSubscriptionUpdateClosed, CachedSubscriptionUpdateReceiver,
    };

    #[test]
    fn closed_error_has_display_message() {
        let error: Box<dyn std::error::Error + Send + Sync> =
            Box::new(CachedSubscriptionUpdateClosed);

        assert_eq!(
            error.to_string(),
            "cached subscription update stream closed"
        );
    }

    #[test]
    fn try_recv_returns_none_when_no_update_is_available() {
        let (_sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = CachedSubscriptionUpdateReceiver::new(receiver);

        assert!(matches!(receiver.try_recv(), Ok(None)));
    }

    #[test]
    fn try_recv_returns_closed_when_sender_is_dropped() {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = CachedSubscriptionUpdateReceiver::new(receiver);
        drop(sender);

        assert!(matches!(
            receiver.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }

    #[test]
    fn try_recv_reports_lagged_update_when_receiver_falls_behind() {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = CachedSubscriptionUpdateReceiver::new(receiver);

        sender.send(CachedSubscriptionUpdate::DataChanged).unwrap();
        sender.send(CachedSubscriptionUpdate::DataChanged).unwrap();

        let dropped: u64 = match receiver.try_recv().unwrap().unwrap() {
            CachedSubscriptionUpdate::Lagged { dropped } => dropped,
            update => panic!("expected lagged update, got {update:?}"),
        };
        assert_eq!(dropped, 1);
        assert!(matches!(
            receiver.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::DataChanged))
        ));
    }

    #[tokio::test]
    async fn recv_returns_closed_after_sender_closes() {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        let mut receiver = CachedSubscriptionUpdateReceiver::new(receiver);
        drop(sender);

        assert!(matches!(
            receiver.recv().await,
            Err(CachedSubscriptionUpdateClosed)
        ));
    }
}

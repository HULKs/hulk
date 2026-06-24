use std::sync::Arc;

use color_eyre::Result;
use ros_z::{Message, dynamic::DynamicPayload, node::Node, qos::QosProfile};
use ros_z_debug::{
    ManagerOptions, RetentionPolicy, SubscriptionHandle, SubscriptionManager,
    SubscriptionUpdateReceiver,
};

pub(crate) const MAX_UPDATES_PER_WAKE: usize = 64;

pub(crate) struct UpdateDrainBudget {
    processed: usize,
}

impl UpdateDrainBudget {
    pub(crate) fn new() -> Self {
        Self { processed: 0 }
    }

    pub(crate) fn can_process(&self) -> bool {
        self.processed < MAX_UPDATES_PER_WAKE
    }

    pub(crate) fn record_processed(&mut self) {
        debug_assert!(self.can_process());
        self.processed += 1;
    }

    pub(crate) fn may_have_more(&self) -> bool {
        self.processed == MAX_UPDATES_PER_WAKE
    }
}

pub(crate) struct ActiveSubscription<T = DynamicPayload> {
    _manager: SubscriptionManager,
    pub(crate) handle: SubscriptionHandle<T>,
    pub(crate) updates: SubscriptionUpdateReceiver,
}

pub(crate) async fn subscribe_dynamic_with_qos(
    node: Arc<Node>,
    target_namespace: String,
    selector: String,
    retention: RetentionPolicy,
    qos: QosProfile,
) -> Result<ActiveSubscription<DynamicPayload>> {
    let manager = SubscriptionManager::new(
        node,
        ManagerOptions::with_target_namespace(target_namespace)?,
    );
    let handle = manager
        .subscribe_dynamic(selector)
        .retention(retention)
        .qos(qos)
        .build()
        .await?;
    let updates = handle.subscribe_updates()?;

    Ok(ActiveSubscription {
        _manager: manager,
        handle,
        updates,
    })
}

pub(crate) async fn subscribe_typed_with_qos<T>(
    node: Arc<Node>,
    target_namespace: String,
    selector: String,
    retention: RetentionPolicy,
    qos: QosProfile,
) -> Result<ActiveSubscription<T>>
where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    let manager = SubscriptionManager::new(
        node,
        ManagerOptions::with_target_namespace(target_namespace)?,
    );
    let handle = manager
        .subscribe_typed::<T>(selector)
        .retention(retention)
        .qos(qos)
        .build()
        .await?;
    let updates = handle.subscribe_updates()?;

    Ok(ActiveSubscription {
        _manager: manager,
        handle,
        updates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_drain_budget_stops_at_max_updates() {
        let mut budget = UpdateDrainBudget::new();
        let updates = 0..MAX_UPDATES_PER_WAKE + 1;
        let mut processed = 0;

        for _ in updates {
            if !budget.can_process() {
                break;
            }

            budget.record_processed();
            processed += 1;
        }

        assert_eq!(processed, MAX_UPDATES_PER_WAKE);
        assert!(budget.may_have_more());
    }
}

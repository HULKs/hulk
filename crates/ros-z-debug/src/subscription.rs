use std::sync::Arc;

use arc_swap::ArcSwapOption;
use parking_lot::Mutex;

use crate::{
    DebugEvent, SampleRecord, SubscriptionStatus, SubscriptionStatusSnapshot, event::EventBuffer,
};

pub struct SubscriptionHandle<V> {
    state: Arc<SubscriptionState<V>>,
}

impl<V> Clone for SubscriptionHandle<V> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl<V> SubscriptionHandle<V> {
    pub fn status(&self) -> SubscriptionStatusSnapshot {
        self.state.status()
    }

    pub fn latest(&self) -> Option<Arc<SampleRecord<V>>> {
        self.state.latest()
    }

    pub fn drain_events(&self) -> Vec<DebugEvent> {
        self.state.drain_events()
    }
}

pub(crate) struct SubscriptionState<V> {
    latest: ArcSwapOption<SampleRecord<V>>,
    meta: Mutex<SubscriptionMeta>,
}

struct SubscriptionMeta {
    status: SubscriptionStatusSnapshot,
    events: EventBuffer,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "async subscription builders wire this in Task 10")
)]
impl<V> SubscriptionState<V> {
    pub(crate) fn new(status: SubscriptionStatusSnapshot) -> Self {
        Self {
            latest: ArcSwapOption::empty(),
            meta: Mutex::new(SubscriptionMeta {
                status,
                events: EventBuffer::new(256),
            }),
        }
    }

    pub(crate) fn handle(self: &Arc<Self>) -> SubscriptionHandle<V> {
        SubscriptionHandle {
            state: Arc::clone(self),
        }
    }

    pub(crate) fn store_latest(&self, record: Arc<SampleRecord<V>>) {
        self.latest.store(Some(record));
        self.meta.lock().status.status = SubscriptionStatus::Ready;
    }

    pub(crate) fn push_event(&self, event: DebugEvent) {
        self.meta.lock().events.push(event);
    }

    fn status(&self) -> SubscriptionStatusSnapshot {
        self.meta.lock().status.clone()
    }

    fn latest(&self) -> Option<Arc<SampleRecord<V>>> {
        self.latest.load_full()
    }

    fn drain_events(&self) -> Vec<DebugEvent> {
        self.meta.lock().events.drain()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::time::Time;

    use super::SubscriptionState;
    use crate::{
        DebugEvent, SampleRecord, SubscriptionStatus, SubscriptionStatusSnapshot, TopicSelector,
    };

    struct NonClonePayload(u32);

    fn sample_record(value: NonClonePayload) -> Arc<SampleRecord<NonClonePayload>> {
        Arc::new(SampleRecord {
            value,
            source_time: Time::zero(),
            transport_time: None,
            publication_id: None,
            source_global_id: None,
            requested_topic: TopicSelector::new("camera").unwrap(),
            resolved_topic: "/camera".to_string(),
            namespace_version: 0,
            type_info: None,
            schema: None,
        })
    }

    #[test]
    fn latest_returns_arc_record_without_cloning_payload() {
        let state = Arc::new(SubscriptionState::new(SubscriptionStatusSnapshot::new(
            SubscriptionStatus::WaitingForFirstSample,
        )));
        let record = sample_record(NonClonePayload(7));

        state.store_latest(Arc::clone(&record));

        let latest = state.handle().latest().unwrap();
        assert!(Arc::ptr_eq(&record, &latest));
        assert_eq!(latest.value.0, 7);
    }

    #[test]
    fn drain_events_returns_and_clears_events() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
        ));

        state.push_event(DebugEvent::Diagnostic("connected".to_string()));

        let events = state.handle().drain_events();
        assert!(matches!(&events[..], [DebugEvent::Diagnostic(message)] if message == "connected"));
        assert!(state.handle().drain_events().is_empty());
    }

    #[test]
    fn status_becomes_ready_after_storing_latest() {
        let state = Arc::new(SubscriptionState::new(SubscriptionStatusSnapshot::new(
            SubscriptionStatus::WaitingForFirstSample,
        )));

        state.store_latest(sample_record(NonClonePayload(1)));

        assert_eq!(state.handle().status().status, SubscriptionStatus::Ready);
    }
}

use std::sync::Arc;

use arc_swap::ArcSwapOption;
use parking_lot::Mutex;
use ros_z::time::Time;
use tokio::sync::watch;

use crate::{
    DebugEvent, RetentionPolicy, SampleRecord, SubscriptionStatus, SubscriptionStatusSnapshot,
    event::EventBuffer, history::TimeIndexedHistory,
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

    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<SampleRecord<V>>> {
        self.state.window(start, end)
    }

    pub fn drain_events(&self) -> Vec<DebugEvent> {
        self.state.drain_events()
    }
}

pub(crate) struct SubscriptionState<V> {
    latest: ArcSwapOption<SampleRecord<V>>,
    history: Option<Mutex<TimeIndexedHistory<V>>>,
    meta: Mutex<SubscriptionMeta>,
    cancellation_tx: watch::Sender<()>,
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
    pub(crate) fn new(status: SubscriptionStatusSnapshot, retention: RetentionPolicy) -> Self {
        let (cancellation_tx, _) = watch::channel(());
        let history = match retention {
            RetentionPolicy::LatestOnly => None,
            RetentionPolicy::TimeWindow { .. } => {
                Some(Mutex::new(TimeIndexedHistory::new(retention)))
            }
        };

        Self {
            latest: ArcSwapOption::empty(),
            history,
            meta: Mutex::new(SubscriptionMeta {
                status,
                events: EventBuffer::new(256),
            }),
            cancellation_tx,
        }
    }

    pub(crate) fn handle(self: &Arc<Self>) -> SubscriptionHandle<V> {
        SubscriptionHandle {
            state: Arc::clone(self),
        }
    }

    pub(crate) fn cancellation_receiver(&self) -> watch::Receiver<()> {
        self.cancellation_tx.subscribe()
    }

    pub(crate) fn store_latest(&self, record: Arc<SampleRecord<V>>) {
        if let Some(history) = &self.history {
            history.lock().insert(Arc::clone(&record));
        }
        self.latest.store(Some(record));

        let mut meta = self.meta.lock();
        let status_changed = meta.status.status != SubscriptionStatus::Ready;
        meta.status.status = SubscriptionStatus::Ready;
        meta.status.message = None;
        if status_changed {
            meta.events.push(DebugEvent::StatusChanged);
        }
        meta.events.push(DebugEvent::ValueUpdated);
    }

    pub(crate) fn set_receive_error(&self, status: SubscriptionStatus, message: String) {
        let mut meta = self.meta.lock();
        let status_changed = meta.status.status != status;
        meta.status.status = status;
        meta.status.message = Some(message.clone());
        if status_changed {
            meta.events.push(DebugEvent::StatusChanged);
        }
        meta.events.push(DebugEvent::Diagnostic(message));
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

    fn window(&self, start: Time, end: Time) -> Vec<Arc<SampleRecord<V>>> {
        self.history
            .as_ref()
            .map_or_else(Vec::new, |history| history.lock().window(start, end))
    }

    fn drain_events(&self) -> Vec<DebugEvent> {
        self.meta.lock().events.drain()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use ros_z::time::Time;

    use super::SubscriptionState;
    use crate::{
        DebugEvent, RetentionPolicy, SampleRecord, SubscriptionStatus, SubscriptionStatusSnapshot,
        TopicSelector,
    };

    struct NonClonePayload(u32);

    fn sample_record_at(
        value: NonClonePayload,
        source_time: Time,
    ) -> Arc<SampleRecord<NonClonePayload>> {
        Arc::new(SampleRecord {
            value,
            source_time,
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

    fn sample_record(value: NonClonePayload) -> Arc<SampleRecord<NonClonePayload>> {
        sample_record_at(value, Time::zero())
    }

    #[test]
    fn latest_returns_arc_record_without_cloning_payload() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let record = sample_record(NonClonePayload(7));

        state.store_latest(Arc::clone(&record));

        let latest = state.handle().latest().unwrap();
        assert!(Arc::ptr_eq(&record, &latest));
        assert_eq!(latest.value.0, 7);
    }

    #[test]
    fn time_window_handle_returns_stored_records_in_window() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::TimeWindow {
                duration: Duration::from_secs(10),
                max_samples: None,
            },
        ));
        let first = sample_record_at(NonClonePayload(1), Time::from_nanos(1));
        let second = sample_record_at(NonClonePayload(2), Time::from_nanos(2));

        state.store_latest(Arc::clone(&first));
        state.store_latest(Arc::clone(&second));

        let records = state
            .handle()
            .window(Time::from_nanos(1), Time::from_nanos(2));
        assert_eq!(records.len(), 2);
        assert!(Arc::ptr_eq(&records[0], &first));
        assert!(Arc::ptr_eq(&records[1], &second));
    }

    #[test]
    fn latest_only_handle_returns_empty_window_but_keeps_latest() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let record = sample_record_at(NonClonePayload(3), Time::from_nanos(3));

        state.store_latest(Arc::clone(&record));

        assert!(
            state
                .handle()
                .window(Time::from_nanos(0), Time::from_nanos(10))
                .is_empty()
        );
        assert!(Arc::ptr_eq(&record, &state.handle().latest().unwrap()));
    }

    #[test]
    fn drain_events_returns_and_clears_events() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));

        state.push_event(DebugEvent::Diagnostic("connected".to_string()));

        let events = state.handle().drain_events();
        assert!(matches!(&events[..], [DebugEvent::Diagnostic(message)] if message == "connected"));
        assert!(state.handle().drain_events().is_empty());
    }

    #[test]
    fn status_becomes_ready_after_storing_latest() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));

        state.store_latest(sample_record(NonClonePayload(1)));

        assert_eq!(state.handle().status().status, SubscriptionStatus::Ready);
    }

    #[test]
    fn recovery_from_error_emits_status_changed_and_value_updated() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        state.set_receive_error(
            SubscriptionStatus::DecodeError,
            "failed to deserialize sample".to_string(),
        );
        state.handle().drain_events();

        state.store_latest(sample_record(NonClonePayload(1)));

        let events = state.handle().drain_events();
        assert!(matches!(
            events.as_slice(),
            [DebugEvent::StatusChanged, DebugEvent::ValueUpdated]
        ));
    }

    #[test]
    fn dropping_state_closes_cancellation_receiver_and_weak_state() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let mut cancellation = state.cancellation_receiver();
        let weak_state = Arc::downgrade(&state);

        drop(state);

        assert!(weak_state.upgrade().is_none());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        assert!(runtime.block_on(cancellation.changed()).is_err());
    }
}

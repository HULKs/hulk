use std::sync::Arc;

use arc_swap::ArcSwapOption;
use parking_lot::Mutex;
use ros_z::dynamic::DynamicPayload;
use ros_z::time::Time;
use serde_json::Value;
use tokio::{sync::watch, task::AbortHandle};

use crate::{
    DebugEvent, JsonRenderPolicy, RetentionPolicy, SampleRecord, SubscriptionStatus,
    SubscriptionStatusSnapshot, event::EventBuffer, history::TimeIndexedHistory,
    json::dynamic_payload_to_json,
};

pub(crate) trait ManagedSubscription: Send + Sync {
    fn close(&self);
}

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

pub struct JsonSubscriptionHandle {
    dynamic: SubscriptionHandle<DynamicPayload>,
    policy: JsonRenderPolicy,
}

impl JsonSubscriptionHandle {
    pub(crate) fn new(
        dynamic: SubscriptionHandle<DynamicPayload>,
        policy: JsonRenderPolicy,
    ) -> Self {
        Self { dynamic, policy }
    }

    pub fn status(&self) -> SubscriptionStatusSnapshot {
        self.dynamic.status()
    }

    pub fn latest_json(&self) -> Option<Value> {
        self.dynamic
            .latest()
            .map(|record| dynamic_payload_to_json(&record.value, self.policy))
    }

    pub fn window_json(&self, start: Time, end: Time) -> Vec<Value> {
        self.dynamic
            .window(start, end)
            .iter()
            .map(|record| dynamic_payload_to_json(&record.value, self.policy))
            .collect()
    }

    pub fn drain_events(&self) -> Vec<DebugEvent> {
        self.dynamic.drain_events()
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
    receive_task: Option<AbortHandle>,
}

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
                receive_task: None,
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
        let mut receiver = self.cancellation_tx.subscribe();
        if self.meta.lock().status.status == SubscriptionStatus::Closed {
            receiver.mark_changed();
        }
        receiver
    }

    pub(crate) fn set_receive_task(&self, abort_handle: AbortHandle) {
        let mut meta = self.meta.lock();
        if meta.status.status == SubscriptionStatus::Closed {
            drop(meta);
            abort_handle.abort();
            return;
        }

        if let Some(previous) = meta.receive_task.replace(abort_handle) {
            previous.abort();
        }
    }

    pub(crate) fn close(&self) {
        let mut meta = self.meta.lock();
        if meta.status.status == SubscriptionStatus::Closed {
            return;
        }
        meta.status.status = SubscriptionStatus::Closed;
        meta.status.message = None;
        meta.events.push(DebugEvent::StatusChanged);
        let receive_task = meta.receive_task.take();
        drop(meta);

        let _ = self.cancellation_tx.send(());
        if let Some(receive_task) = receive_task {
            receive_task.abort();
        }
    }

    pub(crate) fn store_latest(&self, record: Arc<SampleRecord<V>>) {
        let mut meta = self.meta.lock();
        if meta.status.status == SubscriptionStatus::Closed {
            return;
        }

        let history_updated = self.history.is_some();
        if let Some(history) = &self.history {
            history.lock().insert(Arc::clone(&record));
        }

        let latest_updated = self
            .latest
            .load_full()
            .is_none_or(|latest| record.source_time >= latest.source_time);
        if latest_updated {
            self.latest.store(Some(record));
        }

        let status_changed = meta.status.status != SubscriptionStatus::Ready;
        meta.status.status = SubscriptionStatus::Ready;
        meta.status.message = None;
        if status_changed {
            meta.events.push(DebugEvent::StatusChanged);
        }
        if latest_updated || history_updated {
            meta.events.push(DebugEvent::ValueUpdated);
        }
    }

    pub(crate) fn set_receive_error(&self, status: SubscriptionStatus, message: String) {
        let mut meta = self.meta.lock();
        if meta.status.status == SubscriptionStatus::Closed {
            return;
        }

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

impl<V> ManagedSubscription for SubscriptionState<V>
where
    V: Send + Sync,
{
    fn close(&self) {
        SubscriptionState::close(self);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use ros_z::{
        dynamic::{DynamicPayload, DynamicValue, PrimitiveTypeDef, SchemaBundle, TypeDef},
        time::Time,
    };

    use super::{JsonSubscriptionHandle, SubscriptionState};
    use crate::{
        DebugEvent, JsonRenderPolicy, RetentionPolicy, SampleRecord, SubscriptionStatus,
        SubscriptionStatusSnapshot, TopicSelector,
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

    fn dynamic_payload(value: i32) -> DynamicPayload {
        DynamicPayload::new(
            Arc::new(SchemaBundle {
                root: TypeDef::Primitive(PrimitiveTypeDef::I32),
                definitions: Default::default(),
            }),
            DynamicValue::Int32(value),
        )
        .unwrap()
    }

    fn dynamic_record_at(value: i32, source_time: Time) -> Arc<SampleRecord<DynamicPayload>> {
        Arc::new(SampleRecord {
            value: dynamic_payload(value),
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
    fn latest_uses_newest_source_time_not_receive_order() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();

        state.store_latest(sample_record_at(NonClonePayload(1), Time::from_nanos(3)));
        state.store_latest(sample_record_at(NonClonePayload(2), Time::from_nanos(2)));

        let latest = handle.latest().unwrap();
        assert_eq!(latest.value.0, 1);
        assert_eq!(latest.source_time, Time::from_nanos(3));
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

    #[test]
    fn close_sets_closed_status_emits_event_and_signals_cancellation() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut cancellation = state.cancellation_receiver();

        state.close();

        assert_eq!(handle.status().status, SubscriptionStatus::Closed);
        assert!(matches!(
            handle.drain_events().as_slice(),
            [DebugEvent::StatusChanged]
        ));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        assert!(
            runtime
                .block_on(async {
                    tokio::time::timeout(Duration::from_secs(1), cancellation.changed()).await
                })
                .unwrap()
                .is_ok()
        );
    }

    #[tokio::test]
    async fn cancellation_receiver_created_after_close_observes_cancellation() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        state.close();
        let mut cancellation = state.cancellation_receiver();

        tokio::time::timeout(Duration::from_millis(10), cancellation.changed())
            .await
            .expect("closed state should wake late cancellation receivers")
            .expect("sender should remain alive");
    }

    #[tokio::test]
    async fn close_aborts_registered_receive_task() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let task = tokio::spawn(std::future::pending::<()>());
        state.set_receive_task(task.abort_handle());

        state.close();

        assert!(task.await.unwrap_err().is_cancelled());
    }

    #[test]
    fn store_latest_after_close_keeps_closed_terminal() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();

        state.close();
        state.store_latest(sample_record(NonClonePayload(1)));

        assert_eq!(handle.status().status, SubscriptionStatus::Closed);
        assert!(handle.latest().is_none());
        assert!(matches!(
            handle.drain_events().as_slice(),
            [DebugEvent::StatusChanged]
        ));
    }

    #[test]
    fn receive_error_after_close_keeps_closed_terminal() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();

        state.close();
        state.set_receive_error(
            SubscriptionStatus::DecodeError,
            "late decode failure".to_string(),
        );

        let status = handle.status();
        assert_eq!(status.status, SubscriptionStatus::Closed);
        assert_eq!(status.message, None);
        assert!(matches!(
            handle.drain_events().as_slice(),
            [DebugEvent::StatusChanged]
        ));
    }

    #[test]
    fn close_is_idempotent() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();

        state.close();
        state.close();

        assert_eq!(handle.status().status, SubscriptionStatus::Closed);
        assert!(matches!(
            handle.drain_events().as_slice(),
            [DebugEvent::StatusChanged]
        ));
    }

    #[test]
    fn json_handle_projects_latest_dynamic_payload() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        state.store_latest(dynamic_record_at(42, Time::zero()));
        let handle = JsonSubscriptionHandle::new(state.handle(), JsonRenderPolicy::default());

        assert_eq!(handle.latest_json(), Some(serde_json::json!(42)));
    }

    #[test]
    fn json_handle_drains_underlying_dynamic_events() {
        let state = Arc::new(SubscriptionState::<DynamicPayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        state.push_event(DebugEvent::Diagnostic("connected".to_string()));
        let handle = JsonSubscriptionHandle::new(state.handle(), JsonRenderPolicy::default());

        let events = handle.drain_events();

        assert!(matches!(&events[..], [DebugEvent::Diagnostic(message)] if message == "connected"));
        assert!(handle.drain_events().is_empty());
    }

    #[test]
    fn json_handle_projects_dynamic_payload_window() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::TimeWindow {
                duration: Duration::from_secs(10),
                max_samples: None,
            },
        ));
        state.store_latest(dynamic_record_at(1, Time::from_nanos(1)));
        state.store_latest(dynamic_record_at(2, Time::from_nanos(2)));
        let handle = JsonSubscriptionHandle::new(state.handle(), JsonRenderPolicy::default());

        assert_eq!(
            handle.window_json(Time::from_nanos(1), Time::from_nanos(2)),
            vec![serde_json::json!(1), serde_json::json!(2)]
        );
    }
}

use std::sync::Arc;

use arc_swap::ArcSwapOption;
use parking_lot::Mutex;
use ros_z::dynamic::DynamicPayload;
use ros_z::time::Time;
use serde_json::Value;
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    DebugEvent, JsonRenderPolicy, RetentionPolicy, SampleRecord, SubscriptionStatus,
    SubscriptionStatusSnapshot, dynamic_payload_to_json, event::EventBuffer,
    history::TimeIndexedHistory,
};

pub(crate) trait ManagedSubscription: Send + Sync {
    fn close(&self);
}

/// Handle for reading retained subscription state.
///
/// Cloning the handle keeps the subscription alive. Dropping the last handle
/// cancels the receive task.
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
    /// Return the current status snapshot.
    pub fn status(&self) -> SubscriptionStatusSnapshot {
        self.state.status()
    }

    /// Return the latest retained sample, if one has arrived.
    pub fn latest(&self) -> Option<Arc<SampleRecord<V>>> {
        self.state.latest()
    }

    /// Return retained samples whose source time falls inside `[start, end]`.
    ///
    /// Handles with [`RetentionPolicy::LatestOnly`] return an empty window.
    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<SampleRecord<V>>> {
        self.state.window(start, end)
    }

    /// Drain queued status/value/diagnostic events.
    pub fn drain_events(&self) -> Vec<DebugEvent> {
        self.state.drain_events()
    }
}

/// Handle that renders retained dynamic payloads as JSON on demand.
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

    /// Return the current status snapshot.
    pub fn status(&self) -> SubscriptionStatusSnapshot {
        self.dynamic.status()
    }

    /// Render the latest retained dynamic payload as JSON.
    pub fn latest_json(&self) -> Option<Value> {
        self.dynamic
            .latest()
            .map(|record| dynamic_payload_to_json(&record.value, self.policy))
    }

    /// Render retained dynamic payloads in `[start, end]` as JSON.
    pub fn window_json(&self, start: Time, end: Time) -> Vec<Value> {
        self.dynamic
            .window(start, end)
            .iter()
            .map(|record| dynamic_payload_to_json(&record.value, self.policy))
            .collect()
    }

    /// Drain queued status/value/diagnostic events.
    pub fn drain_events(&self) -> Vec<DebugEvent> {
        self.dynamic.drain_events()
    }
}

pub(crate) struct SubscriptionState<V> {
    latest: ArcSwapOption<SampleRecord<V>>,
    history: Option<Mutex<TimeIndexedHistory<V>>>,
    meta: Mutex<SubscriptionMeta>,
    cancellation_token: CancellationToken,
}

struct SubscriptionMeta {
    status: SubscriptionStatusSnapshot,
    events: EventBuffer,
    receive_task: Option<AbortHandle>,
}

impl<V> SubscriptionState<V> {
    pub(crate) fn new(status: SubscriptionStatusSnapshot, retention: RetentionPolicy) -> Self {
        let history = match retention {
            RetentionPolicy::LatestOnly => None,
            RetentionPolicy::TimeWindow(_) => Some(Mutex::new(TimeIndexedHistory::new(retention))),
        };

        Self {
            latest: ArcSwapOption::empty(),
            history,
            meta: Mutex::new(SubscriptionMeta {
                status,
                events: EventBuffer::new(256),
                receive_task: None,
            }),
            cancellation_token: CancellationToken::new(),
        }
    }

    pub(crate) fn handle(self: &Arc<Self>) -> SubscriptionHandle<V> {
        SubscriptionHandle {
            state: Arc::clone(self),
        }
    }

    pub(crate) fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub(crate) fn set_receive_task(&self, abort_handle: AbortHandle) {
        let mut meta = self.meta.lock();
        if meta.status.status() == &SubscriptionStatus::Closed {
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
        if meta.status.status() == &SubscriptionStatus::Closed {
            return;
        }
        meta.status.set_status(SubscriptionStatus::Closed);
        meta.events.push(DebugEvent::StatusChanged);
        let receive_task = meta.receive_task.take();
        drop(meta);

        self.cancellation_token.cancel();
        if let Some(receive_task) = receive_task {
            receive_task.abort();
        }
    }

    pub(crate) fn store_latest(&self, record: Arc<SampleRecord<V>>) {
        let mut meta = self.meta.lock();
        if meta.status.status() == &SubscriptionStatus::Closed {
            return;
        }

        if let Some(history) = &self.history {
            history.lock().insert(Arc::clone(&record));
        }
        self.latest.store(Some(record));

        let status_changed = meta.status.status() != &SubscriptionStatus::Ready;
        meta.status.set_status(SubscriptionStatus::Ready);
        if status_changed {
            meta.events.push(DebugEvent::StatusChanged);
        }
        meta.events.push(DebugEvent::ValueUpdated);
    }

    pub(crate) fn set_receive_error(&self, status: SubscriptionStatus) {
        let mut meta = self.meta.lock();
        if meta.status.status() == &SubscriptionStatus::Closed {
            return;
        }

        let status_changed = !meta.status.status().is_same_kind(&status);
        let message = status.message().map(str::to_owned);
        meta.status.set_status(status);
        if status_changed {
            meta.events.push(DebugEvent::StatusChanged);
        }
        if let Some(message) = message {
            meta.events.push(DebugEvent::Diagnostic(message));
        }
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

impl<V> Drop for SubscriptionState<V> {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
        if let Some(receive_task) = self.meta.lock().receive_task.take() {
            receive_task.abort();
        }
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
        DebugEvent, JsonRenderPolicy, RetentionPolicy, SampleMetadata, SampleRecord,
        SubscriptionStatus, SubscriptionStatusSnapshot, TopicSelector,
    };

    struct NonClonePayload(u32);

    fn test_type_info() -> ros_z::TypeInfo {
        ros_z::TypeInfo::new("test_msgs::DebugValue", ros_z::SchemaHash::zero())
    }

    fn test_publication_id() -> ros_z::pubsub::PublicationId {
        ros_z::pubsub::Received {
            message: (),
            transport_time: None,
            source_time: Time::zero(),
            sequence_number: 1,
            source_global_id: ros_z::EndpointGlobalId::from([7; 16]),
        }
        .publication_id()
    }

    fn test_metadata() -> Arc<SampleMetadata> {
        Arc::new(SampleMetadata {
            requested_topic: TopicSelector::new("camera").unwrap(),
            resolved_topic: "/camera".to_string(),
            type_info: test_type_info(),
        })
    }

    fn sample_record_at(
        value: NonClonePayload,
        source_time: Time,
    ) -> Arc<SampleRecord<NonClonePayload>> {
        Arc::new(SampleRecord {
            value,
            source_time,
            transport_time: None,
            publication_id: test_publication_id(),
            metadata: test_metadata(),
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
            publication_id: test_publication_id(),
            metadata: test_metadata(),
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
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
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
    fn time_window_handle_returns_empty_for_inverted_window() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
        ));
        state.store_latest(sample_record_at(NonClonePayload(1), Time::from_nanos(1)));

        let records = state
            .handle()
            .window(Time::from_nanos(2), Time::from_nanos(1));

        assert!(records.is_empty());
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

        state.store_latest(sample_record(NonClonePayload(1)));

        let events = state.handle().drain_events();
        assert!(matches!(
            &events[..],
            [DebugEvent::StatusChanged, DebugEvent::ValueUpdated]
        ));
        assert!(state.handle().drain_events().is_empty());
    }

    #[test]
    fn status_becomes_ready_after_storing_latest() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));

        state.store_latest(sample_record(NonClonePayload(1)));

        assert_eq!(state.handle().status().status(), &SubscriptionStatus::Ready);
    }

    #[test]
    fn recovery_from_error_emits_status_changed_and_value_updated() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        state.set_receive_error(SubscriptionStatus::decode_error(
            "failed to deserialize sample",
        ));
        state.handle().drain_events();

        state.store_latest(sample_record(NonClonePayload(1)));

        let events = state.handle().drain_events();
        assert!(matches!(
            events.as_slice(),
            [DebugEvent::StatusChanged, DebugEvent::ValueUpdated]
        ));
    }

    #[test]
    fn close_sets_closed_status_and_emits_event() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();

        state.close();

        assert_eq!(handle.status().status(), &SubscriptionStatus::Closed);
        assert!(matches!(
            handle.drain_events().as_slice(),
            [DebugEvent::StatusChanged]
        ));
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

        assert_eq!(handle.status().status(), &SubscriptionStatus::Closed);
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
        state.set_receive_error(SubscriptionStatus::decode_error("late decode failure"));

        let status = handle.status();
        assert_eq!(status.status(), &SubscriptionStatus::Closed);
        assert_eq!(status.message(), None);
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

        assert_eq!(handle.status().status(), &SubscriptionStatus::Closed);
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
        state.store_latest(dynamic_record_at(1, Time::zero()));
        let handle = JsonSubscriptionHandle::new(state.handle(), JsonRenderPolicy::default());

        let events = handle.drain_events();

        assert!(matches!(
            &events[..],
            [DebugEvent::StatusChanged, DebugEvent::ValueUpdated]
        ));
        assert!(handle.drain_events().is_empty());
    }

    #[test]
    fn json_handle_projects_dynamic_payload_window() {
        let state = Arc::new(SubscriptionState::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
        ));
        state.store_latest(dynamic_record_at(1, Time::from_nanos(1)));
        state.store_latest(dynamic_record_at(2, Time::from_nanos(2)));
        let handle = JsonSubscriptionHandle::new(state.handle(), JsonRenderPolicy::default());

        assert_eq!(
            handle.window_json(Time::from_nanos(1), Time::from_nanos(2)),
            vec![serde_json::json!(1), serde_json::json!(2)]
        );
    }

    #[test]
    fn dropping_last_handle_cancels_subscription_state() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let cancellation = state.cancellation_token();
        let handle = state.handle();

        drop(state);
        assert!(!cancellation.is_cancelled());

        drop(handle);

        assert!(cancellation.is_cancelled());
    }
}

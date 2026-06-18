use std::sync::Arc;

use arc_swap::ArcSwapOption;
use parking_lot::Mutex;
use ros_z::{dynamic::DynamicPayload, time::Time};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    JsonRenderPolicy, RetentionPolicy, SampleRecord, SubscriptionStatus,
    SubscriptionStatusSnapshot, SubscriptionUpdate, SubscriptionUpdateReceiver,
    dynamic_payload_to_json, history::TimeIndexedHistory,
};

const UPDATE_BUFFER_CAPACITY: usize = 256;

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

    /// Subscribe to status, data, and diagnostic update notifications.
    ///
    /// Terminal [`SubscriptionStatus::Closed`] is delivered as a
    /// [`SubscriptionUpdate::StatusChanged`] update, not as end-of-stream while
    /// handles keep the subscription state alive. Callers that want to stop at
    /// terminal close should break when they observe that status.
    pub fn subscribe_updates(&self) -> SubscriptionUpdateReceiver {
        self.state.subscribe_updates()
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

    /// Subscribe to status, data, and diagnostic update notifications.
    ///
    /// See [`SubscriptionHandle::subscribe_updates`] for terminal close
    /// semantics.
    pub fn subscribe_updates(&self) -> SubscriptionUpdateReceiver {
        self.dynamic.subscribe_updates()
    }
}

pub(crate) struct SubscriptionState<V> {
    latest: ArcSwapOption<SampleRecord<V>>,
    history: Option<Mutex<TimeIndexedHistory<V>>>,
    meta: Mutex<SubscriptionMeta>,
    updates: broadcast::Sender<SubscriptionUpdate>,
    cancellation_token: CancellationToken,
}

struct SubscriptionMeta {
    status: SubscriptionStatusSnapshot,
    receive_task: Option<AbortHandle>,
}

impl<V> SubscriptionState<V> {
    pub(crate) fn new(status: SubscriptionStatusSnapshot, retention: RetentionPolicy) -> Self {
        let history = match retention {
            RetentionPolicy::LatestOnly => None,
            RetentionPolicy::TimeWindow(_) => Some(Mutex::new(TimeIndexedHistory::new(retention))),
        };

        let (updates, _) = broadcast::channel(UPDATE_BUFFER_CAPACITY);

        Self {
            latest: ArcSwapOption::empty(),
            history,
            meta: Mutex::new(SubscriptionMeta {
                status,
                receive_task: None,
            }),
            updates,
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
        let snapshot = meta.status.clone();
        let receive_task = meta.receive_task.take();
        self.publish_update(SubscriptionUpdate::StatusChanged(snapshot));
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
        let snapshot = status_changed.then(|| meta.status.clone());

        if let Some(snapshot) = snapshot {
            self.publish_update(SubscriptionUpdate::StatusChanged(snapshot));
        }
        self.publish_update(SubscriptionUpdate::DataChanged);
        drop(meta);
    }

    pub(crate) fn set_receive_error(&self, status: SubscriptionStatus) {
        let mut meta = self.meta.lock();
        if meta.status.status() == &SubscriptionStatus::Closed {
            return;
        }

        let status_changed = meta.status.status() != &status;
        let message = status.message().map(str::to_owned);
        meta.status.set_status(status);
        let snapshot = status_changed.then(|| meta.status.clone());

        if let Some(snapshot) = snapshot {
            self.publish_update(SubscriptionUpdate::StatusChanged(snapshot));
        }
        if let Some(message) = message {
            self.publish_update(SubscriptionUpdate::Diagnostic(message));
        }
        drop(meta);
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

    fn subscribe_updates(&self) -> SubscriptionUpdateReceiver {
        SubscriptionUpdateReceiver::new(self.updates.subscribe())
    }

    fn publish_update(&self, update: SubscriptionUpdate) {
        let _ = self.updates.send(update);
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
        JsonRenderPolicy, RetentionPolicy, SampleMetadata, SampleRecord, SubscriptionStatus,
        SubscriptionStatusSnapshot, SubscriptionUpdate, TopicSelector,
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
    fn update_receiver_reports_data_changed_after_store_latest() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates();

        state.store_latest(sample_record(NonClonePayload(1)));

        let first_update = updates.try_recv();
        assert!(matches!(
            first_update,
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &SubscriptionStatus::Ready
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::DataChanged))
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn update_receiver_reports_diagnostic_messages() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates();

        state.set_receive_error(SubscriptionStatus::decode_error(
            "failed to deserialize sample",
        ));

        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if matches!(snapshot.status(), SubscriptionStatus::DecodeError { .. })
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::Diagnostic(message)))
                if message == "failed to deserialize sample"
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn update_receiver_reports_status_changed_for_repeated_error_with_new_message() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates();

        state.set_receive_error(SubscriptionStatus::decode_error("first decode failure"));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.message() == Some("first decode failure")
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::Diagnostic(message)))
                if message == "first decode failure"
        ));

        state.set_receive_error(SubscriptionStatus::decode_error("second decode failure"));

        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.message() == Some("second decode failure")
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::Diagnostic(message)))
                if message == "second decode failure"
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
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
    fn update_receiver_reports_closed_status() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates();

        state.close();

        assert_eq!(handle.status().status(), &SubscriptionStatus::Closed);
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &SubscriptionStatus::Closed
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
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
        let mut updates = handle.subscribe_updates();

        state.close();
        state.store_latest(sample_record(NonClonePayload(1)));

        assert_eq!(handle.status().status(), &SubscriptionStatus::Closed);
        assert!(handle.latest().is_none());
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &SubscriptionStatus::Closed
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn receive_error_after_close_keeps_closed_terminal() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates();

        state.close();
        state.set_receive_error(SubscriptionStatus::decode_error("late decode failure"));

        let status = handle.status();
        assert_eq!(status.status(), &SubscriptionStatus::Closed);
        assert_eq!(status.message(), None);
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &SubscriptionStatus::Closed
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn close_is_idempotent() {
        let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates();

        state.close();
        state.close();

        assert_eq!(handle.status().status(), &SubscriptionStatus::Closed);
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &SubscriptionStatus::Closed
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
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
    fn json_handle_subscribes_to_underlying_dynamic_updates() {
        let state = Arc::new(SubscriptionState::<DynamicPayload>::new(
            SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let handle = JsonSubscriptionHandle::new(state.handle(), JsonRenderPolicy::default());
        let mut updates = handle.subscribe_updates();

        state.store_latest(dynamic_record_at(1, Time::zero()));

        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &SubscriptionStatus::Ready
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(SubscriptionUpdate::DataChanged))
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
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

    #[test]
    fn update_receiver_closes_after_subscription_state_is_dropped() {
        let mut updates = {
            let state = Arc::new(SubscriptionState::<NonClonePayload>::new(
                SubscriptionStatusSnapshot::new(SubscriptionStatus::WaitingForFirstSample),
                RetentionPolicy::LatestOnly,
            ));

            state.handle().subscribe_updates()
        };

        assert!(matches!(
            updates.try_recv(),
            Err(crate::SubscriptionUpdateClosed)
        ));
    }
}

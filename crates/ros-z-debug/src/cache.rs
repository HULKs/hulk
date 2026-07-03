use std::{num::NonZeroUsize, sync::Arc};

use arc_swap::ArcSwapOption;
use parking_lot::Mutex;
use ros_z::{dynamic::DynamicPayload, time::Time};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::{
    CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot, CachedSubscriptionUpdate,
    CachedSubscriptionUpdateClosed, CachedSubscriptionUpdateReceiver, JsonRenderPolicy,
    RetentionPolicy, SampleRecord,
    history::TimeIndexedHistory,
    sample::{dynamic_record_json_value, dynamic_record_to_json_sample},
};

/// Handle for reading retained subscription state.
///
/// Cloning the handle keeps the subscription alive. Dropping the last handle
/// cancels the receive task.
pub struct CachedSubscription<V> {
    state: Arc<CachedSubscriptionState<V>>,
}

impl<V> Clone for CachedSubscription<V> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl<V> CachedSubscription<V> {
    /// Return the current status snapshot.
    pub fn status(&self) -> CachedSubscriptionStatusSnapshot {
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

    /// Subscribe to future status and data update notifications.
    ///
    /// The receiver is a live stream and does not replay updates that happened
    /// before subscription. When the subscription closes, the update stream ends;
    /// call [`Self::status`] to inspect the terminal status and [`Self::latest`] or
    /// [`Self::window`] to inspect retained data.
    pub fn subscribe_updates(
        &self,
    ) -> Result<CachedSubscriptionUpdateReceiver, CachedSubscriptionUpdateClosed> {
        self.state.subscribe_updates()
    }

    pub(crate) fn close_retaining_samples(&self) {
        self.state.close();
    }
}

/// Handle that renders retained dynamic payloads as JSON on demand.
pub struct CachedJsonSubscription {
    dynamic: CachedSubscription<DynamicPayload>,
    policy: JsonRenderPolicy,
}

impl CachedJsonSubscription {
    pub(crate) fn new(
        dynamic: CachedSubscription<DynamicPayload>,
        policy: JsonRenderPolicy,
    ) -> Self {
        Self { dynamic, policy }
    }

    /// Return the current status snapshot.
    pub fn status(&self) -> CachedSubscriptionStatusSnapshot {
        self.dynamic.status()
    }

    /// Render the latest retained dynamic payload as JSON.
    pub fn latest_json(&self) -> Option<Value> {
        self.dynamic
            .latest()
            .map(|record| dynamic_record_json_value(record.as_ref(), self.policy))
    }

    /// Render the latest retained dynamic payload as JSON with sample metadata.
    pub fn latest_json_record(&self) -> Option<crate::SampleRecord<Value>> {
        self.dynamic
            .latest()
            .map(|record| dynamic_record_to_json_sample(record, self.policy))
    }

    /// Render retained dynamic payloads in `[start, end]` as JSON.
    pub fn window_json(&self, start: Time, end: Time) -> Vec<Value> {
        self.dynamic
            .window(start, end)
            .iter()
            .map(|record| dynamic_record_json_value(record.as_ref(), self.policy))
            .collect()
    }

    /// Render retained dynamic payloads in `[start, end]` as JSON records.
    pub fn window_json_records(&self, start: Time, end: Time) -> Vec<crate::SampleRecord<Value>> {
        self.dynamic
            .window(start, end)
            .into_iter()
            .map(|record| dynamic_record_to_json_sample(record, self.policy))
            .collect()
    }

    /// Subscribe to future status and data update notifications.
    ///
    /// The receiver is a live stream and does not replay updates that happened
    /// before subscription. When the subscription closes, the update stream ends;
    /// call [`Self::status`] to inspect terminal status and [`Self::latest_json`]
    /// or [`Self::window_json`] to inspect retained data.
    pub fn subscribe_updates(
        &self,
    ) -> Result<CachedSubscriptionUpdateReceiver, CachedSubscriptionUpdateClosed> {
        self.dynamic.subscribe_updates()
    }
}

pub(crate) struct CachedSubscriptionState<V> {
    latest: ArcSwapOption<SampleRecord<V>>,
    history: Option<Mutex<TimeIndexedHistory<V>>>,
    meta: Mutex<CachedSubscriptionMeta>,
    cancellation_token: CancellationToken,
}

enum CachedSubscriptionMeta {
    Open {
        status: CachedSubscriptionStatusSnapshot,
        updates: broadcast::Sender<CachedSubscriptionUpdate>,
    },
    Closed {
        status: CachedSubscriptionStatusSnapshot,
    },
}

impl<V> CachedSubscriptionState<V> {
    pub(crate) fn new(
        status: CachedSubscriptionStatusSnapshot,
        retention: RetentionPolicy,
        update_buffer_capacity: NonZeroUsize,
    ) -> Self {
        let history = match retention {
            RetentionPolicy::LatestOnly => None,
            RetentionPolicy::TimeWindow(_) => Some(Mutex::new(TimeIndexedHistory::new(retention))),
        };

        let (updates, _) = broadcast::channel(update_buffer_capacity.get());

        Self {
            latest: ArcSwapOption::empty(),
            history,
            meta: Mutex::new(CachedSubscriptionMeta::Open { status, updates }),
            cancellation_token: CancellationToken::new(),
        }
    }

    pub(crate) fn spawn<F, Fut>(
        status: CachedSubscriptionStatusSnapshot,
        retention: RetentionPolicy,
        update_buffer_capacity: NonZeroUsize,
        receive_loop: F,
    ) -> Arc<Self>
    where
        F: FnOnce(std::sync::Weak<Self>, CancellationToken) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
        V: Send + Sync + 'static,
    {
        let state = Arc::new(Self::new(status, retention, update_buffer_capacity));
        let weak_state = Arc::downgrade(&state);
        let cancellation = state.cancellation_token();

        tokio::spawn(receive_loop(weak_state, cancellation));

        state
    }

    pub(crate) fn handle(self: &Arc<Self>) -> CachedSubscription<V> {
        CachedSubscription {
            state: Arc::clone(self),
        }
    }

    pub(crate) fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub(crate) fn close(&self) {
        let mut meta = self.meta.lock();
        let closed_status = match &mut *meta {
            CachedSubscriptionMeta::Open { status, .. } => {
                status.set_status(CachedSubscriptionStatus::Closed);
                status.clone()
            }
            CachedSubscriptionMeta::Closed { .. } => return,
        };
        let updates = match std::mem::replace(
            &mut *meta,
            CachedSubscriptionMeta::Closed {
                status: closed_status,
            },
        ) {
            CachedSubscriptionMeta::Open { updates, .. } => updates,
            CachedSubscriptionMeta::Closed { .. } => unreachable!("closed state was handled above"),
        };
        drop(meta);

        drop(updates);
        self.cancellation_token.cancel();
    }

    pub(crate) fn store_latest(&self, record: Arc<SampleRecord<V>>) {
        let mut meta = self.meta.lock();
        let CachedSubscriptionMeta::Open { status, updates } = &mut *meta else {
            return;
        };

        if let Some(history) = &self.history {
            history.lock().insert(Arc::clone(&record));
        }
        self.latest.store(Some(record));

        let status_changed = status.status() != &CachedSubscriptionStatus::Ready;
        status.set_status(CachedSubscriptionStatus::Ready);
        let snapshot = status_changed.then(|| status.clone());

        if let Some(snapshot) = snapshot {
            let _ = updates.send(CachedSubscriptionUpdate::StatusChanged(snapshot));
        }
        let _ = updates.send(CachedSubscriptionUpdate::DataChanged);
    }

    pub(crate) fn set_receive_error(&self, status: CachedSubscriptionStatus) {
        let mut meta = self.meta.lock();
        let CachedSubscriptionMeta::Open {
            status: current_status,
            updates,
        } = &mut *meta
        else {
            return;
        };

        let status_changed = current_status.status() != &status;
        current_status.set_status(status);
        let snapshot = status_changed.then(|| current_status.clone());

        if let Some(snapshot) = snapshot {
            let _ = updates.send(CachedSubscriptionUpdate::StatusChanged(snapshot));
        }
    }

    fn status(&self) -> CachedSubscriptionStatusSnapshot {
        match &*self.meta.lock() {
            CachedSubscriptionMeta::Open { status, .. }
            | CachedSubscriptionMeta::Closed { status } => status.clone(),
        }
    }

    fn latest(&self) -> Option<Arc<SampleRecord<V>>> {
        self.latest.load_full()
    }

    fn window(&self, start: Time, end: Time) -> Vec<Arc<SampleRecord<V>>> {
        self.history
            .as_ref()
            .map_or_else(Vec::new, |history| history.lock().window(start, end))
    }

    fn subscribe_updates(
        &self,
    ) -> Result<CachedSubscriptionUpdateReceiver, CachedSubscriptionUpdateClosed> {
        let meta = self.meta.lock();
        let CachedSubscriptionMeta::Open { updates, .. } = &*meta else {
            return Err(CachedSubscriptionUpdateClosed);
        };

        Ok(CachedSubscriptionUpdateReceiver::new(updates.subscribe()))
    }
}

impl<V> Drop for CachedSubscriptionState<V> {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, sync::Arc, time::Duration};

    use ros_z::{
        dynamic::{DynamicPayload, DynamicValue, PrimitiveTypeDef, SchemaBundle, TypeDef},
        time::Time,
    };

    use super::{CachedJsonSubscription, CachedSubscriptionState};
    use crate::{
        CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot, CachedSubscriptionUpdate,
        CachedSubscriptionUpdateClosed, JsonRenderPolicy, RetentionPolicy, SampleMetadata,
        SampleRecord, TopicReference,
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
            topic_reference: TopicReference::new("camera").unwrap(),
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
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let record = sample_record(NonClonePayload(7));

        state.store_latest(Arc::clone(&record));

        let latest = state.handle().latest().unwrap();
        assert!(Arc::ptr_eq(&record, &latest));
        assert_eq!(latest.value.0, 7);
    }

    #[test]
    fn time_window_handle_returns_stored_records_in_window() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
            NonZeroUsize::new(256).unwrap(),
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
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
            NonZeroUsize::new(256).unwrap(),
        ));
        state.store_latest(sample_record_at(NonClonePayload(1), Time::from_nanos(1)));

        let records = state
            .handle()
            .window(Time::from_nanos(2), Time::from_nanos(1));

        assert!(records.is_empty());
    }

    #[test]
    fn latest_only_handle_returns_empty_window_but_keeps_latest() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
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
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.store_latest(sample_record(NonClonePayload(1)));

        let first_update = updates.try_recv();
        assert!(matches!(
            first_update,
            Ok(Some(CachedSubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &CachedSubscriptionStatus::Ready
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::DataChanged))
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn update_receiver_does_not_replay_updates_sent_before_subscription() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();

        state.store_latest(sample_record(NonClonePayload(1)));
        let mut updates = handle.subscribe_updates().unwrap();

        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn update_receiver_reports_error_message_in_status_update() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.set_receive_error(CachedSubscriptionStatus::decode_error(
            "failed to deserialize sample",
        ));

        assert!(matches!(
            updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::StatusChanged(snapshot)))
                if matches!(snapshot.status(), CachedSubscriptionStatus::DecodeError { .. })
                    && snapshot.message() == Some("failed to deserialize sample")
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn update_receiver_reports_status_changed_for_repeated_error_with_new_message() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.set_receive_error(CachedSubscriptionStatus::decode_error(
            "first decode failure",
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.message() == Some("first decode failure")
        ));

        state.set_receive_error(CachedSubscriptionStatus::decode_error(
            "second decode failure",
        ));

        assert!(matches!(
            updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.message() == Some("second decode failure")
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn status_becomes_ready_after_storing_latest() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));

        state.store_latest(sample_record(NonClonePayload(1)));

        assert_eq!(
            state.handle().status().status(),
            &CachedSubscriptionStatus::Ready
        );
    }

    #[test]
    fn update_receiver_closes_when_subscription_closes() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.close();

        assert_eq!(handle.status().status(), &CachedSubscriptionStatus::Closed);
        assert!(matches!(
            updates.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }

    #[test]
    fn subscribing_after_close_returns_closed() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();

        state.close();

        assert!(matches!(
            handle.subscribe_updates(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }

    #[test]
    fn close_keeps_previously_retained_latest_sample() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let record = sample_record(NonClonePayload(11));

        state.store_latest(Arc::clone(&record));
        state.close();

        let latest = handle.latest().unwrap();
        assert!(Arc::ptr_eq(&latest, &record));
        assert_eq!(handle.status().status(), &CachedSubscriptionStatus::Closed);
    }

    #[test]
    fn close_keeps_previously_retained_time_window_samples() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let first = sample_record_at(NonClonePayload(1), Time::from_nanos(1));
        let second = sample_record_at(NonClonePayload(2), Time::from_nanos(2));

        state.store_latest(Arc::clone(&first));
        state.store_latest(Arc::clone(&second));
        state.close();

        let records = handle.window(Time::from_nanos(1), Time::from_nanos(2));
        assert_eq!(records.len(), 2);
        assert!(Arc::ptr_eq(&records[0], &first));
        assert!(Arc::ptr_eq(&records[1], &second));
        assert_eq!(handle.status().status(), &CachedSubscriptionStatus::Closed);
    }

    #[test]
    fn close_does_not_emit_closed_status_update() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.close();

        assert!(matches!(
            updates.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }

    #[tokio::test]
    async fn spawned_receive_task_exits_when_subscription_closes() {
        let (exited_sender, exited_receiver) = tokio::sync::oneshot::channel();
        let state = CachedSubscriptionState::<NonClonePayload>::spawn(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
            move |_state, cancellation| async move {
                cancellation.cancelled().await;
                let _ = exited_sender.send(());
            },
        );

        state.close();

        tokio::time::timeout(Duration::from_secs(1), exited_receiver)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn spawned_receive_task_exits_when_last_handle_drops() {
        let (exited_sender, exited_receiver) = tokio::sync::oneshot::channel();
        let state = CachedSubscriptionState::<NonClonePayload>::spawn(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
            move |_state, cancellation| async move {
                cancellation.cancelled().await;
                let _ = exited_sender.send(());
            },
        );
        let handle = state.handle();

        drop(state);
        drop(handle);

        tokio::time::timeout(Duration::from_secs(1), exited_receiver)
            .await
            .unwrap()
            .unwrap();
    }

    #[test]
    fn store_latest_after_close_keeps_closed_terminal() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.close();
        state.store_latest(sample_record(NonClonePayload(1)));

        assert_eq!(handle.status().status(), &CachedSubscriptionStatus::Closed);
        assert!(handle.latest().is_none());
        assert!(matches!(
            updates.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }

    #[test]
    fn receive_error_after_close_keeps_closed_terminal() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.close();
        state.set_receive_error(CachedSubscriptionStatus::decode_error(
            "late decode failure",
        ));

        let status = handle.status();
        assert_eq!(status.status(), &CachedSubscriptionStatus::Closed);
        assert_eq!(status.message(), None);
        assert!(matches!(
            updates.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }

    #[test]
    fn close_is_idempotent() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = state.handle();
        let mut updates = handle.subscribe_updates().unwrap();

        state.close();
        state.close();

        assert_eq!(handle.status().status(), &CachedSubscriptionStatus::Closed);
        assert!(matches!(
            updates.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }

    #[test]
    fn json_handle_projects_latest_dynamic_payload() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        state.store_latest(dynamic_record_at(42, Time::zero()));
        let handle = CachedJsonSubscription::new(state.handle(), JsonRenderPolicy::default());

        assert_eq!(handle.latest_json(), Some(serde_json::json!(42)));
    }

    #[test]
    fn json_handle_projects_latest_dynamic_payload_record() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let source = dynamic_record_at(42, Time::from_nanos(7));
        state.store_latest(Arc::clone(&source));
        let handle = CachedJsonSubscription::new(state.handle(), JsonRenderPolicy::default());

        let record = handle
            .latest_json_record()
            .expect("latest JSON record should be available");

        assert_eq!(record.value, serde_json::json!(42));
        assert_eq!(record.source_time, source.source_time);
        assert_eq!(record.transport_time, source.transport_time);
        assert_eq!(record.publication_id, source.publication_id);
        assert!(Arc::ptr_eq(&record.metadata, &source.metadata));
    }

    #[test]
    fn json_handle_subscribes_to_underlying_dynamic_updates() {
        let state = Arc::new(CachedSubscriptionState::<DynamicPayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
        ));
        let handle = CachedJsonSubscription::new(state.handle(), JsonRenderPolicy::default());
        let mut updates = handle.subscribe_updates().unwrap();

        state.store_latest(dynamic_record_at(1, Time::zero()));

        assert!(matches!(
            updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::StatusChanged(snapshot)))
                if snapshot.status() == &CachedSubscriptionStatus::Ready
        ));
        assert!(matches!(
            updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::DataChanged))
        ));
        assert!(matches!(updates.try_recv(), Ok(None)));
    }

    #[test]
    fn json_handle_projects_dynamic_payload_window() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
            NonZeroUsize::new(256).unwrap(),
        ));
        state.store_latest(dynamic_record_at(1, Time::from_nanos(1)));
        state.store_latest(dynamic_record_at(2, Time::from_nanos(2)));
        let handle = CachedJsonSubscription::new(state.handle(), JsonRenderPolicy::default());

        assert_eq!(
            handle.window_json(Time::from_nanos(1), Time::from_nanos(2)),
            vec![serde_json::json!(1), serde_json::json!(2)]
        );
    }

    #[test]
    fn json_handle_projects_dynamic_payload_window_records() {
        let state = Arc::new(CachedSubscriptionState::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::time_window(Duration::from_secs(10)).unwrap(),
            NonZeroUsize::new(256).unwrap(),
        ));
        let first = dynamic_record_at(1, Time::from_nanos(1));
        let second = dynamic_record_at(2, Time::from_nanos(2));
        state.store_latest(Arc::clone(&first));
        state.store_latest(Arc::clone(&second));
        let handle = CachedJsonSubscription::new(state.handle(), JsonRenderPolicy::default());

        let records = handle.window_json_records(Time::from_nanos(1), Time::from_nanos(2));

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].value, serde_json::json!(1));
        assert_eq!(records[0].source_time, first.source_time);
        assert!(Arc::ptr_eq(&records[0].metadata, &first.metadata));
        assert_eq!(records[1].value, serde_json::json!(2));
        assert_eq!(records[1].source_time, second.source_time);
        assert!(Arc::ptr_eq(&records[1].metadata, &second.metadata));
    }

    #[test]
    fn dropping_last_handle_cancels_subscription_state() {
        let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
            NonZeroUsize::new(256).unwrap(),
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
            let state = Arc::new(CachedSubscriptionState::<NonClonePayload>::new(
                CachedSubscriptionStatusSnapshot::new(
                    CachedSubscriptionStatus::WaitingForFirstSample,
                ),
                RetentionPolicy::LatestOnly,
                NonZeroUsize::new(256).unwrap(),
            ));

            state.handle().subscribe_updates().unwrap()
        };

        assert!(matches!(
            updates.try_recv(),
            Err(CachedSubscriptionUpdateClosed)
        ));
    }
}

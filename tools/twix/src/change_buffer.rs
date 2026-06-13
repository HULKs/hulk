use std::{fmt::Display, sync::Arc, time::Duration, time::SystemTime};

use color_eyre::eyre::{self, eyre};
use color_eyre::{Result, eyre::Report};
use eframe::egui::Context as EguiContext;
use ros_z::{dynamic::DynamicPayload, node::Node, pubsub::PublicationId, time::Time};
use ros_z_debug::{
    DebugEvent, JsonRenderPolicy, ManagerOptions, RetentionPolicy, SampleRecord,
    SubscriptionManager, dynamic_payload_to_json,
};
use serde_json::Value;
use tokio::{
    runtime::Runtime,
    sync::watch,
    time::{self, MissedTickBehavior},
};

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const SUBSCRIBE_RETRY_DELAY: Duration = Duration::from_secs(1);
const CHANGE_RETENTION_WINDOW: Duration = Duration::from_secs(1);

type JsonChangeBuffer = ChangeBuffer<Value, Report>;

struct ActiveSubscription {
    _manager: SubscriptionManager,
    handle: ros_z_debug::SubscriptionHandle<DynamicPayload>,
    retention: RetentionPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RebuildReason {
    Retarget,
    Retry,
}

#[derive(Clone, Debug)]
pub struct Change<T> {
    pub timestamp: SystemTime,
    pub value: T,
}

#[derive(Clone)]
pub struct ChangeSeries<T> {
    changes: Vec<Change<T>>,
    first_update: Option<SystemTime>,
    last_update: Option<SystemTime>,
}

impl<T> ChangeSeries<T> {
    fn new() -> Self {
        Self {
            changes: Vec::new(),
            first_update: None,
            last_update: None,
        }
    }

    pub fn changes(&self) -> impl Iterator<Item = &Change<T>> {
        self.changes.iter()
    }

    pub fn first_update(&self) -> Option<SystemTime> {
        self.first_update
    }

    pub fn last_update(&self) -> Option<SystemTime> {
        self.last_update
    }
}

pub struct ChangeBufferHandle<T, E = eyre::Report> {
    receiver: watch::Receiver<Result<ChangeSeries<T>, E>>,
}

impl<T, E> ChangeBufferHandle<T, E>
where
    T: Clone + PartialEq,
    E: Display,
{
    pub fn get(&self) -> Result<ChangeSeries<T>> {
        let guard = self.receiver.borrow();
        guard.as_ref().map_err(|error| eyre!("{error:#}")).cloned()
    }
}

pub struct ChangeBuffer<T, E> {
    sender: watch::Sender<Result<ChangeSeries<T>, E>>,
}

impl<T: PartialEq, E> ChangeBuffer<T, E> {
    pub fn new() -> (ChangeBuffer<T, E>, ChangeBufferHandle<T, E>) {
        let (sender, receiver) = watch::channel(Ok(ChangeSeries::new()));
        let buffer = ChangeBuffer { sender };
        let handle = ChangeBufferHandle { receiver };
        (buffer, handle)
    }

    pub fn push(&self, datum: Change<T>) {
        if datum.timestamp != SystemTime::UNIX_EPOCH {
            self.sender.send_modify(|value| handle_update(value, datum));
        }
    }

    pub fn clear(&self) {
        let _ = self.sender.send(Ok(ChangeSeries::new()));
    }

    pub fn send_error(&self, error: E) {
        let _ = self.sender.send(Err(error));
    }

    pub fn clear_error(&self) -> bool {
        self.sender.send_if_modified(|value| {
            if value.is_err() {
                *value = Ok(ChangeSeries::new());
                true
            } else {
                false
            }
        })
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    pub async fn closed(&self) {
        self.sender.closed().await;
    }
}

pub fn spawn_json_change_buffer(
    runtime: &Runtime,
    node: Arc<Node>,
    target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: String,
) -> ChangeBufferHandle<Value> {
    let (buffer, handle) = ChangeBuffer::new();
    runtime.spawn(run_json_change_buffer(
        node,
        target_namespace,
        egui_context,
        selector,
        buffer,
    ));
    handle
}

async fn run_json_change_buffer(
    node: Arc<Node>,
    mut target_namespace: watch::Receiver<String>,
    egui_context: EguiContext,
    selector: String,
    buffer: JsonChangeBuffer,
) {
    let mut clear_on_rebuild = true;

    loop {
        if buffer.is_closed() {
            break;
        }

        if clear_on_rebuild {
            buffer.clear();
            egui_context.request_repaint();
        }

        let namespace = target_namespace.borrow_and_update().clone();
        let subscription = subscribe_dynamic(node.clone(), namespace, selector.clone());
        tokio::pin!(subscription);

        let active_subscription = tokio::select! {
            result = &mut subscription => result,
            changed = target_namespace.changed() => {
                if changed.is_err() {
                    break;
                }
                clear_on_rebuild = true;
                continue;
            }
            _ = buffer.closed() => break,
        };

        let rebuild_reason = match active_subscription {
            Ok(active_subscription) => {
                if buffer.clear_error() {
                    egui_context.request_repaint();
                }
                let Some(rebuild_reason) = forward_subscription(
                    active_subscription,
                    &mut target_namespace,
                    &buffer,
                    &egui_context,
                )
                .await
                else {
                    break;
                };
                rebuild_reason
            }
            Err(error) => {
                buffer.send_error(error);
                egui_context.request_repaint();
                let Some(rebuild_reason) =
                    wait_for_retry_or_retarget(&mut target_namespace, &buffer).await
                else {
                    break;
                };
                rebuild_reason
            }
        };

        clear_on_rebuild = should_clear_on_rebuild(rebuild_reason);
    }
}

async fn subscribe_dynamic(
    node: Arc<Node>,
    target_namespace: String,
    selector: String,
) -> Result<ActiveSubscription> {
    let retention = change_retention_policy();
    let manager = SubscriptionManager::new(
        node,
        ManagerOptions::with_target_namespace(target_namespace)?,
    );
    let handle = manager
        .subscribe_dynamic(selector)
        .retention(retention)
        .build()
        .await?;

    Ok(ActiveSubscription {
        _manager: manager,
        handle,
        retention,
    })
}

async fn forward_subscription(
    active_subscription: ActiveSubscription,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonChangeBuffer,
    egui_context: &EguiContext,
) -> Option<RebuildReason> {
    let mut poll = time::interval(EVENT_POLL_INTERVAL);
    poll.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = poll.tick() => drain_events(&active_subscription, buffer, egui_context),
            changed = target_namespace.changed() => return changed.ok().map(|()| RebuildReason::Retarget),
            _ = buffer.closed() => return None,
        }
    }
}

fn drain_events(
    active_subscription: &ActiveSubscription,
    buffer: &JsonChangeBuffer,
    egui_context: &EguiContext,
) {
    let events = active_subscription.handle.drain_events();
    if events.is_empty() {
        return;
    }

    let has_identity_events = events
        .iter()
        .any(|event| matches!(event, DebugEvent::ValueRetained { .. }));
    let mut requested_repaint = false;

    for event in events {
        match event {
            DebugEvent::ValueUpdated => {
                if !has_identity_events && let Some(record) = active_subscription.handle.latest() {
                    forward_record(record, buffer);
                    requested_repaint = true;
                }
            }
            DebugEvent::ValueRetained {
                source_time,
                publication_id,
            } => {
                if let Some(record) = retained_record(
                    &active_subscription.handle,
                    active_subscription.retention,
                    source_time,
                    publication_id,
                ) {
                    forward_record(record, buffer);
                    requested_repaint = true;
                }
            }
            DebugEvent::Diagnostic(message) => {
                buffer.send_error(eyre!(message));
                requested_repaint = true;
            }
            DebugEvent::StatusChanged => {}
            _ => {}
        }
    }

    if requested_repaint {
        egui_context.request_repaint();
    }
}

fn forward_record(record: Arc<SampleRecord<DynamicPayload>>, buffer: &JsonChangeBuffer) {
    buffer.push(Change {
        timestamp: record.source_time.to_wallclock(),
        value: dynamic_payload_to_json(&record.value, JsonRenderPolicy::default()),
    });
}

fn retained_record(
    handle: &ros_z_debug::SubscriptionHandle<DynamicPayload>,
    retention: RetentionPolicy,
    source_time: Time,
    publication_id: PublicationId,
) -> Option<Arc<SampleRecord<DynamicPayload>>> {
    match retention {
        RetentionPolicy::TimeWindow(_) => handle
            .window(source_time, source_time)
            .into_iter()
            .find(|record| record.publication_id == publication_id),
        RetentionPolicy::LatestOnly => handle.latest().filter(|record| {
            record.source_time == source_time && record.publication_id == publication_id
        }),
        _ => handle.latest().filter(|record| {
            record.source_time == source_time && record.publication_id == publication_id
        }),
    }
}

async fn wait_for_retry_or_retarget(
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonChangeBuffer,
) -> Option<RebuildReason> {
    let retry = time::sleep(SUBSCRIBE_RETRY_DELAY);
    tokio::pin!(retry);

    tokio::select! {
        _ = &mut retry => Some(RebuildReason::Retry),
        changed = target_namespace.changed() => changed.ok().map(|()| RebuildReason::Retarget),
        _ = buffer.closed() => None,
    }
}

fn should_clear_on_rebuild(rebuild_reason: RebuildReason) -> bool {
    matches!(rebuild_reason, RebuildReason::Retarget)
}

fn change_retention_policy() -> RetentionPolicy {
    RetentionPolicy::time_window(CHANGE_RETENTION_WINDOW).unwrap_or(RetentionPolicy::LatestOnly)
}

fn handle_update<T: PartialEq, E>(value: &mut Result<ChangeSeries<T>, E>, datum: Change<T>) {
    match value.as_mut() {
        Ok(buffer) => {
            let timestamp = datum.timestamp;
            let index = buffer
                .changes
                .partition_point(|sample| sample.timestamp < datum.timestamp);

            buffer.last_update = match buffer.last_update {
                Some(last_update) => Some(last_update.max(timestamp)),
                None => Some(timestamp),
            };
            buffer.first_update = match buffer.first_update {
                Some(first_update) => Some(first_update.min(timestamp)),
                None => Some(timestamp),
            };

            if buffer
                .changes
                .get(index)
                .is_some_and(|change| change.timestamp == timestamp)
            {
                buffer.changes[index] = datum;
            } else {
                buffer.changes.insert(index, datum);
            }
            compress_adjacent_duplicate_changes(&mut buffer.changes);
        }
        Err(_) => {
            *value = Ok(ChangeSeries {
                first_update: Some(datum.timestamp),
                last_update: Some(datum.timestamp),
                changes: vec![datum],
            });
        }
    }
}

fn compress_adjacent_duplicate_changes<T: PartialEq>(changes: &mut Vec<Change<T>>) {
    let mut index = 1;
    while index < changes.len() {
        if changes[index - 1].value == changes[index].value {
            changes.remove(index);
        } else {
            index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use color_eyre::Report;

    use super::*;

    #[test]
    fn push_ignores_unix_epoch_updates() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();

        buffer.push(Change {
            timestamp: SystemTime::UNIX_EPOCH,
            value: 1,
        });

        let series = handle.get().unwrap();
        assert_eq!(series.changes().count(), 0);
        assert_eq!(series.first_update(), None);
        assert_eq!(series.last_update(), None);
    }

    #[test]
    fn push_records_only_changed_values() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();
        let first = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let second = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
        let third = SystemTime::UNIX_EPOCH + Duration::from_secs(3);

        buffer.push(Change {
            timestamp: first,
            value: 1,
        });
        buffer.push(Change {
            timestamp: second,
            value: 1,
        });
        buffer.push(Change {
            timestamp: third,
            value: 2,
        });

        let series = handle.get().unwrap();
        let values = series
            .changes()
            .map(|change| change.value)
            .collect::<Vec<_>>();
        assert_eq!(values, [1, 2]);
        assert_eq!(series.first_update(), Some(first));
        assert_eq!(series.last_update(), Some(third));
    }

    #[test]
    fn send_error_is_visible_to_handle() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();

        buffer.send_error(eyre!("subscription failed"));

        match handle.get() {
            Ok(_) => panic!("expected subscription error"),
            Err(error) => assert!(format!("{error:#}").contains("subscription failed")),
        }
    }

    #[test]
    fn clear_resets_series_after_values_and_errors() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();

        buffer.push(Change {
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            value: 1,
        });
        buffer.send_error(eyre!("subscription failed"));
        buffer.clear();

        let series = handle.get().unwrap();
        assert_eq!(series.changes().count(), 0);
        assert_eq!(series.first_update(), None);
        assert_eq!(series.last_update(), None);
    }

    #[test]
    fn clear_error_replaces_error_with_empty_series() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();
        buffer.send_error(eyre!("subscription failed"));

        assert!(buffer.clear_error());

        let series = handle.get().unwrap();
        assert_eq!(series.changes().count(), 0);
        assert_eq!(series.first_update(), None);
        assert_eq!(series.last_update(), None);
    }

    #[test]
    fn clear_error_preserves_existing_series() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();
        let first = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        buffer.push(Change {
            timestamp: first,
            value: 1,
        });

        assert!(!buffer.clear_error());

        let series = handle.get().unwrap();
        let values = series
            .changes()
            .map(|change| change.value)
            .collect::<Vec<_>>();
        assert_eq!(values, [1]);
        assert_eq!(series.first_update(), Some(first));
        assert_eq!(series.last_update(), Some(first));
    }

    #[test]
    fn out_of_order_insertion_preserves_later_changes() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();
        let first = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let second = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
        let third = SystemTime::UNIX_EPOCH + Duration::from_secs(3);

        buffer.push(Change {
            timestamp: first,
            value: 1,
        });
        buffer.push(Change {
            timestamp: third,
            value: 3,
        });
        buffer.push(Change {
            timestamp: second,
            value: 2,
        });

        let series = handle.get().unwrap();
        let changes = series
            .changes()
            .map(|change| (change.timestamp, change.value))
            .collect::<Vec<_>>();
        assert_eq!(changes, [(first, 1), (second, 2), (third, 3)]);
        assert_eq!(series.first_update(), Some(first));
        assert_eq!(series.last_update(), Some(third));
    }

    #[test]
    fn same_value_out_of_order_samples_compress_changes_and_update_bounds() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();
        let first = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let second = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
        let third = SystemTime::UNIX_EPOCH + Duration::from_secs(3);
        let fourth = SystemTime::UNIX_EPOCH + Duration::from_secs(4);
        let fifth = SystemTime::UNIX_EPOCH + Duration::from_secs(5);

        buffer.push(Change {
            timestamp: second,
            value: 1,
        });
        buffer.push(Change {
            timestamp: fourth,
            value: 2,
        });
        buffer.push(Change {
            timestamp: third,
            value: 1,
        });
        buffer.push(Change {
            timestamp: first,
            value: 1,
        });
        buffer.push(Change {
            timestamp: fifth,
            value: 2,
        });

        let series = handle.get().unwrap();
        let changes = series
            .changes()
            .map(|change| (change.timestamp, change.value))
            .collect::<Vec<_>>();
        assert_eq!(changes, [(first, 1), (fourth, 2)]);
        assert_eq!(series.first_update(), Some(first));
        assert_eq!(series.last_update(), Some(fifth));
    }

    #[test]
    fn replacing_existing_timestamp_does_not_duplicate_entries() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();
        let first = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let second = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
        let third = SystemTime::UNIX_EPOCH + Duration::from_secs(3);

        buffer.push(Change {
            timestamp: first,
            value: 1,
        });
        buffer.push(Change {
            timestamp: second,
            value: 2,
        });
        buffer.push(Change {
            timestamp: third,
            value: 3,
        });
        buffer.push(Change {
            timestamp: second,
            value: 4,
        });

        let series = handle.get().unwrap();
        let changes = series
            .changes()
            .map(|change| (change.timestamp, change.value))
            .collect::<Vec<_>>();
        assert_eq!(changes, [(first, 1), (second, 4), (third, 3)]);
        assert_eq!(series.first_update(), Some(first));
        assert_eq!(series.last_update(), Some(third));
    }
}

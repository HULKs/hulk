use std::{fmt::Display, sync::Arc, time::Duration, time::SystemTime};

use color_eyre::eyre::{self, eyre};
use color_eyre::{Result, eyre::Report};
use eframe::egui::Context as EguiContext;
use ros_z::{dynamic::DynamicPayload, node::Node};
use ros_z_debug::{JsonRenderPolicy, RetentionPolicy, SampleRecord, dynamic_payload_to_json};
use serde_json::Value;
use tokio::{runtime::Runtime, sync::watch, time};

use crate::backend::subscription::{self, ActiveSubscription, RebuildReason};

const CHANGE_RETENTION_WINDOW: Duration = Duration::from_secs(1);

type JsonChangeBuffer = ChangeBuffer<Value, Report>;

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

    pub fn error_message(&self) -> Option<String> {
        let guard = self.receiver.borrow();
        guard.as_ref().err().map(|error| format!("{error:#}"))
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
        self.sender.send_modify(|value| handle_update(value, datum));
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
        let subscription = subscription::subscribe_dynamic(
            node.clone(),
            namespace,
            selector.clone(),
            change_retention_policy(),
        );
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
                let Some(rebuild_reason) = subscription::wait_for_retry_or_retarget(
                    &mut target_namespace,
                    buffer.closed(),
                )
                .await
                else {
                    break;
                };
                rebuild_reason
            }
        };

        clear_on_rebuild = should_clear_on_rebuild(rebuild_reason);
    }
}

async fn forward_subscription(
    active_subscription: ActiveSubscription,
    target_namespace: &mut watch::Receiver<String>,
    buffer: &JsonChangeBuffer,
    egui_context: &EguiContext,
) -> Option<RebuildReason> {
    let mut poll = time::interval(subscription::EVENT_POLL_INTERVAL);
    subscription::skip_missed_ticks(&mut poll);

    loop {
        tokio::select! {
            _ = poll.tick() => {
                subscription::drain_events(
                    &active_subscription,
                    egui_context,
                    |error| buffer.send_error(error),
                    |record| async move { forward_record(record, buffer) },
                ).await;
            }
            changed = target_namespace.changed() => return changed.ok().map(|()| RebuildReason::Retarget),
            _ = buffer.closed() => return None,
        }
    }
}

fn forward_record(record: Arc<SampleRecord<DynamicPayload>>, buffer: &JsonChangeBuffer) {
    buffer.push(Change {
        timestamp: record.source_time.to_wallclock(),
        value: dynamic_payload_to_json(&record.value, JsonRenderPolicy::default()),
    });
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

            let changed_index = if buffer
                .changes
                .get(index)
                .is_some_and(|change| change.timestamp == timestamp)
            {
                buffer.changes[index] = datum;
                index
            } else {
                buffer.changes.insert(index, datum);
                index
            };
            compress_adjacent_duplicate_changes_around(&mut buffer.changes, changed_index);
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

fn compress_adjacent_duplicate_changes_around<T: PartialEq>(
    changes: &mut Vec<Change<T>>,
    mut index: usize,
) {
    if changes.is_empty() {
        return;
    }

    while index > 0 && changes[index - 1].value == changes[index].value {
        changes.remove(index);
        index -= 1;
    }

    while index + 1 < changes.len() && changes[index].value == changes[index + 1].value {
        changes.remove(index + 1);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use color_eyre::Report;

    use super::*;

    #[test]
    fn push_records_unix_epoch_updates() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();

        buffer.push(Change {
            timestamp: SystemTime::UNIX_EPOCH,
            value: 1,
        });

        let series = handle.get().unwrap();
        let changes = series
            .changes()
            .map(|change| (change.timestamp, change.value))
            .collect::<Vec<_>>();
        assert_eq!(changes, [(SystemTime::UNIX_EPOCH, 1)]);
        assert_eq!(series.first_update(), Some(SystemTime::UNIX_EPOCH));
        assert_eq!(series.last_update(), Some(SystemTime::UNIX_EPOCH));
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
    fn error_message_reports_current_error_without_replacing_successful_series() {
        let (buffer, handle) = ChangeBuffer::<i32, Report>::new();
        assert_eq!(handle.error_message(), None);

        buffer.push(Change {
            timestamp: SystemTime::UNIX_EPOCH,
            value: 1,
        });
        assert_eq!(handle.error_message(), None);

        buffer.send_error(eyre!("subscription failed"));
        assert_eq!(
            handle.error_message().as_deref(),
            Some("subscription failed")
        );
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

use std::{
    fmt::Display,
    time::{Duration, SystemTime},
};

use color_eyre::Result;
use color_eyre::eyre::{self, eyre};
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct Datum<T> {
    pub timestamp: SystemTime,
    pub value: T,
}

type TimeSeries<T> = Vec<Datum<T>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferHistory {
    LatestOnly,
    TimeWindow(Duration),
}

impl BufferHistory {
    pub fn from_duration(duration: Duration) -> Self {
        if duration.is_zero() {
            Self::LatestOnly
        } else {
            Self::TimeWindow(duration)
        }
    }
}

pub struct BufferHandle<T, E = eyre::Report> {
    receiver: watch::Receiver<Result<TimeSeries<T>, E>>,
    history: watch::Sender<BufferHistory>,
}

impl<T, E> Clone for BufferHandle<T, E> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            history: self.history.clone(),
        }
    }
}

impl<T, E> BufferHandle<T, E>
where
    T: Clone,
    E: Display,
{
    pub fn get(&self) -> Result<TimeSeries<T>> {
        let guard = self.receiver.borrow();
        guard.as_ref().map_err(|error| eyre!("{error:#}")).cloned()
    }

    pub fn get_last(&self) -> Result<Option<Datum<T>>> {
        let guard = self.receiver.borrow();
        match guard.as_ref() {
            Ok(series) => Ok(series.last().cloned()),
            Err(error) => Err(eyre!("{error:#}")),
        }
    }

    pub fn get_last_timestamp(&self) -> Result<Option<SystemTime>> {
        let guard = self.receiver.borrow();
        match guard.as_ref() {
            Ok(series) => Ok(series.last().map(|datum| datum.timestamp)),
            Err(error) => Err(eyre!("{error:#}")),
        }
    }

    pub fn get_last_value(&self) -> Result<Option<T>> {
        Ok(self.get_last()?.map(|datum| datum.value))
    }

    pub fn set_history(&self, history: BufferHistory) {
        self.history.send_replace(history);
    }
}

pub struct Buffer<T, E> {
    sender: watch::Sender<Result<TimeSeries<T>, E>>,
    history: watch::Receiver<BufferHistory>,
}

impl<T, E> Buffer<T, E> {
    pub fn new(history: BufferHistory) -> (Buffer<T, E>, BufferHandle<T, E>) {
        let (sender, receiver) = watch::channel(Ok(TimeSeries::new()));
        let (history_sender, history_receiver) = watch::channel(history);
        let buffer = Buffer {
            sender,
            history: history_receiver,
        };
        let handle = BufferHandle {
            receiver,
            history: history_sender,
        };
        (buffer, handle)
    }

    pub async fn history(&self) -> BufferHistory {
        *self.history.borrow()
    }

    pub fn subscribe_history(&self) -> watch::Receiver<BufferHistory> {
        self.history.clone()
    }

    pub fn send_error(&self, error: E) {
        let _ = self.sender.send(Err(error));
    }

    pub fn clear_error(&self) -> bool {
        self.sender.send_if_modified(|value| {
            if value.is_err() {
                *value = Ok(TimeSeries::new());
                true
            } else {
                false
            }
        })
    }

    pub async fn push(&self, datum: Datum<T>) {
        let history = *self.history.borrow();
        self.sender
            .send_modify(|value| handle_update(value, datum, history));
    }

    pub fn replace(&self, series: TimeSeries<T>) {
        let _ = self.sender.send(Ok(series));
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    pub async fn closed(&self) {
        self.sender.closed().await;
    }
}

fn handle_update<T, E>(
    value: &mut Result<Vec<Datum<T>>, E>,
    datum: Datum<T>,
    history: BufferHistory,
) {
    match value.as_mut() {
        Ok(buffer) => {
            let BufferHistory::TimeWindow(history) = history else {
                if buffer
                    .last()
                    .is_none_or(|sample| datum.timestamp >= sample.timestamp)
                {
                    buffer.clear();
                    buffer.push(datum);
                }
                return;
            };

            let insert_at = buffer.partition_point(|sample| sample.timestamp < datum.timestamp);
            let replace_until =
                buffer.partition_point(|sample| sample.timestamp <= datum.timestamp);
            buffer.drain(insert_at..replace_until);
            buffer.insert(insert_at, datum);

            if let Some(cutoff) = buffer
                .last()
                .and_then(|sample| sample.timestamp.checked_sub(history))
            {
                let remove_until = buffer.partition_point(|sample| sample.timestamp < cutoff);
                buffer.drain(..remove_until);
            }
        }
        Err(_) => {
            *value = Ok(TimeSeries::from([datum]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn buffer_retains_values_inside_history_window() {
        let (buffer, handle) =
            Buffer::<i32, eyre::Report>::new(BufferHistory::TimeWindow(Duration::from_secs(1)));
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10);

        buffer
            .push(Datum {
                timestamp: now - Duration::from_secs(2),
                value: 1,
            })
            .await;
        buffer
            .push(Datum {
                timestamp: now,
                value: 2,
            })
            .await;

        assert_eq!(
            handle
                .get()
                .unwrap()
                .iter()
                .map(|datum| datum.value)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[tokio::test]
    async fn set_history_notifies_buffer_history_subscribers() {
        let (buffer, handle) = Buffer::<i32, eyre::Report>::new(BufferHistory::LatestOnly);
        let mut history = buffer.subscribe_history();

        handle.set_history(BufferHistory::TimeWindow(Duration::from_secs(2)));

        tokio::time::timeout(Duration::from_millis(100), history.changed())
            .await
            .expect("history changes should wake promptly")
            .expect("history channel should stay open");
        assert_eq!(
            *history.borrow(),
            BufferHistory::TimeWindow(Duration::from_secs(2))
        );
    }

    #[tokio::test]
    async fn buffer_recovers_from_error_with_next_json_value() {
        let (buffer, handle) =
            Buffer::<serde_json::Value, eyre::Report>::new(BufferHistory::LatestOnly);
        buffer.send_error(color_eyre::eyre::eyre!("decode error"));

        buffer
            .push(Datum {
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                value: serde_json::json!({ "state": "ready" }),
            })
            .await;

        assert_eq!(
            handle.get_last_value().unwrap(),
            Some(serde_json::json!({ "state": "ready" }))
        );
    }

    #[tokio::test]
    async fn clear_error_replaces_error_with_empty_series() {
        let (buffer, handle) =
            Buffer::<i32, eyre::Report>::new(BufferHistory::TimeWindow(Duration::from_secs(1)));
        buffer.send_error(color_eyre::eyre::eyre!("subscription failed"));

        assert!(buffer.clear_error());

        assert!(handle.get().unwrap().is_empty());
    }

    #[tokio::test]
    async fn clear_error_preserves_existing_series() {
        let (buffer, handle) =
            Buffer::<i32, eyre::Report>::new(BufferHistory::TimeWindow(Duration::from_secs(1)));
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        buffer
            .push(Datum {
                timestamp: now,
                value: 1,
            })
            .await;

        assert!(!buffer.clear_error());

        assert_eq!(
            handle
                .get()
                .unwrap()
                .iter()
                .map(|datum| datum.value)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[tokio::test]
    async fn out_of_order_sample_preserves_newer_history_entries() {
        let (buffer, handle) =
            Buffer::<i32, eyre::Report>::new(BufferHistory::TimeWindow(Duration::from_secs(10)));
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);

        buffer
            .push(Datum {
                timestamp: now,
                value: 1,
            })
            .await;
        buffer
            .push(Datum {
                timestamp: now + Duration::from_secs(10),
                value: 3,
            })
            .await;
        buffer
            .push(Datum {
                timestamp: now + Duration::from_secs(5),
                value: 2,
            })
            .await;

        assert_eq!(
            handle
                .get()
                .unwrap()
                .iter()
                .map(|datum| datum.value)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[tokio::test]
    async fn latest_only_history_keeps_only_newest_equal_timestamp_sample() {
        let (buffer, handle) = Buffer::<i32, eyre::Report>::new(BufferHistory::LatestOnly);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);

        buffer
            .push(Datum {
                timestamp: now,
                value: 1,
            })
            .await;
        buffer
            .push(Datum {
                timestamp: now,
                value: 2,
            })
            .await;

        assert_eq!(
            handle
                .get()
                .unwrap()
                .iter()
                .map(|datum| datum.value)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[tokio::test]
    async fn nonzero_history_replaces_equal_timestamp_sample() {
        let (buffer, handle) =
            Buffer::<i32, eyre::Report>::new(BufferHistory::TimeWindow(Duration::from_secs(10)));
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);

        buffer
            .push(Datum {
                timestamp: now,
                value: 1,
            })
            .await;
        buffer
            .push(Datum {
                timestamp: now,
                value: 2,
            })
            .await;

        assert_eq!(
            handle
                .get()
                .unwrap()
                .iter()
                .map(|datum| datum.value)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }
}

use std::sync::{
    Arc, Mutex as StdMutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use color_eyre::{Result, eyre::eyre};
use tokio::sync::Notify;

struct StampedMessage<T> {
    sequence: u64,
    value: T,
}

struct LatestSlot<T> {
    name: &'static str,
    message: StdMutex<Option<StampedMessage<T>>>,
    notify: Notify,
    latest_sequence: AtomicU64,
    closed: AtomicBool,
}

pub struct LatestSender<T> {
    slot: Arc<LatestSlot<T>>,
}

pub struct LatestReceiver<T> {
    slot: Arc<LatestSlot<T>>,
    last_seen_sequence: u64,
}

#[derive(Debug)]
pub struct ReceivedMessage<T> {
    pub value: T,
    pub dropped_messages: u64,
}

pub fn latest_channel<T>(name: &'static str) -> (LatestSender<T>, LatestReceiver<T>) {
    let slot = Arc::new(LatestSlot {
        name,
        message: StdMutex::new(None),
        notify: Notify::new(),
        latest_sequence: AtomicU64::new(0),
        closed: AtomicBool::new(false),
    });

    (
        LatestSender { slot: slot.clone() },
        LatestReceiver {
            slot,
            last_seen_sequence: 0,
        },
    )
}

impl<T> LatestSender<T> {
    pub fn send_latest(&self, value: T) {
        let sequence = self.slot.latest_sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let mut message = self
            .slot
            .message
            .lock()
            .expect("latest slot mutex poisoned");
        *message = Some(StampedMessage { sequence, value });
        drop(message);
        self.slot.notify.notify_one();
    }
}

impl<T> Drop for LatestSender<T> {
    fn drop(&mut self) {
        self.slot.closed.store(true, Ordering::Release);
        self.slot.notify.notify_waiters();
    }
}

impl<T> LatestReceiver<T> {
    fn latest_sequence(&self) -> u64 {
        self.slot.latest_sequence.load(Ordering::Acquire)
    }

    pub async fn recv_latest(&mut self) -> Result<ReceivedMessage<T>> {
        loop {
            let notified = self.slot.notify.notified();

            if self.latest_sequence() > self.last_seen_sequence {
                let maybe_message = {
                    let mut message = self
                        .slot
                        .message
                        .lock()
                        .expect("latest slot mutex poisoned");
                    message.take()
                };

                if let Some(stamped_message) = maybe_message {
                    if stamped_message.sequence <= self.last_seen_sequence {
                        continue;
                    }

                    let dropped_messages = stamped_message
                        .sequence
                        .saturating_sub(self.last_seen_sequence.saturating_add(1));
                    self.last_seen_sequence = stamped_message.sequence;

                    return Ok(ReceivedMessage {
                        value: stamped_message.value,
                        dropped_messages,
                    });
                }
            }

            if self.slot.closed.load(Ordering::Acquire) {
                return Err(eyre!("ROS receiver for `{}` closed", self.slot.name));
            }

            notified.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::latest_channel;

    #[derive(Debug, PartialEq, Eq)]
    struct NonCloneMessage {
        id: u32,
        payload: Vec<u8>,
    }

    #[tokio::test]
    async fn recv_latest_returns_sent_value_without_cloning() {
        let (sender, mut receiver) = latest_channel("test_topic");

        sender.send_latest(NonCloneMessage {
            id: 7,
            payload: vec![1_u8, 2, 3],
        });

        let message = receiver.recv_latest().await.unwrap();
        assert_eq!(
            message.value,
            NonCloneMessage {
                id: 7,
                payload: vec![1_u8, 2, 3],
            }
        );
        assert_eq!(message.dropped_messages, 0);
    }

    #[tokio::test]
    async fn recv_latest_returns_newest_message_after_overwrites() {
        let (sender, mut receiver) = latest_channel("test_topic");

        sender.send_latest(1_u32);
        sender.send_latest(2_u32);
        sender.send_latest(3_u32);

        let message = receiver.recv_latest().await.unwrap();
        assert_eq!(message.value, 3);
        assert_eq!(message.dropped_messages, 2);
    }

    #[tokio::test]
    async fn recv_latest_waits_for_newer_message_after_consumption() {
        let (sender, mut receiver) = latest_channel("test_topic");

        sender.send_latest(1_u32);
        let message = receiver.recv_latest().await.unwrap();
        assert_eq!(message.value, 1);
        assert_eq!(message.dropped_messages, 0);

        let task = tokio::spawn(async move { receiver.recv_latest().await.unwrap() });
        tokio::task::yield_now().await;
        assert!(!task.is_finished());

        sender.send_latest(2_u32);
        let message = task.await.unwrap();
        assert_eq!(message.value, 2);
        assert_eq!(message.dropped_messages, 0);
    }

    #[tokio::test]
    async fn recv_latest_returns_error_when_sender_is_dropped() {
        let (sender, mut receiver) = latest_channel::<u32>("test_topic");
        drop(sender);

        let error = receiver.recv_latest().await.unwrap_err();
        assert!(error.to_string().contains("test_topic"));
        assert!(error.to_string().contains("closed"));
    }
}

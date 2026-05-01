use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Duration,
};

use ros_z::{
    EndpointGlobalId, Message, msg::WireDecoder, node::Node, pubsub::Subscriber, time::Time,
};
use tokio::select;
use zenoh::Result as ZResult;

use crate::announce::Announcement;

type PublicationKey = (EndpointGlobalId, i64);

const MAX_UNMATCHED_PUBLICATIONS: usize = 128;

struct PendingData<T> {
    source_time: Option<Time>,
    value: T,
}

fn trim_unmatched_publications<T>(
    pending_data: &mut BTreeMap<PublicationKey, PendingData<T>>,
    tombstones: &mut BTreeSet<PublicationKey>,
) {
    while pending_data.len() > MAX_UNMATCHED_PUBLICATIONS {
        if let Some(key) = pending_data.first_key_value().map(|(key, _)| *key) {
            pending_data.remove(&key);
        }
    }

    while tombstones.len() > MAX_UNMATCHED_PUBLICATIONS {
        if let Some(key) = tombstones.first().cloned() {
            tombstones.remove(&key);
        }
    }
}

/// Runtime lag policy for streams with empty in-flight set.
#[derive(Debug, Clone, Copy, Default)]
pub enum LagPolicy {
    /// Empty queue provides no safe-time constraint.
    #[default]
    Immediate,
    /// Empty queue constrains safe-time using `reference_time - max_lag`.
    Watermark {
        /// Maximum lag applied when measured lag is larger.
        max_lag: Duration,
    },
}

/// Diagnostic warning emitted while deriving queue state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LagWarning {
    /// Metadata did not contain source timestamp.
    SourceTimeMissing,
    /// Measured lag exceeded configured watermark cap and was clamped.
    LagExceeded {
        /// Measured lag from source timestamp and announcement timestamp.
        measured: Duration,
        /// Effective lag after clamping.
        clamped_to: Duration,
    },
}

/// Per-stream state snapshot produced by queue events.
#[derive(Debug, Clone, Copy)]
pub struct QueueState {
    /// Current safe-time constraint for this stream.
    pub safe_time: Option<Time>,
    /// Latest observed source timestamp for this stream.
    pub reference_time: Time,
    /// Effective lag currently used by watermark mode.
    pub effective_lag: Option<Duration>,
    /// Optional warning produced while updating this state.
    pub warning: Option<LagWarning>,
}

/// Event produced by [`FutureQueueSubscriber::recv`].
pub enum QueueEvent<T> {
    /// Announcement received and queue state updated.
    Announcement { state: QueueState },
    /// Data payload received with current queue state and payload timestamp.
    Data {
        state: QueueState,
        data_time: Time,
        value: T,
    },
}

/// Subscriber that tracks in-flight messages for one stream.
pub struct FutureQueueSubscriber<T>
where
    T: Message,
    for<'a> T::Codec: WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    data_subscriber: Subscriber<T>,
    announcement_subscriber: Subscriber<Announcement>,
    lag_policy: LagPolicy,
    reference_time: Time,
    effective_lag: Option<Duration>,
    inflight: BTreeMap<PublicationKey, Time>,
    pending_data: BTreeMap<PublicationKey, PendingData<T>>,
    ready_data: VecDeque<(Time, T)>,
    tombstones: BTreeSet<PublicationKey>,
}

impl<T> FutureQueueSubscriber<T>
where
    T: Message,
    for<'a> T::Codec: WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    fn safe_time(&self) -> Option<Time> {
        if let Some((_, oldest_inflight)) = self.inflight.first_key_value() {
            return Some(*oldest_inflight);
        }

        match (self.lag_policy, self.effective_lag) {
            (LagPolicy::Immediate, _) => None,
            (LagPolicy::Watermark { .. }, Some(effective_lag)) => {
                Some(self.reference_time - effective_lag)
            }
            (LagPolicy::Watermark { max_lag }, None) => Some(self.reference_time - max_lag),
        }
    }

    fn queue_state(&self, warning: Option<LagWarning>) -> QueueState {
        QueueState {
            safe_time: self.safe_time(),
            reference_time: self.reference_time,
            effective_lag: self.effective_lag,
            warning,
        }
    }

    fn update_reference_time(
        &mut self,
        source_time: Option<Time>,
        announcement_time: Option<Time>,
    ) -> Option<LagWarning> {
        match self.lag_policy {
            LagPolicy::Immediate => {
                if let Some(source_time) = source_time {
                    self.reference_time = source_time;
                }
                None
            }
            LagPolicy::Watermark { max_lag } => {
                let source_time = match source_time {
                    Some(source_time) => source_time,
                    None => return Some(LagWarning::SourceTimeMissing),
                };

                self.reference_time = source_time;
                if let Some(announcement_time) = announcement_time {
                    let measured = source_time.duration_since(announcement_time);
                    let clamped = measured.min(max_lag);
                    self.effective_lag = Some(clamped);
                    if measured > max_lag {
                        return Some(LagWarning::LagExceeded {
                            measured,
                            clamped_to: max_lag,
                        });
                    }
                }
                None
            }
        }
    }

    fn register_announcement(
        &mut self,
        announcement: Announcement,
        source_time: Option<Time>,
    ) -> Option<LagWarning> {
        let publication_key = (announcement.source_global_id, announcement.sequence_number);
        if announcement.canceled {
            self.inflight.remove(&publication_key);
            self.pending_data.remove(&publication_key);
            self.tombstones.insert(publication_key);
            self.trim_unmatched_publications();
            return None;
        }

        if self.tombstones.remove(&publication_key) {
            return None;
        }

        let warning = self.update_reference_time(source_time, Some(announcement.time));
        if let Some(pending_data) = self.pending_data.remove(&publication_key) {
            self.update_reference_time(pending_data.source_time, None);
            self.ready_data
                .push_back((announcement.time, pending_data.value));
        } else {
            self.inflight.insert(publication_key, announcement.time);
        }
        warning
    }

    fn trim_unmatched_publications(&mut self) {
        trim_unmatched_publications(&mut self.pending_data, &mut self.tombstones);
    }

    async fn ingest_pending_announcements(&mut self) -> ZResult<Option<LagWarning>> {
        let mut warning = None;
        while self.announcement_subscriber.is_ready() {
            let received = self.announcement_subscriber.recv_with_metadata().await?;
            warning =
                warning.or(self.register_announcement(received.message, received.source_time));
        }
        Ok(warning)
    }

    /// Return current stream state without consuming events.
    pub fn current_state(&self) -> QueueState {
        self.queue_state(None)
    }

    /// Drain ready announcements and return updated state.
    pub async fn drain_announcements(&mut self) -> ZResult<QueueState> {
        let warning = self.ingest_pending_announcements().await?;
        Ok(self.queue_state(warning))
    }

    /// Wait for next announcement or payload event.
    pub async fn recv(&mut self) -> ZResult<QueueEvent<T>> {
        loop {
            if let Some((data_time, value)) = self.ready_data.pop_front() {
                return Ok(QueueEvent::Data {
                    state: self.queue_state(None),
                    data_time,
                    value,
                });
            }

            select! {
                result = self.announcement_subscriber.recv_with_metadata() => {
                    let received = result?;
                    let warning = self.register_announcement(received.message, received.source_time);
                    if let Some((data_time, value)) = self.ready_data.pop_front() {
                        return Ok(QueueEvent::Data {
                            state: self.queue_state(warning),
                            data_time,
                            value,
                        });
                    }
                    return Ok(QueueEvent::Announcement { state: self.queue_state(warning) });
                }
                result = self.data_subscriber.recv_with_metadata() => {
                    let received = result?;

                    let mut warning = self.ingest_pending_announcements().await?;
                    warning = warning.or(self.update_reference_time(received.source_time, None));

                    let publication_id = received
                        .publication_id()
                        .ok_or_else(|| zenoh::Error::from("received data without attachment publication id"))?;
                    let publication_key = (publication_id.endpoint_global_id(), publication_id.sequence_number());

                    if let Some(data_time) = self.inflight.remove(&publication_key) {
                        return Ok(QueueEvent::Data {
                            state: self.queue_state(warning),
                            data_time,
                            value: received.message,
                        });
                    }

                    if !self.tombstones.contains(&publication_key) {
                        self.pending_data.insert(publication_key, PendingData {
                            source_time: received.source_time,
                            value: received.message,
                        });
                        self.trim_unmatched_publications();
                    }
                },
            }
        }
    }
}

/// Extension trait for creating future queue subscribers.
pub trait CreateFutureQueue {
    /// Subscribe to one stream with configured lag policy.
    fn create_future_subscriber<'a, T>(
        &'a self,
        topic: &'a str,
        lag_policy: LagPolicy,
    ) -> impl std::future::Future<Output = ZResult<FutureQueueSubscriber<T>>> + 'a
    where
        T: Message + 'a,
        for<'de> T::Codec: WireDecoder<Input<'de> = &'de [u8], Output = T>;
}

impl CreateFutureQueue for Node {
    async fn create_future_subscriber<'a, T>(
        &'a self,
        topic: &'a str,
        lag_policy: LagPolicy,
    ) -> ZResult<FutureQueueSubscriber<T>>
    where
        T: Message + 'a,
        for<'de> T::Codec: WireDecoder<Input<'de> = &'de [u8], Output = T>,
    {
        let data_subscriber = self.subscriber(topic).build().await?;
        let announcement_subscriber = self
            .subscriber(&format!("{}/announce", topic))
            .build()
            .await?;

        Ok(FutureQueueSubscriber {
            data_subscriber,
            announcement_subscriber,
            lag_policy,
            reference_time: self.clock().now(),
            effective_lag: match lag_policy {
                LagPolicy::Immediate => None,
                LagPolicy::Watermark { max_lag } => Some(max_lag),
            },
            inflight: BTreeMap::new(),
            pending_data: BTreeMap::new(),
            ready_data: VecDeque::new(),
            tombstones: BTreeSet::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ros_z::context::ContextBuilder;
    use ros_z_msgs::std_msgs::String as TestMessage;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn payload_received_before_announcement_is_delivered_after_announcement() {
        let context = ContextBuilder::default()
            .build()
            .await
            .expect("create context");
        let node = context
            .create_node("queue_payload_before_announcement")
            .build()
            .await
            .expect("create node");
        let data_publisher = node
            .publisher::<TestMessage>("queue/payload_first")
            .build()
            .await
            .expect("create data publisher");
        let announcement_publisher = node
            .publisher::<Announcement>("queue/payload_first/announce")
            .build()
            .await
            .expect("create announcement publisher");
        let mut subscriber = node
            .create_future_subscriber::<TestMessage>("queue/payload_first", LagPolicy::Immediate)
            .await
            .expect("create queue subscriber");

        tokio::time::sleep(Duration::from_millis(100)).await;

        let prepared = data_publisher.prepare();
        let publication_id = prepared.id();
        prepared
            .publish(&TestMessage {
                data: "payload".to_owned(),
            })
            .await
            .expect("publish payload first");

        let result = tokio::time::timeout(Duration::from_millis(100), subscriber.recv()).await;
        assert!(
            result.is_err(),
            "payload without announcement should be buffered"
        );

        announcement_publisher
            .publish(&Announcement {
                time: Time::from_nanos(42),
                source_global_id: publication_id.endpoint_global_id(),
                sequence_number: publication_id.sequence_number(),
                canceled: false,
            })
            .await
            .expect("publish matching announcement");

        let event = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
            .await
            .expect("timeout waiting for queued payload")
            .expect("receive queued payload");
        match event {
            QueueEvent::Data {
                state,
                data_time,
                value,
            } => {
                assert_eq!(state.safe_time, None);
                assert_eq!(data_time, Time::from_nanos(42));
                assert_eq!(value.data, "payload");
            }
            QueueEvent::Announcement { .. } => panic!("expected buffered data event"),
        }
    }

    #[test]
    fn unmatched_payload_and_tombstone_storage_is_bounded() {
        let mut pending_data = BTreeMap::new();
        let mut tombstones = BTreeSet::new();

        for index in 0..256 {
            pending_data.insert(
                ([index as u8; 16], index),
                PendingData {
                    source_time: None,
                    value: TestMessage {
                        data: format!("payload {index}"),
                    },
                },
            );
        }
        trim_unmatched_publications(&mut pending_data, &mut tombstones);
        assert!(pending_data.len() <= MAX_UNMATCHED_PUBLICATIONS);

        for index in 0..256 {
            tombstones.insert(([index as u8; 16], index));
        }
        trim_unmatched_publications(&mut pending_data, &mut tombstones);
        assert!(tombstones.len() <= MAX_UNMATCHED_PUBLICATIONS);
    }
}

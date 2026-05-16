use std::collections::{BTreeSet, VecDeque};

use ros_z::{
    Message,
    node::Node,
    pubsub::{Received, Subscriber},
    time::Time,
};
use tokio::select;
use zenoh::Result as ZResult;

use crate::announce::Announcement;

/// Subscriber that tracks in-flight messages for one stream.
pub struct FutureQueueSubscriber<T: Message> {
    data_subscriber: Subscriber<T>,
    announcement_subscriber: Subscriber<Announcement>,
    inflight: BTreeSet<Announcement>,
    pending_data: VecDeque<Received<T>>,
}

trait BelongToExt {
    fn belongs_to(&self, announcement: &Announcement) -> bool;
}

impl<T> BelongToExt for Received<T> {
    fn belongs_to(&self, announcement: &Announcement) -> bool {
        self.source_global_id == announcement.source_global_id
            && self.sequence_number == announcement.sequence_number
    }
}

impl<T: Message> FutureQueueSubscriber<T> {
    /// Returns the earliest announced safe time for this stream.
    pub(crate) fn safe_time(&self) -> Option<Time> {
        self.inflight.first().map(|announcement| announcement.time)
    }

    /// Check if any of the pending data messages matches the first outstanding announcement
    fn next_publishable(&mut self) -> Option<(Time, T)> {
        let announcement = self.inflight.first()?;
        let index = self
            .pending_data
            .iter()
            .position(|pending| pending.belongs_to(announcement))?;
        // Only remove the announcement if the data matches it
        let announcement = self.inflight.pop_first().expect("announcement must exist");

        self.pending_data
            .remove(index)
            .map(|pending| (announcement.time, pending.message))
    }

    /// Wait for the next data event.
    /// The result is a tuple of the data time and the data value.
    pub async fn recv(&mut self) -> ZResult<(Time, T)> {
        loop {
            if let Some(data) = self.next_publishable() {
                return Ok(data);
            }

            select! {
                announcement = self.announcement_subscriber.recv() => {
                    self.inflight.insert(announcement?);
                }
                data = self.data_subscriber.recv_with_metadata() => {
                    self.pending_data.push_back(data?);
                }
            }
        }
    }
}

/// Extension trait for creating future queue subscribers.
pub trait CreateFutureQueue {
    /// Subscribe to one stream with configured lag policy.
    fn create_future_subscriber<T: Message>(
        &self,
        topic: &str,
    ) -> impl Future<Output = ZResult<FutureQueueSubscriber<T>>>;
}

impl CreateFutureQueue for Node {
    async fn create_future_subscriber<T: Message>(
        &self,
        topic: &str,
    ) -> ZResult<FutureQueueSubscriber<T>> {
        let data_subscriber = self.subscriber(topic)?.build().await?;
        let announcement_subscriber = self
            .subscriber(&format!("{}/announce", topic))?
            .build()
            .await?;

        Ok(FutureQueueSubscriber {
            data_subscriber,
            announcement_subscriber,
            inflight: BTreeSet::new(),
            pending_data: VecDeque::new(),
        })
    }
}

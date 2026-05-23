use std::{
    collections::{BTreeSet, VecDeque},
    time::Duration,
};

use ros_z::{
    Message, Result,
    node::Node,
    pubsub::{Received, Subscriber},
    time::Time,
};
use tokio::select;

use crate::announce::Announcement;

/// Subscriber that tracks in-flight messages for a single stream.
///
/// This struct maintains two collections:
/// - `inflight`: A set of announced timestamps awaiting corresponding data
/// - `pending_data`: A queue of received messages awaiting matching announcements
///
/// The queue enforces a strict ordering: data is only released when both the
/// announcement has arrived and the data payload has been received, ensuring
/// temporal consistency even with variable network delays and out-of-order delivery.
#[derive(Debug)]
pub struct FutureQueueSubscriber<T: Message> {
    data_subscriber: Subscriber<T>,
    announcement_subscriber: Subscriber<Announcement>,
    inflight: BTreeSet<Announcement>,
    pending_data: VecDeque<Received<T>>,
    transit_lag: Duration,
}

/// Events from a single stream subscription.
///
/// The subscriber emits these events to the fusion engine, which buffers them
/// according to their timestamp and arrival pattern.
pub enum QueueEvent<T> {
    /// A new timestamp announcement was received for future data on this stream.
    ///
    /// This indicates that data with the announced timestamp is coming.
    /// The fusion engine uses this to update its global safe-time boundaries.
    Announcement,
    /// Data message with matched announcement arrived on this stream.
    ///
    /// The timestamp is the value announced earlier. The data is now available
    /// for inclusion in the time-ordered persistent buffer.
    Data(Time, T),
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
    pub(crate) fn safe_time(&self, now: Time) -> Time {
        let transit_boundary = now - self.transit_lag;
        self.inflight
            .first()
            .map_or(transit_boundary, |announcement| {
                announcement.time.min(transit_boundary)
            })
    }

    /// Returns the transit lag for this stream.
    pub(crate) fn transit_lag(&self) -> Duration {
        self.transit_lag
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

    /// Wait for the next publishable data event from this stream.
    ///
    /// This method blocks until either an announcement or data is received.
    /// It returns `QueueEvent::Data` only when both the announcement and data
    /// for a message have arrived, ensuring temporal ordering at the stream level.
    pub async fn recv(&mut self) -> Result<QueueEvent<T>> {
        loop {
            if let Some((time, data)) = self.next_publishable() {
                return Ok(QueueEvent::Data(time, data));
            }

            select! {
                announcement = self.announcement_subscriber.recv() => {
                    self.inflight.insert(announcement?);
                    return Ok(QueueEvent::Announcement);
                }
                data = self.data_subscriber.recv_with_metadata() => {
                    self.pending_data.push_back(data?);
                }
            }
        }
    }
}

/// Extension trait for creating future queue subscribers.
///
/// This trait extends [`ros_z::node::Node`] to provide convenient construction
/// of subscribers that coordinate announcements with data delivery for a single stream.
pub trait CreateFutureQueue {
    /// Subscribe to a topic with configured transit safety lag.
    ///
    /// Creates a [`FutureQueueSubscriber<T>`] that tracks in-flight messages on the given
    /// topic. The corresponding announcements must be published on `{topic}/announce` using
    /// an [`crate::AnnouncingPublisher`].
    ///
    /// # Arguments
    /// * `topic` - The base topic name for data messages
    /// * `transit_lag` - Maximum expected duration between announcement and data receipt.
    ///   This safety margin ensures data is held long enough for delayed announcements
    ///   to arrive from slow transports.
    ///
    /// # Returns
    /// A future that resolves to a [`FutureQueueSubscriber<T>`] when the subscription
    /// is established and ready to receive messages.
    fn create_future_subscriber<T: Message>(
        &self,
        topic: &str,
        transit_lag: Duration,
    ) -> impl Future<Output = Result<FutureQueueSubscriber<T>>>;
}

impl CreateFutureQueue for Node {
    async fn create_future_subscriber<T: Message>(
        &self,
        topic: &str,
        transit_lag: Duration,
    ) -> Result<FutureQueueSubscriber<T>> {
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
            transit_lag,
        })
    }
}

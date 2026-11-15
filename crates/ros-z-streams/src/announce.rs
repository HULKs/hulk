use std::fmt::Debug;

use ros_z::{
    EndpointGlobalId, Message, Result,
    node::Node,
    pubsub::{PreparedPublication, Publisher},
    time::Time,
};
use serde::{Deserialize, Serialize};

/// Announcement that marks an in-flight message at a given timestamp.
///
/// Announcements are sent before the actual output of a node has been computed to allow downstream subscribers
/// to anticipate data arrival and prepare buffers. This enables deterministic stream
/// fusion even with variable network delays.
#[derive(Debug, Serialize, Deserialize, Message, PartialEq, Eq, PartialOrd, Ord)]
pub struct Announcement {
    pub(crate) time: Time,
    pub(crate) source_global_id: EndpointGlobalId,
    pub(crate) sequence_number: i64,
}

/// Publisher that emits announcements before sending payload data.
///
/// This publisher coordinates with its corresponding data channel to ensure that
/// timestamps are announced before the actual node output is transmitted. This two-phase
/// pattern allows receivers to buffer data intelligently and maintain time-ordered
/// streams even when messages arrive out-of-order or with variable latencies.
pub struct AnnouncingPublisher<T: Message> {
    data_publisher: Publisher<T>,
    announcement_publisher: Publisher<Announcement>,
}

impl<T: Message> AnnouncingPublisher<T> {
    /// Announce upcoming payload timestamp and return handle to publish payload with matching id.
    ///
    /// This method sends a timestamp announcement and returns a [`PendingAnnouncement`]
    /// handle that must be used to publish the corresponding payload. The announcement serves
    /// as a signal to downstream receivers that data with the given timestamp is incoming.
    pub async fn announce(&self, time: Time) -> Result<PendingAnnouncement<'_, T>> {
        let data = self.data_publisher.prepare();
        let publication_id = data.id();
        self.announcement_publisher
            .publish(&Announcement {
                time,
                source_global_id: publication_id.endpoint_global_id(),
                sequence_number: publication_id.sequence_number(),
            })
            .await?;

        Ok(PendingAnnouncement {
            data: Some(data),
            time,
        })
    }
}

/// Extension trait for creating announcing publishers.
///
/// This trait extends [`ros_z::node::Node`] to provide convenient construction
/// of announcing publishers, which coordinate timestamp announcements with payload delivery.
pub trait CreateAnnouncingPublisher {
    /// Create announcing publisher for a topic and its corresponding announce channel.
    ///
    /// Creates an [`AnnouncingPublisher<T>`] that manages both the primary data stream
    /// on the given topic and a separate announce stream on `{topic}/announce`.
    /// Messages sent through this publisher are announced before transmission.
    fn announcing_publisher<T: Message>(
        &self,
        topic: &str,
    ) -> impl Future<Output = Result<AnnouncingPublisher<T>>>;
}

impl CreateAnnouncingPublisher for Node {
    async fn announcing_publisher<T: Message>(
        &self,
        topic: &str,
    ) -> Result<AnnouncingPublisher<T>> {
        let data_publisher = self.publisher(topic).build().await?;
        let announcement_publisher = self.publisher(&format!("{topic}/announce")).build().await?;

        Ok(AnnouncingPublisher {
            data_publisher,
            announcement_publisher,
        })
    }
}

/// Handle returned by [`AnnouncingPublisher::announce`] to publish matching payload.
///
/// This handle ensures that for each announcement, exactly one payload is published.
/// The announcement and payload must be published in order—the announcement first,
/// then the payload via this handle. If this handle is dropped without publishing,
/// an error is logged.
#[must_use = "data must be published before the announcement is dropped"]
pub struct PendingAnnouncement<'a, T: Message> {
    data: Option<PreparedPublication<'a, T, T::Codec>>,
    time: Time,
}

impl<'a, T: Message> PendingAnnouncement<'a, T> {
    /// Publish payload with id that matches earlier announcement.
    ///
    /// This completes the announcement-payload cycle by sending the actual data.
    /// The payload is published with the same message ID as the prior announcement,
    /// allowing receivers to correlate them and maintain temporal ordering. Its source time is
    /// the announced time, so that the payload alone tells which time it refers to.
    pub async fn publish(mut self, value: &T) -> Result<()> {
        let data = self.data.take().expect("can only publish once");
        data.publish_with_source_time(value, self.time).await
    }
}

impl<'a, T: Message> Drop for PendingAnnouncement<'a, T> {
    fn drop(&mut self) {
        if self.data.is_some() {
            log::error!("dropped a PendingAnnouncement before completion")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::prelude::*;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn payload_keeps_the_announced_source_time() {
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(std::iter::empty::<&str>())
            .build()
            .await
            .unwrap();
        let node = context
            .create_node("announcement_stamp")
            .build()
            .await
            .unwrap();
        let payload = node.subscriber::<u32>("data").build().await.unwrap();
        let publisher = node.announcing_publisher::<u32>("data").await.unwrap();
        let time = node.clock().now() - Duration::from_millis(100);
        publisher
            .announce(time)
            .await
            .unwrap()
            .publish(&42)
            .await
            .unwrap();
        let received = tokio::time::timeout(Duration::from_secs(3), payload.recv_with_metadata())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received.message, 42);
        assert_eq!(received.source_time, time);
        context.shutdown().unwrap();
    }
}

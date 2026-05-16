use std::fmt::Debug;

use ros_z::{
    EndpointGlobalId, Message, Result,
    node::Node,
    pubsub::{PreparedPublication, Publisher},
    time::Time,
};
use serde::{Deserialize, Serialize};

/// Lightweight announcement that marks message id as in-flight at given timestamp.
#[derive(Debug, Serialize, Deserialize, Message, PartialEq, Eq, PartialOrd, Ord)]
pub struct Announcement {
    pub(crate) time: Time,
    pub(crate) source_global_id: EndpointGlobalId,
    pub(crate) sequence_number: i64,
}

/// Publisher that emits announcements before payload transmission.
pub struct AnnouncingPublisher<T: Message> {
    data_publisher: Publisher<T>,
    announcement_publisher: Publisher<Announcement>,
}

impl<T: Message> AnnouncingPublisher<T> {
    /// Announce upcoming payload timestamp and return handle to publish payload with matching id.
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

        Ok(PendingAnnouncement { data: Some(data) })
    }
}

/// Extension trait for creating announcing publishers.
pub trait CreateAnnouncingPublisher {
    /// Create announcing publisher for topic and its corresponding announce topic.
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
        let data_publisher = self.publisher(topic)?.build().await?;
        let announcement_publisher = self
            .publisher(&format!("{topic}/announce"))?
            .build()
            .await?;

        Ok(AnnouncingPublisher {
            data_publisher,
            announcement_publisher,
        })
    }
}

/// Handle returned by [`AnnouncingPublisher::announce`] to publish matching payload.
#[must_use = "data must be published before the announcement is dropped"]
pub struct PendingAnnouncement<'a, T: Message> {
    data: Option<PreparedPublication<'a, T, T::Codec>>,
}

impl<'a, T: Message> PendingAnnouncement<'a, T> {
    /// Publish payload with id that matches earlier announcement.
    pub async fn publish(mut self, value: &T) -> Result<()> {
        let data = self.data.take().expect("can only publish once");
        data.publish(value).await?;
        Ok(())
    }
}

impl<'a, T: Message> Drop for PendingAnnouncement<'a, T> {
    fn drop(&mut self) {
        if self.data.is_some() {
            log::error!("dropped a PendingAnnouncement before completion")
        }
    }
}

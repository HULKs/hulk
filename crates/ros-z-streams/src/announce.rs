use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ros_z::{
    EndpointGlobalId, Message,
    msg::WireEncoder,
    node::Node,
    pubsub::{PreparedPublication, PublicationId, Publisher},
    time::Time,
};
use serde::{Deserialize, Serialize};
use zenoh::Result as ZResult;

/// Lightweight announcement that marks message id as in-flight at given timestamp.
#[derive(Debug, Serialize, Deserialize, Message)]
#[message(name = "ros_z_streams::Announcement")]
pub struct Announcement {
    pub(crate) time: Time,
    pub(crate) source_global_id: EndpointGlobalId,
    pub(crate) sequence_number: i64,
    pub(crate) canceled: bool,
}

/// Publisher that emits lightweight announcements before payload transmission.
pub struct AnnouncingPublisher<T>
where
    T: Message,
    for<'a> T::Codec: WireEncoder<Input<'a> = &'a T>,
{
    data_publisher: Publisher<T>,
    announcement_publisher: Arc<Publisher<Announcement>>,
}

impl<T> AnnouncingPublisher<T>
where
    T: Message,
    for<'a> T::Codec: WireEncoder<Input<'a> = &'a T>,
{
    /// Announce upcoming payload timestamp and return handle to publish payload with matching id.
    pub async fn announce(&self, time: Time) -> ZResult<PendingAnnouncement<'_, T>> {
        let data = self.data_publisher.prepare();
        let publication_id = data.id();
        self.announcement_publisher
            .publish(&Announcement {
                time,
                source_global_id: publication_id.endpoint_global_id(),
                sequence_number: publication_id.sequence_number(),
                canceled: false,
            })
            .await?;
        Ok(PendingAnnouncement {
            data: Some(data),
            announcement_publisher: Arc::clone(&self.announcement_publisher),
            time,
            source_global_id: publication_id.endpoint_global_id(),
            sequence_number: publication_id.sequence_number(),
            completed: false,
        })
    }
}

/// Extension trait for creating announcing publishers.
pub trait CreateAnnouncingPublisher {
    /// Create announcing publisher for topic and its corresponding announce topic.
    fn announcing_publisher<'a, T>(
        &'a self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = ZResult<AnnouncingPublisher<T>>> + 'a>>
    where
        T: Message + 'a,
        for<'de> T::Codec: WireEncoder<Input<'de> = &'de T>;
}

impl CreateAnnouncingPublisher for Node {
    fn announcing_publisher<'a, T>(
        &'a self,
        topic: &'a str,
    ) -> Pin<Box<dyn Future<Output = ZResult<AnnouncingPublisher<T>>> + 'a>>
    where
        T: Message + 'a,
        for<'de> T::Codec: WireEncoder<Input<'de> = &'de T>,
    {
        Box::pin(async move {
            let data_publisher = self.publisher(topic).build().await?;
            let announcement_publisher =
                Arc::new(self.publisher(&format!("{topic}/announce")).build().await?);

            Ok(AnnouncingPublisher {
                data_publisher,
                announcement_publisher,
            })
        })
    }
}

/// Handle returned by [`AnnouncingPublisher::announce`] to publish matching payload.
#[must_use = "data must be published before the announcement is dropped"]
pub struct PendingAnnouncement<'a, T>
where
    T: Message,
    for<'b> T::Codec: WireEncoder<Input<'b> = &'b T>,
{
    data: Option<PreparedPublication<'a, T, T::Codec>>,
    announcement_publisher: Arc<Publisher<Announcement>>,
    time: Time,
    source_global_id: EndpointGlobalId,
    sequence_number: i64,
    completed: bool,
}

impl<'a, T> PendingAnnouncement<'a, T>
where
    T: Message,
    for<'b> T::Codec: WireEncoder<Input<'b> = &'b T>,
{
    /// Publish payload with id that matches earlier announcement.
    pub async fn publish(mut self, value: &T) -> ZResult<()> {
        let data = self.data.take().expect("pending announcement data missing");
        data.publish(value).await?;
        self.completed = true;
        Ok(())
    }

    /// Return id that was reserved when announcement was published.
    pub fn id(&self) -> PublicationId {
        self.data
            .as_ref()
            .expect("pending announcement data missing")
            .id()
    }
}

impl<'a, T> Drop for PendingAnnouncement<'a, T>
where
    T: Message,
    for<'b> T::Codec: WireEncoder<Input<'b> = &'b T>,
{
    fn drop(&mut self) {
        if self.completed {
            return;
        }

        let publisher = Arc::clone(&self.announcement_publisher);
        let announcement = Announcement {
            time: self.time,
            source_global_id: self.source_global_id,
            sequence_number: self.sequence_number,
            canceled: true,
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = publisher.publish(&announcement).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ros_z::{context::ContextBuilder, time::Time};
    use ros_z_msgs::std_msgs::String as TestMessage;

    use super::{Announcement, CreateAnnouncingPublisher};

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn announcing_publisher_reuses_reserved_id_for_announcement_and_payload() {
        let context = ContextBuilder::default()
            .build()
            .await
            .expect("create context");
        let node = context
            .create_node("announce_alignment")
            .build()
            .await
            .expect("create node");

        let publisher = node
            .announcing_publisher::<TestMessage>("alignment/topic")
            .await
            .expect("create announcing publisher");
        let announcement_subscriber = node
            .subscriber::<Announcement>("alignment/topic/announce")
            .build()
            .await
            .expect("create announcement subscriber");
        let data_subscriber = node
            .subscriber::<TestMessage>("alignment/topic")
            .build()
            .await
            .expect("create data subscriber");

        tokio::time::sleep(Duration::from_millis(100)).await;

        let pending = publisher
            .announce(Time::from_nanos(42))
            .await
            .expect("announce payload");
        let publication_id = pending.id();

        let received_announcement =
            tokio::time::timeout(Duration::from_secs(2), announcement_subscriber.recv())
                .await
                .expect("timeout waiting for announcement")
                .expect("receive announcement");
        assert_eq!(
            received_announcement.source_global_id,
            publication_id.endpoint_global_id()
        );
        assert_eq!(
            received_announcement.sequence_number,
            publication_id.sequence_number()
        );

        pending
            .publish(&TestMessage {
                data: "payload".to_owned(),
            })
            .await
            .expect("publish payload");

        let received_payload =
            tokio::time::timeout(Duration::from_secs(2), data_subscriber.recv_with_metadata())
                .await
                .expect("timeout waiting for payload")
                .expect("receive payload");
        assert_eq!(received_payload.publication_id(), Some(publication_id));
    }
}

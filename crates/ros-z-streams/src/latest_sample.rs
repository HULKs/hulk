use std::num::NonZeroUsize;

use ros_z::{
    Message, Result,
    node::Node,
    pubsub::{QueueOverflowReporting, Subscriber},
    time::Time,
};

/// The latest received message of a topic, together with the time it refers to.
///
/// A [`ros_z::cache::Cache`] indexes messages by transport time unless it runs on a logical clock.
/// That is the time of publication, whereas the time data refers to is its source time, which may
/// lie much further back: estimators publish after their fusion lag and hold their output until
/// the next update. Data is still taken in the order it arrives.
pub struct LatestSample<T: Message> {
    subscriber: Subscriber<T>,
    latest: Option<(Time, T)>,
}

impl<T: Message> LatestSample<T> {
    /// Subscribes to `topic`.
    pub async fn new(node: &Node, topic: &str) -> Result<Self> {
        Ok(Self {
            subscriber: node
                .subscriber::<T>(topic)
                // Only the newest message counts, so older ones need not be decoded.
                .queue_capacity(NonZeroUsize::MIN)
                .queue_overflow_reporting(QueueOverflowReporting::Silent)
                .build()
                .await?,
            latest: None,
        })
    }

    /// Takes in what has been received since the last update.
    pub async fn update(&mut self) -> Result<()> {
        if self.subscriber.is_ready() {
            let received = self.subscriber.recv_with_metadata().await?;
            self.latest = Some((received.source_time, received.message));
        }
        Ok(())
    }

    /// The source time and the message as of the last update.
    pub fn latest(&self) -> Option<(Time, &T)> {
        self.latest.as_ref().map(|(time, message)| (*time, message))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ros_z::prelude::*;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn keeps_the_source_time_of_the_last_arrival() {
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(std::iter::empty::<&str>())
            .build()
            .await
            .unwrap();
        let node = context.create_node("latest_sample").build().await.unwrap();
        let mut sample = LatestSample::<u32>::new(&node, "value").await.unwrap();
        let publisher = node.publisher::<u32>("value").build().await.unwrap();
        assert!(
            publisher
                .wait_for_subscribers(1, Duration::from_secs(3))
                .await
        );

        sample.update().await.unwrap();
        assert_eq!(sample.latest(), None);

        for (value, nanos) in [(1, 2_000), (2, 3_000), (3, 1_000)] {
            publisher
                .publish_with_source_time(&value, Time::from_nanos(nanos))
                .await
                .unwrap();
        }
        tokio::time::timeout(Duration::from_secs(3), async {
            while sample.latest() != Some((Time::from_nanos(1_000), &3)) {
                sample.update().await.unwrap();
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        // Without anything new, the message is still there.
        sample.update().await.unwrap();
        assert_eq!(sample.latest(), Some((Time::from_nanos(1_000), &3)));
        context.shutdown().unwrap();
    }
}

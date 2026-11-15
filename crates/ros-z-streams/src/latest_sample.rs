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

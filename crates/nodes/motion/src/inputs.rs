use std::sync::Arc;
use std::time::Duration;

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use ros_z::{
    Message,
    prelude::*,
    pubsub::Received,
    time::{Clock, Time},
};
use tokio::{sync::watch, task::JoinHandle};

pub(super) struct Sample<T> {
    pub received: Received<T>,
    pub receipt_time: Time,
}

impl<T> Sample<T> {
    pub fn validate_freshness(&self, now: Time, age: Duration) -> Result<()> {
        let now_ns = now.as_nanos();
        let source_age_ns = now_ns - self.received.source_time.as_nanos();
        let receipt_age_ns = now_ns - self.receipt_time.as_nanos();
        ensure!(
            self.received.source_time <= now && self.receipt_time <= now,
            "input timestamp is in the future: now_ns={now_ns}, \
             source_age_ns={source_age_ns}, receipt_age_ns={receipt_age_ns}"
        );
        ensure!(
            now.duration_since(self.received.source_time) <= age
                && now.duration_since(self.receipt_time) <= age,
            "input expired: now_ns={now_ns}, source_age_ns={source_age_ns}, \
             receipt_age_ns={receipt_age_ns}, maximum_age_ns={}",
            age.as_nanos()
        );
        Ok(())
    }
}

/// Retains source and local receipt times; a stopped receiver cannot renew its lease.
pub(super) struct Latest<T> {
    values: watch::Receiver<Option<Arc<Sample<T>>>>,
    task: JoinHandle<()>,
}
impl<T: Message + Send + Sync + 'static> Latest<T>
where
    T::Codec: Send + Sync,
{
    pub async fn subscribe(node: &Node, topic: &str, qos: QosProfile) -> Result<Self> {
        let subscriber = node.subscriber::<T>(topic).qos(qos).build().await?;
        let (sender, values) = watch::channel(None);
        let clock = node.clock().clone();
        let task = tokio::spawn(async move {
            let mut last = None;
            while let Ok(received) = subscriber.recv_with_metadata().await {
                // Logical time can be paused while lifecycle states change (e.g.
                // inference Idle -> Initialized). Keep arrival order for equal
                // timestamps; freshness still checks the original source time.
                if last.is_some_and(|time| received.source_time < time) {
                    continue;
                }
                last = Some(received.source_time);
                sender.send_replace(Some(Arc::new(Sample {
                    received,
                    receipt_time: clock.now(),
                })));
            }
        });
        Ok(Self { values, task })
    }
    pub fn latest(&self) -> Option<Arc<Sample<T>>> {
        self.values.borrow().clone()
    }
    pub fn snapshot(&self) -> Result<Arc<Sample<T>>> {
        ensure!(!self.task.is_finished(), "input receiver stopped");
        self.latest().ok_or_else(|| eyre!("input unavailable"))
    }
    pub fn fresh(&self, clock: &Clock, age: Duration) -> Result<Arc<Sample<T>>> {
        let sample = self.snapshot()?;
        // A subscription may update concurrently, so capture the sample before the clock.
        sample.validate_freshness(clock.now(), age)?;
        Ok(sample)
    }
}
impl<T> Drop for Latest<T> {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motion_inference::node::{State, Status};

    #[tokio::test(flavor = "multi_thread")]
    async fn accepts_state_changes_while_paused_without_renewing_source_freshness() {
        let time = Time::from_nanos(1_000_000);
        let clock = Clock::logical(time);
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(std::iter::empty::<&str>())
            .with_clock(clock.clone())
            .build()
            .await
            .unwrap();
        let node = context
            .create_node("paused_state_test")
            .build()
            .await
            .unwrap();
        let status = node.publisher::<Status>("status").build().await.unwrap();
        let latest = Latest::<Status>::subscribe(&node, "status", QosProfile::default())
            .await
            .unwrap();
        let mut changes = latest.values.clone();

        for state in [State::Idle, State::Initialized] {
            status.publish(&Status { time, state }).await.unwrap();
            tokio::time::timeout(Duration::from_secs(2), changes.changed())
                .await
                .unwrap()
                .unwrap();
        }
        assert!(matches!(
            latest.snapshot().unwrap().received.state,
            State::Initialized
        ));
        assert_eq!(clock.now(), time);

        status
            .publish_with_source_time(
                &Status {
                    time: Time::zero(),
                    state: State::Idle,
                },
                Time::zero(),
            )
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), changes.changed())
                .await
                .is_err()
        );
        assert!(matches!(
            latest.snapshot().unwrap().received.state,
            State::Initialized
        ));

        clock.set_time(time + Duration::from_millis(20)).unwrap();
        status
            .publish_with_source_time(
                &Status {
                    time,
                    state: State::Initialized,
                },
                time,
            )
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), changes.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(
            latest.fresh(&clock, Duration::from_millis(10)).is_err(),
            "retransmission must not renew an expired source sample"
        );
        context.shutdown().unwrap();
    }
}

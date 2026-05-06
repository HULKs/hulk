use std::time::Duration;

use ros_z::prelude::*;
use ros_z::time::{Clock, Time};
use ros_z_streams::{CreateAnnouncingPublisher, CreateFutureQueue, LagPolicy, QueueEvent};
#[tokio::main]
async fn main() -> zenoh::Result<()> {
    let logical = Clock::logical(Time::zero());
    let context = ContextBuilder::default()
        .with_namespace("/ros_z_streams_example")
        .with_clock(logical.clone())
        .build()
        .await?;
    let node = context.create_node("future_queue_example").build().await?;

    let publisher = node.announcing_publisher::<String>("sensors/a").await?;
    let mut queue = node
        .create_future_subscriber::<String>(
            "sensors/a",
            LagPolicy::Watermark {
                max_lag: Duration::from_millis(20),
            },
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    logical
        .set_time(Time::from_nanos(100_000_000))
        .expect("set logical clock to 100ms");
    let pending_10 = publisher.announce(Time::from_nanos(10)).await?;

    logical
        .advance(Duration::from_millis(40))
        .expect("advance logical clock by 40ms");
    let pending_20 = publisher.announce(Time::from_nanos(20)).await?;

    logical
        .advance(Duration::from_millis(20))
        .expect("advance logical clock by 20ms");
    pending_20.publish(&"message@20".to_owned()).await?;

    logical
        .advance(Duration::from_millis(5))
        .expect("advance logical clock by 5ms");
    pending_10.publish(&"message@10".to_owned()).await?;

    let mut received = 0usize;
    while received < 2 {
        match queue.recv().await? {
            QueueEvent::Announcement { state } => {
                println!(
                    "announcement: safe_time={:?}, reference_time={:?}, effective_lag={:?}, warning={:?}",
                    state.safe_time, state.reference_time, state.effective_lag, state.warning
                );
            }
            QueueEvent::Data {
                state,
                data_time,
                value,
            } => {
                println!(
                    "data: time={:?}, payload={}, safe_time={:?}, reference_time={:?}, effective_lag={:?}, warning={:?}",
                    data_time,
                    value,
                    state.safe_time,
                    state.reference_time,
                    state.effective_lag,
                    state.warning
                );
                received += 1;
            }
        }
    }

    Ok(())
}

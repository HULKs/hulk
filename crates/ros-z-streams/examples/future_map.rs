use std::time::Duration;

use ros_z::prelude::*;
use ros_z::time::{Clock, Time};
use ros_z_msgs::std_msgs::String as RosString;
use ros_z_streams::{CreateAnnouncingPublisher, CreateFutureMapBuilder, LagPolicy};

fn t(nanos: i64) -> Time {
    Time::from_nanos(nanos)
}

#[tokio::main]
async fn main() -> zenoh::Result<()> {
    let logical = Clock::logical(Time::zero());
    let context = ContextBuilder::default()
        .with_namespace("/ros_z_streams_future_map_example")
        .with_clock(logical.clone())
        .build()
        .await?;
    let node = context.create_node("future_map_example").build().await?;

    let pub_a = node.announcing_publisher::<RosString>("fusion/a").await?;
    let pub_b = node.announcing_publisher::<RosString>("fusion/b").await?;

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<RosString>(
            "fusion/a",
            LagPolicy::Watermark {
                max_lag: Duration::from_millis(20),
            },
        )
        .await?
        .create_future_subscriber::<RosString>("fusion/b", LagPolicy::Immediate)
        .await?
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    logical
        .set_time(t(100_000_000))
        .expect("set logical clock to 100ms");
    pub_a
        .announce(t(10))
        .await?
        .publish(&RosString {
            data: "a@10".to_owned(),
        })
        .await?;

    let out1 = map.recv().await?;
    println!(
        "out1: persistent={:?} temporary={:?} state_a={:?} state_b={:?}",
        out1.item.persistent.keys().collect::<Vec<_>>(),
        out1.item.temporary.keys().collect::<Vec<_>>(),
        out1.stream_states.0,
        out1.stream_states.1
    );

    logical
        .advance(Duration::from_millis(35))
        .expect("advance logical clock by 35ms");
    pub_b
        .announce(t(10))
        .await?
        .publish(&RosString {
            data: "b@10".to_owned(),
        })
        .await?;

    let sender = pub_a.announce(t(15)).await?;

    let out2 = map.recv().await?;
    println!(
        "out2: persistent={:?} temporary={:?} state_a={:?} state_b={:?}",
        out2.item.persistent.keys().collect::<Vec<_>>(),
        out2.item.temporary.keys().collect::<Vec<_>>(),
        out2.stream_states.0,
        out2.stream_states.1
    );

    pub_b
        .announce(t(20))
        .await?
        .publish(&RosString {
            data: "b@20".to_owned(),
        })
        .await?;

    let out3 = map.recv().await?;
    println!(
        "out2: persistent={:?} temporary={:?} state_a={:?} state_b={:?}",
        out3.item.persistent.keys().collect::<Vec<_>>(),
        out3.item.temporary.keys().collect::<Vec<_>>(),
        out3.stream_states.0,
        out3.stream_states.1
    );

    sender
        .publish(&RosString {
            data: "a@10".to_string(),
        })
        .await?;

    for (ts, (a, b)) in &out3.item.persistent {
        println!(
            "ts={:?} a={:?} b={:?}",
            ts,
            a.as_ref().map(|m| m.data.as_str()),
            b.as_ref().map(|m| m.data.as_str())
        );
    }

    Ok(())
}

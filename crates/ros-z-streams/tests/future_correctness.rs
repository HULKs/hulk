use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use ros_z::{context::ContextBuilder, time::Clock, time::Time};
use ros_z_msgs::std_msgs::String as TestMessage;
use ros_z_streams::{
    CreateAnnouncingPublisher, CreateFutureMapBuilder, CreateFutureQueue, FutureReceive, LagPolicy,
    LagWarning, QueueEvent,
};

static NEXT_NS: AtomicU64 = AtomicU64::new(1);

fn test_ns(label: &str) -> String {
    let id = NEXT_NS.fetch_add(1, Ordering::Relaxed);
    format!("/ros_z_streams_{label}_{id}")
}

fn t(nanos: i64) -> Time {
    Time::from_nanos(nanos)
}

fn keys<T>(map: &BTreeMap<Time, T>) -> Vec<Time> {
    map.keys().copied().collect()
}

async fn recv_queue(
    sub: &mut ros_z_streams::FutureQueueSubscriber<TestMessage>,
) -> QueueEvent<TestMessage> {
    tokio::time::timeout(Duration::from_secs(2), sub.recv())
        .await
        .expect("timeout while waiting for queue message")
        .expect("queue receive failed")
}

async fn recv_map1(
    map: &mut ros_z_streams::FutureMap<(TestMessage,)>,
) -> FutureReceive<'_, (Option<TestMessage>,), (ros_z_streams::QueueState,)> {
    tokio::time::timeout(Duration::from_secs(2), map.recv())
        .await
        .expect("timeout while waiting for map message")
        .expect("map receive failed")
}

async fn recv_map2(
    map: &mut ros_z_streams::FutureMap<(TestMessage, TestMessage)>,
) -> FutureReceive<
    '_,
    (Option<TestMessage>, Option<TestMessage>),
    (ros_z_streams::QueueState, ros_z_streams::QueueState),
> {
    tokio::time::timeout(Duration::from_secs(2), map.recv())
        .await
        .expect("timeout while waiting for map message")
        .expect("map receive failed")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn single_stream_in_order_ideal_flow() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("single_in_order"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher");
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    pub_a
        .announce(t(10))
        .await
        .expect("announce t10")
        .publish(&TestMessage {
            data: "10".to_owned(),
        })
        .await
        .expect("publish t10");
    let out1 = recv_map1(&mut map).await;
    assert_eq!(keys(&out1.item.persistent), vec![t(10)]);
    assert!(out1.item.temporary.is_empty());
    assert_eq!(
        out1.item.persistent[&t(10)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("10")
    );

    pub_a
        .announce(t(20))
        .await
        .expect("announce t20")
        .publish(&TestMessage {
            data: "20".to_owned(),
        })
        .await
        .expect("publish t20");
    let out2 = recv_map1(&mut map).await;
    assert_eq!(keys(&out2.item.persistent), vec![t(20)]);
    assert!(out2.item.temporary.is_empty());
    assert_eq!(
        out2.item.persistent[&t(20)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("20")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn single_stream_out_of_order_receipt_delayed_payload() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("single_out_of_order"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher");
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let p10 = pub_a.announce(t(10)).await.expect("announce t10");
    let p20 = pub_a.announce(t(20)).await.expect("announce t20");
    tokio::time::sleep(Duration::from_millis(50)).await;

    p20.publish(&TestMessage {
        data: "20".to_owned(),
    })
    .await
    .expect("publish t20");
    let out1 = recv_map1(&mut map).await;
    assert!(out1.item.persistent.is_empty());
    assert_eq!(keys(out1.item.temporary), vec![t(20)]);
    assert_eq!(
        out1.item.temporary[&t(20)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("20")
    );

    p10.publish(&TestMessage {
        data: "10".to_owned(),
    })
    .await
    .expect("publish t10");
    let out2 = recv_map1(&mut map).await;
    assert_eq!(keys(&out2.item.persistent), vec![t(10), t(20)]);
    assert!(out2.item.temporary.is_empty());
    assert_eq!(
        out2.item.persistent[&t(10)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("10")
    );
    assert_eq!(
        out2.item.persistent[&t(20)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("20")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn dropped_pending_announcement_releases_safe_time() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("dropped_pending_releases"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher");
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let p10 = pub_a.announce(t(10)).await.expect("announce t10");
    drop(p10);
    tokio::time::sleep(Duration::from_millis(50)).await;

    pub_a
        .announce(t(20))
        .await
        .expect("announce t20")
        .publish(&TestMessage {
            data: "20".to_owned(),
        })
        .await
        .expect("publish t20");

    let out = recv_map1(&mut map).await;
    assert_eq!(keys(&out.item.persistent), vec![t(20)]);
    assert!(out.item.temporary.is_empty());
    assert_eq!(out.stream_states.0.safe_time, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn canceled_pending_announcement_releases_existing_temporary_data() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("canceled_pending_releases_temporary"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher");
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let p10 = pub_a.announce(t(10)).await.expect("announce t10");
    let p20 = pub_a.announce(t(20)).await.expect("announce t20");
    tokio::time::sleep(Duration::from_millis(50)).await;

    p20.publish(&TestMessage {
        data: "20".to_owned(),
    })
    .await
    .expect("publish t20");
    let out1 = recv_map1(&mut map).await;
    assert!(out1.item.persistent.is_empty());
    assert_eq!(keys(out1.item.temporary), vec![t(20)]);

    drop(out1);
    drop(p10);

    let out2 = recv_map1(&mut map).await;
    assert_eq!(keys(&out2.item.persistent), vec![t(20)]);
    assert!(out2.item.temporary.is_empty());
    assert_eq!(out2.stream_states.0.safe_time, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn future_queue_out_of_order_receipt_reports_oldest_inflight() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("queue_out_of_order"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("queue/a")
        .await
        .expect("create publisher");
    let mut sub = node
        .create_future_subscriber::<TestMessage>("queue/a", LagPolicy::Immediate)
        .await
        .expect("create subscriber");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let p10 = pub_a.announce(t(10)).await.expect("announce t10");
    let p20 = pub_a.announce(t(20)).await.expect("announce t20");
    tokio::time::sleep(Duration::from_millis(50)).await;

    p20.publish(&TestMessage {
        data: "20".to_owned(),
    })
    .await
    .expect("publish t20");

    let (oldest1, time1, msg1) = loop {
        match recv_queue(&mut sub).await {
            QueueEvent::Announcement { .. } => {}
            QueueEvent::Data {
                state,
                data_time,
                value,
            } => break (state.safe_time, data_time, value),
        }
    };
    assert_eq!(oldest1, Some(t(10)));
    assert_eq!(time1, t(20));
    assert_eq!(msg1.data, "20");

    p10.publish(&TestMessage {
        data: "10".to_owned(),
    })
    .await
    .expect("publish t10");
    let (oldest2, time2, msg2) = match recv_queue(&mut sub).await {
        QueueEvent::Data {
            state,
            data_time,
            value,
        } => (state.safe_time, data_time, value),
        QueueEvent::Announcement { .. } => panic!("unexpected pure announcement event"),
    };
    assert_eq!(oldest2, None);
    assert_eq!(time2, t(10));
    assert_eq!(msg2.data, "10");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn multi_stream_lagging_stream_bottlenecks_fast_stream() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("lagging_bottleneck"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let _b10 = pub_b.announce(t(10)).await.expect("announce b10");

    pub_a
        .announce(t(10))
        .await
        .expect("announce a10")
        .publish(&TestMessage {
            data: "10".to_owned(),
        })
        .await
        .expect("publish a10");
    let out1 = recv_map2(&mut map).await;
    assert!(out1.item.persistent.is_empty());
    assert_eq!(keys(out1.item.temporary), vec![t(10)]);

    pub_a
        .announce(t(20))
        .await
        .expect("announce a20")
        .publish(&TestMessage {
            data: "20".to_owned(),
        })
        .await
        .expect("publish a20");
    let out2 = recv_map2(&mut map).await;
    assert!(out2.item.persistent.is_empty());
    assert_eq!(keys(out2.item.temporary), vec![t(10), t(20)]);

    pub_a
        .announce(t(30))
        .await
        .expect("announce a30")
        .publish(&TestMessage {
            data: "30".to_owned(),
        })
        .await
        .expect("publish a30");
    let out3 = recv_map2(&mut map).await;
    assert!(out3.item.persistent.is_empty());
    assert_eq!(keys(out3.item.temporary), vec![t(10), t(20), t(30)]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn multi_stream_release_valve_flushes_persistent_window() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("release_valve"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let b10 = pub_b.announce(t(10)).await.expect("announce b10");

    pub_a
        .announce(t(10))
        .await
        .expect("announce a10")
        .publish(&TestMessage {
            data: "110".to_owned(),
        })
        .await
        .expect("publish a10");
    let _ = recv_map2(&mut map).await;

    pub_a
        .announce(t(20))
        .await
        .expect("announce a20")
        .publish(&TestMessage {
            data: "120".to_owned(),
        })
        .await
        .expect("publish a20");
    let _ = recv_map2(&mut map).await;

    pub_a
        .announce(t(30))
        .await
        .expect("announce a30")
        .publish(&TestMessage {
            data: "130".to_owned(),
        })
        .await
        .expect("publish a30");
    let _ = recv_map2(&mut map).await;

    b10.publish(&TestMessage {
        data: "210".to_owned(),
    })
    .await
    .expect("publish b10");
    let out = recv_map2(&mut map).await;

    assert_eq!(keys(&out.item.persistent), vec![t(10), t(20), t(30)]);
    assert!(out.item.temporary.is_empty());
    assert_eq!(
        out.item.persistent[&t(10)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("110")
    );
    assert_eq!(
        out.item.persistent[&t(10)]
            .1
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("210")
    );
    assert_eq!(
        out.item.persistent[&t(20)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("120")
    );
    assert_eq!(
        out.item.persistent[&t(20)]
            .1
            .as_ref()
            .map(|m| m.data.as_str()),
        None
    );
    assert_eq!(
        out.item.persistent[&t(30)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("130")
    );
    assert_eq!(
        out.item.persistent[&t(30)]
            .1
            .as_ref()
            .map(|m| m.data.as_str()),
        None
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn multi_stream_partial_tuple_update_same_timestamp() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("partial_tuple"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let a100 = pub_a.announce(t(100)).await.expect("announce a100");
    let b100 = pub_b.announce(t(100)).await.expect("announce b100");
    tokio::time::sleep(Duration::from_millis(50)).await;

    a100.publish(&TestMessage {
        data: "1".to_owned(),
    })
    .await
    .expect("publish a100");
    let out1 = recv_map2(&mut map).await;
    assert!(out1.item.persistent.is_empty());
    assert_eq!(keys(out1.item.temporary), vec![t(100)]);
    assert_eq!(
        out1.item.temporary[&t(100)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("1")
    );
    assert_eq!(
        out1.item.temporary[&t(100)]
            .1
            .as_ref()
            .map(|m| m.data.as_str()),
        None
    );

    b100.publish(&TestMessage {
        data: "2".to_owned(),
    })
    .await
    .expect("publish b100");
    let out2 = recv_map2(&mut map).await;
    assert_eq!(keys(&out2.item.persistent), vec![t(100)]);
    assert!(out2.item.temporary.is_empty());
    assert_eq!(
        out2.item.persistent[&t(100)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("1")
    );
    assert_eq!(
        out2.item.persistent[&t(100)]
            .1
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("2")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn multi_stream_interleaved_safe_time_advancement() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("interleaved_safe_time"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let a10 = pub_a.announce(t(10)).await.expect("announce a10");
    let b15 = pub_b.announce(t(15)).await.expect("announce b15");

    b15.publish(&TestMessage {
        data: "15".to_owned(),
    })
    .await
    .expect("publish b15");
    let out1 = recv_map2(&mut map).await;
    assert!(out1.item.persistent.is_empty());
    assert_eq!(keys(out1.item.temporary), vec![t(15)]);

    let _a20 = pub_a.announce(t(20)).await.expect("announce a20");

    a10.publish(&TestMessage {
        data: "10".to_owned(),
    })
    .await
    .expect("publish a10");
    let out2 = recv_map2(&mut map).await;
    assert_eq!(keys(&out2.item.persistent), vec![t(10), t(15)]);
    assert!(out2.item.temporary.is_empty());
    assert_eq!(
        out2.item.persistent[&t(10)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("10")
    );
    assert_eq!(
        out2.item.persistent[&t(10)]
            .1
            .as_ref()
            .map(|m| m.data.as_str()),
        None
    );
    assert_eq!(
        out2.item.persistent[&t(15)]
            .0
            .as_ref()
            .map(|m| m.data.as_str()),
        None
    );
    assert_eq!(
        out2.item.persistent[&t(15)]
            .1
            .as_ref()
            .map(|m| m.data.as_str()),
        Some("15")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn watermark_lag_exceeded_warning_is_latched_and_cleared_after_return() {
    let context = ContextBuilder::default()
        .with_namespace(test_ns("watermark_warning"))
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let max_lag = Duration::from_nanos(1);
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Watermark { max_lag })
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let _ = pub_b.announce(t(10)).await.expect("announce b10");
    let _ = pub_b.announce(t(20)).await.expect("announce b20");

    pub_a
        .announce(t(30))
        .await
        .expect("announce a30")
        .publish(&TestMessage {
            data: "30".to_owned(),
        })
        .await
        .expect("publish a30");

    let first = recv_map2(&mut map).await;
    assert!(matches!(
        first.stream_states.1.warning,
        Some(LagWarning::LagExceeded { .. })
    ));

    pub_a
        .announce(t(40))
        .await
        .expect("announce a40")
        .publish(&TestMessage {
            data: "40".to_owned(),
        })
        .await
        .expect("publish a40");

    let second = recv_map2(&mut map).await;
    assert!(second.stream_states.1.warning.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn watermark_toggle_empty_stream_immediate_allows_persistent() {
    let clock = Clock::logical(t(0));
    let context = ContextBuilder::default()
        .with_namespace(test_ns("watermark_toggle_immediate"))
        .with_clock(clock.clone())
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Immediate)
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    clock.set_time(t(10)).expect("set logical time to 10");
    pub_a
        .announce(t(10))
        .await
        .expect("announce a10")
        .publish(&TestMessage {
            data: "a10".to_owned(),
        })
        .await
        .expect("publish a10");
    let _ = recv_map2(&mut map).await;

    clock.set_time(t(100)).expect("set logical time to 100");
    pub_b
        .announce(t(100))
        .await
        .expect("announce b100")
        .publish(&TestMessage {
            data: "b100".to_owned(),
        })
        .await
        .expect("publish b100");
    let out = recv_map2(&mut map).await;

    assert_eq!(keys(&out.item.persistent), vec![t(100)]);
    assert!(out.item.temporary.is_empty());
    assert_eq!(out.stream_states.0.safe_time, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn watermark_toggle_empty_stream_watermark_clamps_to_temporary() {
    let clock = Clock::logical(t(0));
    let context = ContextBuilder::default()
        .with_namespace(test_ns("watermark_toggle_clamp"))
        .with_clock(clock.clone())
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let max_lag = Duration::from_nanos(20);
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Watermark { max_lag })
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    clock.set_time(t(30)).expect("set logical time to 30");
    pub_a
        .announce(t(10))
        .await
        .expect("announce a10")
        .publish(&TestMessage {
            data: "a10".to_owned(),
        })
        .await
        .expect("publish a10");
    let _ = recv_map2(&mut map).await;

    pub_b
        .announce(t(100))
        .await
        .expect("announce b100")
        .publish(&TestMessage {
            data: "b100".to_owned(),
        })
        .await
        .expect("publish b100");
    let out = recv_map2(&mut map).await;

    assert!(out.item.persistent.is_empty());
    assert!(keys(out.item.temporary).contains(&t(100)));
    assert_eq!(out.stream_states.0.safe_time, Some(t(10)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn watermark_queue_and_map_emit_lag_exceeded() {
    let clock = Clock::logical(t(0));
    let max_lag = Duration::from_nanos(30);

    let queue_context = ContextBuilder::default()
        .with_namespace(test_ns("watermark_queue_warning"))
        .with_clock(clock.clone())
        .build()
        .await
        .expect("create queue context");
    let node_q = queue_context
        .create_node("nq")
        .build()
        .await
        .expect("create queue node");
    let pub_q = node_q
        .announcing_publisher::<TestMessage>("queue/a")
        .await
        .expect("create queue publisher");
    let mut sub_q = node_q
        .create_future_subscriber::<TestMessage>("queue/a", LagPolicy::Watermark { max_lag })
        .await
        .expect("create queue subscriber");

    tokio::time::sleep(Duration::from_millis(100)).await;

    clock.set_time(t(60)).expect("set logical time to 60");
    let _pending_q = pub_q.announce(t(10)).await.expect("announce queue t10");
    let queue_event = recv_queue(&mut sub_q).await;
    match queue_event {
        QueueEvent::Announcement { state } => {
            assert_eq!(
                state.warning,
                Some(LagWarning::LagExceeded {
                    measured: Duration::from_nanos(50),
                    clamped_to: max_lag,
                })
            );
        }
        QueueEvent::Data { .. } => panic!("expected announcement event for warning generation"),
    }

    let map_context = ContextBuilder::default()
        .with_namespace(test_ns("watermark_map_warning"))
        .with_clock(clock.clone())
        .build()
        .await
        .expect("create map context");
    let node_m = map_context
        .create_node("nm")
        .build()
        .await
        .expect("create map node");
    let pub_m = node_m
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create map publisher");
    let mut map = node_m
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>("fusion/a", LagPolicy::Watermark { max_lag })
        .await
        .expect("create map stream")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    clock.set_time(t(60)).expect("set logical time to 60 again");
    let pending_m = pub_m.announce(t(10)).await.expect("announce map t10");
    pending_m
        .publish(&TestMessage {
            data: "a10".to_owned(),
        })
        .await
        .expect("publish map t10");

    let out = recv_map1(&mut map).await;
    assert_eq!(
        out.stream_states.0.warning,
        Some(LagWarning::LagExceeded {
            measured: Duration::from_nanos(50),
            clamped_to: max_lag,
        })
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn watermark_dead_sensor_freezes_reference_and_keeps_fast_stream_temporary() {
    let clock = Clock::logical(t(0));
    let context = ContextBuilder::default()
        .with_namespace(test_ns("watermark_dead_sensor"))
        .with_clock(clock.clone())
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_a = node
        .announcing_publisher::<TestMessage>("fusion/a")
        .await
        .expect("create publisher a");
    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>(
            "fusion/a",
            LagPolicy::Watermark {
                max_lag: Duration::from_nanos(10),
            },
        )
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    clock.set_time(t(10)).expect("set logical time to 10");
    pub_a
        .announce(t(0))
        .await
        .expect("announce a0")
        .publish(&TestMessage {
            data: "a0".to_owned(),
        })
        .await
        .expect("publish a0");
    let _ = recv_map2(&mut map).await;

    for ts in [100, 200, 300] {
        pub_b
            .announce(t(ts))
            .await
            .expect("announce b")
            .publish(&TestMessage {
                data: format!("b{ts}"),
            })
            .await
            .expect("publish b");

        let out = recv_map2(&mut map).await;
        assert!(out.item.persistent.is_empty());
        assert!(keys(out.item.temporary).contains(&t(ts)));
        assert_eq!(out.stream_states.0.safe_time, Some(t(0)));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn watermark_zero_to_one_initialization_holds_before_first_message() {
    let clock = Clock::logical(t(50));
    let context = ContextBuilder::default()
        .with_namespace(test_ns("watermark_zero_to_one"))
        .with_clock(clock)
        .build()
        .await
        .expect("create context");
    let node = context.create_node("n").build().await.expect("create node");

    let pub_b = node
        .announcing_publisher::<TestMessage>("fusion/b")
        .await
        .expect("create publisher b");

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<TestMessage>(
            "fusion/a",
            LagPolicy::Watermark {
                max_lag: Duration::from_nanos(50),
            },
        )
        .await
        .expect("create stream a")
        .create_future_subscriber::<TestMessage>("fusion/b", LagPolicy::Immediate)
        .await
        .expect("create stream b")
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    pub_b
        .announce(t(100))
        .await
        .expect("announce b100")
        .publish(&TestMessage {
            data: "b100".to_owned(),
        })
        .await
        .expect("publish b100");

    let out = recv_map2(&mut map).await;
    assert!(out.item.persistent.is_empty());
    assert_eq!(keys(out.item.temporary), vec![t(100)]);
    assert_eq!(out.stream_states.0.safe_time, Some(t(0)));
    assert_eq!(
        out.stream_states.0.effective_lag,
        Some(Duration::from_nanos(50))
    );
}

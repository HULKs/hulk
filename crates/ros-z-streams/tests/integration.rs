use std::time::Duration;

use ros_z::prelude::*;
use ros_z::time::{Clock, Time};
use ros_z_streams::{CreateAnnouncingPublisher, CreateFutureMapBuilder};
use tokio::time::{sleep, timeout};

async fn setup_node(namespace: &str) -> zenoh::Result<(Node, Clock)> {
    let logical_clock = Clock::logical(Time::zero());
    let context = ContextBuilder::default()
        .with_namespace(namespace)
        .with_clock(logical_clock.clone())
        .build()
        .await?;
    let node = context.create_node("integration_test_node").build().await?;
    Ok((node, logical_clock))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_single_stream_fusion() -> zenoh::Result<()> {
    let (node, clock) = setup_node("/test_single").await?;
    let lag = Duration::from_millis(10);

    let publisher = node.announcing_publisher::<String>("stream/alpha").await?;
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<String>("stream/alpha", lag)
        .await?
        .build();

    let announcement = publisher.announce(Time::from_nanos(10)).await?;
    announcement.publish(&"message_10".to_string()).await?;

    // Advance clock past the safety boundary to release the data to persistent
    clock
        .set_time(Time::from_nanos(10) + lag + Duration::from_nanos(1))
        .unwrap();

    let item = map.recv().await?;

    assert!(item.persistent.contains_key(&Time::from_nanos(10)));
    assert!(item.temporary.is_empty());

    let tuple = item.persistent.get(&Time::from_nanos(10)).unwrap();
    assert_eq!(tuple.0, Some("message_10".to_string()));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_missing_earliest_payload_does_not_block_later_data() -> zenoh::Result<()> {
    let (node, clock) = setup_node("/test_missing_payload").await?;
    let lag = Duration::from_millis(10);

    let publisher = node.announcing_publisher::<String>("stream/alpha").await?;
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<String>("stream/alpha", lag)
        .await?
        .build();

    let _lost_payload = publisher.announce(Time::from_nanos(10)).await?;
    let later_payload = publisher.announce(Time::from_nanos(20)).await?;
    later_payload.publish(&"message_20".to_string()).await?;

    let receive = async {
        let item = timeout(Duration::from_millis(500), map.recv())
            .await
            .expect("later data should be released after the missing payload expires")?;

        assert!(item.persistent.contains_key(&Time::from_nanos(20)));

        let tuple = item.persistent.get(&Time::from_nanos(20)).unwrap();
        assert_eq!(tuple.0, Some("message_20".to_string()));

        Ok::<_, zenoh::Error>(())
    };

    let advance_clock = async {
        sleep(Duration::from_millis(50)).await;
        clock
            .set_time(Time::from_nanos(20) + lag + Duration::from_nanos(1))
            .unwrap();
    };

    let (_, result) = tokio::join!(advance_clock, receive);
    result
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_late_old_payload_does_not_get_released_after_boundary() -> zenoh::Result<()> {
    let (node, clock) = setup_node("/test_late_old_payload").await?;
    let lag = Duration::from_millis(10);

    let publisher = node.announcing_publisher::<String>("stream/alpha").await?;
    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<String>("stream/alpha", lag)
        .await?
        .build();

    let current_payload = publisher.announce(Time::from_nanos(20)).await?;
    current_payload.publish(&"message_20".to_string()).await?;

    clock
        .set_time(Time::from_nanos(20) + lag + Duration::from_nanos(1))
        .unwrap();

    let current_item = map.recv().await?;
    assert!(current_item.persistent.contains_key(&Time::from_nanos(20)));

    let late_old_payload = publisher.announce(Time::from_nanos(10)).await?;
    late_old_payload.publish(&"message_10".to_string()).await?;

    timeout(Duration::from_millis(500), map.recv())
        .await
        .expect_err("late old data should be discarded after the boundary advanced");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_two_streams_boundary_semantics() -> zenoh::Result<()> {
    let (node, clock) = setup_node("/test_boundary").await?;
    let lag = Duration::from_millis(10);

    let publisher_alpha = node.announcing_publisher::<String>("stream/alpha").await?;
    let publisher_beta = node.announcing_publisher::<String>("stream/beta").await?;

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<String>("stream/alpha", lag)
        .await?
        .create_future_subscriber::<String>("stream/beta", lag)
        .await?
        .build();

    let pending_alpha = publisher_alpha.announce(Time::from_nanos(20)).await?;
    let pending_beta = publisher_beta.announce(Time::from_nanos(10)).await?;

    pending_beta.publish(&"beta_10".to_string()).await?;

    clock
        .set_time(Time::from_nanos(10) + lag + Duration::from_nanos(1))
        .unwrap();

    let item_first = map.recv().await?;

    assert!(item_first.persistent.contains_key(&Time::from_nanos(10)));
    assert!(item_first.temporary.is_empty());

    let tuple_first = item_first.persistent.get(&Time::from_nanos(10)).unwrap();
    assert_eq!(tuple_first.0, None);
    assert_eq!(tuple_first.1, Some("beta_10".to_string()));

    pending_alpha.publish(&"alpha_20".to_string()).await?;

    clock
        .set_time(Time::from_nanos(20) + lag + Duration::from_nanos(1))
        .unwrap();

    let item_second = map.recv().await?;

    assert!(item_second.persistent.contains_key(&Time::from_nanos(20)));

    let tuple_second = item_second.persistent.get(&Time::from_nanos(20)).unwrap();
    assert_eq!(tuple_second.0, Some("alpha_20".to_string()));
    assert_eq!(tuple_second.1, None);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_silent_stream_does_not_block() -> zenoh::Result<()> {
    let (node, clock) = setup_node("/test_silent").await?;
    let lag = Duration::from_millis(10);

    let publisher_alpha = node.announcing_publisher::<String>("stream/alpha").await?;
    let _publisher_beta = node.announcing_publisher::<String>("stream/beta").await?;

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<String>("stream/alpha", lag)
        .await?
        .create_future_subscriber::<String>("stream/beta", lag)
        .await?
        .build();

    let pending_alpha = publisher_alpha.announce(Time::from_nanos(15)).await?;
    pending_alpha.publish(&"alpha_15".to_string()).await?;

    clock
        .set_time(Time::from_nanos(15) + lag + Duration::from_nanos(1))
        .unwrap();

    let item = map.recv().await?;

    assert!(item.persistent.contains_key(&Time::from_nanos(15)));
    assert!(item.temporary.is_empty());

    let tuple = item.persistent.get(&Time::from_nanos(15)).unwrap();
    assert_eq!(tuple.0, Some("alpha_15".to_string()));
    assert_eq!(tuple.1, None);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_interleaved_fusion() -> zenoh::Result<()> {
    let (node, clock) = setup_node("/test_interleaved").await?;
    let lag = Duration::from_millis(10);

    let publisher_alpha = node.announcing_publisher::<String>("stream/alpha").await?;
    let publisher_beta = node.announcing_publisher::<String>("stream/beta").await?;

    let mut map = node
        .create_future_map_builder()
        .create_future_subscriber::<String>("stream/alpha", lag)
        .await?
        .create_future_subscriber::<String>("stream/beta", lag)
        .await?
        .build();

    let pending_alpha_30 = publisher_alpha.announce(Time::from_nanos(30)).await?;
    let pending_beta_30 = publisher_beta.announce(Time::from_nanos(30)).await?;

    sleep(Duration::from_millis(50)).await;

    pending_beta_30.publish(&"beta_30".to_string()).await?;

    // Clock is still at 0. The transit boundary (-10ms) is less than the event time (30).
    // The data safely buffers in temporary instead of leaking into persistent.
    let item_first = map.recv().await?;
    assert!(item_first.persistent.is_empty());
    assert!(item_first.temporary.contains_key(&Time::from_nanos(30)));

    clock
        .set_time(Time::from_nanos(30) + Duration::from_nanos(1))
        .unwrap();

    timeout(Duration::from_millis(500), map.recv())
        .await
        .expect_err("should not terminate");

    clock
        .set_time(Time::from_nanos(30) + lag + Duration::from_nanos(1))
        .unwrap();

    pending_alpha_30.publish(&"alpha_30".to_string()).await?;

    let item_second = map.recv().await?;

    assert!(item_second.persistent.contains_key(&Time::from_nanos(30)));
    assert!(item_second.temporary.is_empty());

    let tuple = item_second.persistent.get(&Time::from_nanos(30)).unwrap();
    assert_eq!(tuple.0, Some("alpha_30".to_string()));
    assert_eq!(tuple.1, Some("beta_30".to_string()));

    Ok(())
}

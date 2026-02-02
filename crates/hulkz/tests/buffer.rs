//! Temporal buffer integration tests.

mod common;

use std::time::Duration;

use common::test_namespace;
use hulkz::{BufferBuilder, Session};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lookup_nearest() {
    let session = Session::create(test_namespace("buffer")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    let publisher = node.advertise::<i32>("data").build().await.unwrap();
    let subscriber = node.subscribe::<i32>("data").build().await.unwrap();

    let (buffer, driver) = BufferBuilder::new(subscriber).capacity(10).build();

    // Spawn the driver
    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish some values
    for i in 0..5 {
        publisher.put(&i, &session.now()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Wait for messages to be buffered
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get the latest
    let latest = buffer.get_latest().await;
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().payload, 4);

    driver_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn temporal_lookup() {
    let session = Session::create(test_namespace("temporal")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    let publisher = node.advertise::<i32>("data").build().await.unwrap();
    let subscriber = node.subscribe::<i32>("data").build().await.unwrap();

    let (buffer, driver) = BufferBuilder::new(subscriber).capacity(10).build();

    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish values with delays to get different timestamps
    publisher.put(&10, &session.now()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mid_timestamp = session.now();

    tokio::time::sleep(Duration::from_millis(50)).await;
    publisher.put(&20, &session.now()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Lookup nearest to the middle timestamp
    let nearest = buffer.lookup_nearest(&mid_timestamp).await;
    assert!(nearest.is_some());
    // Should find one of the messages (exact result depends on timing)
    let payload = nearest.unwrap().payload;
    assert!(payload == 10 || payload == 20);

    driver_handle.abort();
}

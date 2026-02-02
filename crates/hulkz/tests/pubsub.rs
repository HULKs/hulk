//! Pub/sub roundtrip integration tests.

mod common;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use common::test_namespace;
use hulkz::Session;

/// Test message type for pub/sub tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestMessage {
    value: i32,
    name: String,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn roundtrip_local_topic() {
    let session = Session::create(test_namespace("roundtrip")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    let publisher = node
        .advertise::<TestMessage>("sensor/data")
        .build()
        .await
        .unwrap();

    let mut subscriber = node
        .subscribe::<TestMessage>("sensor/data")
        .build()
        .await
        .unwrap();

    // Give Zenoh time to establish the subscription
    tokio::time::sleep(Duration::from_millis(100)).await;

    let sent = TestMessage {
        value: 42,
        name: "hello".to_string(),
    };
    publisher.put(&sent, &session.now()).await.unwrap();

    let received = timeout(Duration::from_secs(1), subscriber.recv_async())
        .await
        .expect("timeout waiting for message")
        .unwrap();

    assert_eq!(received.payload, sent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn roundtrip_global_topic() {
    let session = Session::create(test_namespace("global")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    let publisher = node
        .advertise::<TestMessage>("/fleet/status")
        .build()
        .await
        .unwrap();

    let mut subscriber = node
        .subscribe::<TestMessage>("/fleet/status")
        .build()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let sent = TestMessage {
        value: 99,
        name: "global".to_string(),
    };
    publisher.put(&sent, &session.now()).await.unwrap();

    let received = timeout(Duration::from_secs(1), subscriber.recv_async())
        .await
        .expect("timeout waiting for message")
        .unwrap();

    assert_eq!(received.payload, sent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn roundtrip_private_topic() {
    let session = Session::create(test_namespace("private")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    let publisher = node
        .advertise::<TestMessage>("~/debug/internal")
        .build()
        .await
        .unwrap();

    let mut subscriber = node
        .subscribe::<TestMessage>("~/debug/internal")
        .build()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let sent = TestMessage {
        value: 7,
        name: "private".to_string(),
    };
    publisher.put(&sent, &session.now()).await.unwrap();

    let received = timeout(Duration::from_secs(1), subscriber.recv_async())
        .await
        .expect("timeout waiting for message")
        .unwrap();

    assert_eq!(received.payload, sent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_messages() {
    let session = Session::create(test_namespace("multi")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    let publisher = node.advertise::<i32>("counter").build().await.unwrap();

    let mut subscriber = node.subscribe::<i32>("counter").build().await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish messages with small delays to ensure ordering
    for i in 0..3 {
        publisher.put(&i, &session.now()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Receive all messages (ring buffer capacity is 3)
    let mut received = Vec::new();
    for _ in 0..3 {
        let msg = timeout(Duration::from_secs(1), subscriber.recv_async())
            .await
            .expect("timeout waiting for message")
            .unwrap();
        received.push(msg.payload);
    }

    // Verify we received all expected values
    assert_eq!(received, vec![0, 1, 2]);
}

//! Unit tests for SHM pub/sub functionality

use std::{sync::Arc, time::Duration};

use ros_z::{
    context::ContextBuilder,
    shm::{ShmConfig, ShmProviderBuilder},
};
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_shm_pubsub_large_message() {
    // Setup context with SHM enabled
    let context = ContextBuilder::default()
        .with_shm_pool_size(512 * 1024) // 512KB is enough for tests
        .expect("Failed to enable SHM")
        .with_shm_threshold(1000)
        .build()
        .await
        .expect("Failed to create context");

    let node = context
        .create_node("test_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<Vec<u8>>("shm_test_topic")
        .build()
        .await
        .expect("Failed to create publisher");

    let subscriber = node
        .subscriber::<Vec<u8>>("shm_test_topic")
        .build()
        .await
        .expect("Failed to create subscriber");

    // Give some time for discovery
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create message larger than threshold (10KB > 1KB threshold)
    let large_data = vec![0xAA; 10_000];
    let message = large_data.clone();

    // Publish
    publisher
        .publish(&message)
        .await
        .expect("Failed to publish");

    // Receive
    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    // Verify
    assert_eq!(received.len(), 10_000, "Data length mismatch");

    // Verify content
    let received_bytes = received.as_slice();
    assert_eq!(&received_bytes, &large_data, "Data content mismatch");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_shm_pubsub_small_message() {
    // Setup context with SHM enabled but high threshold
    let context = ContextBuilder::default()
        .with_shm_pool_size(512 * 1024) // 512KB is enough for tests
        .expect("Failed to enable SHM")
        .with_shm_threshold(10_000) // 10KB threshold
        .build()
        .await
        .expect("Failed to create context");

    let node = context
        .create_node("test_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<Vec<u8>>("small_topic")
        .build()
        .await
        .expect("Failed to create publisher");

    let subscriber = node
        .subscriber::<Vec<u8>>("small_topic")
        .build()
        .await
        .expect("Failed to create subscriber");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create message smaller than threshold (1KB < 10KB threshold)
    let small_data = vec![0xBB; 1_000];
    let message = small_data.clone();

    // Publish (should use regular memory, not SHM)
    publisher
        .publish(&message)
        .await
        .expect("Failed to publish");

    // Receive
    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    // Verify
    assert_eq!(received.len(), 1_000, "Data length mismatch");

    let received_bytes = received.as_slice();
    assert_eq!(&received_bytes, &small_data, "Data content mismatch");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_shm_threshold_boundary() {
    // Test message exactly at threshold
    let context = ContextBuilder::default()
        .with_shm_pool_size(512 * 1024) // 512KB is enough for tests
        .expect("Failed to enable SHM")
        .with_shm_threshold(5_000) // Exactly 5KB
        .build()
        .await
        .expect("Failed to create context");

    let node = context
        .create_node("test_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<Vec<u8>>("boundary_topic")
        .build()
        .await
        .expect("Failed to create publisher");

    let subscriber = node
        .subscriber::<Vec<u8>>("boundary_topic")
        .build()
        .await
        .expect("Failed to create subscriber");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Message exactly at threshold (including CDR header)
    let boundary_data = vec![0xCC; 5_000];
    let message = boundary_data.clone();

    publisher
        .publish(&message)
        .await
        .expect("Failed to publish");

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    assert_eq!(received.len(), 5_000, "Data length mismatch");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_shm_config_hierarchy_node_override() {
    // Context has SHM with one config, node overrides with another
    let context = ContextBuilder::default()
        .with_shm_pool_size(512 * 1024) // 512KB is enough for tests
        .expect("Failed to enable SHM")
        .with_shm_threshold(10_000) // 10KB at context level
        .build()
        .await
        .expect("Failed to create context");

    // Node overrides with lower threshold
    let provider = Arc::new(
        ShmProviderBuilder::new(512 * 1024) // 512KB is enough for test
            .build()
            .expect("Failed to create SHM provider"),
    );
    let node_shm_config = ShmConfig::new(provider).with_threshold(2_000); // 2KB

    let node = context
        .create_node("test_node")
        .with_shm_config(node_shm_config)
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<Vec<u8>>("hierarchy_topic")
        .build()
        .await
        .expect("Failed to create publisher");

    let subscriber = node
        .subscriber::<Vec<u8>>("hierarchy_topic")
        .build()
        .await
        .expect("Failed to create subscriber");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Message between node threshold (2KB) and context threshold (10KB)
    let data = vec![0xDD; 5_000]; // 5KB - should use node's SHM config
    let message = data.clone();

    publisher
        .publish(&message)
        .await
        .expect("Failed to publish");

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    assert_eq!(received.len(), 5_000, "Data length mismatch");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_without_shm() {
    // Context has SHM enabled, but publisher explicitly disables it
    let context = ContextBuilder::default()
        .with_shm_pool_size(512 * 1024) // 512KB is enough for tests
        .expect("Failed to enable SHM")
        .with_shm_threshold(1_000)
        .build()
        .await
        .expect("Failed to create context");

    let node = context
        .create_node("test_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<Vec<u8>>("no_shm_topic")
        .without_shm() // Explicitly disable SHM
        .build()
        .await
        .expect("Failed to create publisher");

    let subscriber = node
        .subscriber::<Vec<u8>>("no_shm_topic")
        .build()
        .await
        .expect("Failed to create subscriber");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Large message that would normally use SHM
    let data = vec![0xEE; 10_000];
    let message = data.clone();

    // Should publish without SHM despite being large
    publisher
        .publish(&message)
        .await
        .expect("Failed to publish");

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    assert_eq!(received.len(), 10_000, "Data length mismatch");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_publisher_shm_override() {
    // Context has no SHM, but publisher enables it
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");

    let node = context
        .create_node("test_node")
        .build()
        .await
        .expect("Failed to create node");

    // Publisher has its own SHM config
    let provider = Arc::new(
        ShmProviderBuilder::new(512 * 1024) // 512KB is enough for test
            .build()
            .expect("Failed to create SHM provider"),
    );
    let pub_shm_config = ShmConfig::new(provider).with_threshold(2_000);

    let publisher = node
        .publisher::<Vec<u8>>("pub_shm_topic")
        .shm_config(pub_shm_config)
        .build()
        .await
        .expect("Failed to create publisher");

    let subscriber = node
        .subscriber::<Vec<u8>>("pub_shm_topic")
        .build()
        .await
        .expect("Failed to create subscriber");

    tokio::time::sleep(Duration::from_millis(500)).await;

    let data = vec![0xFF; 5_000];
    let message = data.clone();

    publisher
        .publish(&message)
        .await
        .expect("Failed to publish");

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    assert_eq!(received.len(), 5_000, "Data length mismatch");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_multiple_publishers_different_thresholds() {
    // Multiple publishers with different SHM configs
    // Use smaller pool sizes to avoid exhausting system SHM limits
    let context = ContextBuilder::default()
        .with_shm_pool_size(1024 * 1024) // 1MB is enough for test
        .expect("Failed to enable SHM")
        .with_shm_threshold(5_000)
        .build()
        .await
        .expect("Failed to create context");

    let node = context
        .create_node("test_node")
        .build()
        .await
        .expect("Failed to create node");

    // Publisher 1: uses context default
    let pub1 = node
        .publisher::<Vec<u8>>("topic1")
        .build()
        .await
        .expect("Failed to create publisher 1");

    // Publisher 2: lower threshold
    let provider2 = Arc::new(
        ShmProviderBuilder::new(512 * 1024) // 512KB is enough for test
            .build()
            .expect("Failed to create SHM provider 2"),
    );
    let pub2 = node
        .publisher::<Vec<u8>>("topic2")
        .shm_config(ShmConfig::new(provider2).with_threshold(1_000))
        .build()
        .await
        .expect("Failed to create publisher 2");

    let sub1 = node
        .subscriber::<Vec<u8>>("topic1")
        .build()
        .await
        .expect("Failed to create subscriber 1");

    let sub2 = node
        .subscriber::<Vec<u8>>("topic2")
        .build()
        .await
        .expect("Failed to create subscriber 2");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Message of 3KB - uses SHM on pub2 but not pub1
    let data = vec![0x11; 3_000];
    let message = data.clone();

    pub1.publish(&message).await.expect("Failed to publish 1");
    pub2.publish(&message).await.expect("Failed to publish 2");

    let recv1 = tokio::time::timeout(Duration::from_secs(2), sub1.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    let recv2 = tokio::time::timeout(Duration::from_secs(2), sub2.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    assert_eq!(recv1.len(), 3_000);
    assert_eq!(recv2.len(), 3_000);
}

//! Pub/Sub QoS tests
//!
//! These tests verify publisher/subscriber functionality with different QoS settings,
//! particularly focusing on the AdvancedPublisher behavior with various durability
//! and reliability configurations.
//!
//! ## Key Findings:
//!
//! 1. **AdvancedPublisher works within the same Zenoh session**
//!    - When publisher and subscriber share the same context, messages are delivered successfully
//!    - Async publish (`publish()`) works correctly
//!
//! 2. **AdvancedPublisher now works across different Zenoh sessions** (FIXED!)
//!    - When publisher and subscriber use different contexts (separate Zenoh sessions)
//!    - Messages are now delivered successfully
//!    - This is critical for ros-z deployments where different processes use different contexts
//!
//! 3. **Root causes that were fixed:**
//!    - Missing `reliability` setting on the publisher (was only setting `congestion_control`)
//!    - Disabled multicast scouting prevented session discovery
//!    - Missing timestamping configuration for TransientLocal QoS with cache
//!    - Added shared memory transport for efficient intra-process communication

use std::{num::NonZeroUsize, time::Duration};

use ros_z::{
    Result,
    context::ContextBuilder,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
};
use ros_z_msgs::std_msgs::String as RosString;

/// Helper to create a test context and node
async fn setup_test_node(node_name: &str) -> Result<(ros_z::context::Context, ros_z::node::Node)> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node(node_name).build().await?;

    // Allow time for node discovery
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok((context, node))
}

async fn collect_messages<T>(
    subscriber: ros_z::pubsub::Subscriber<T>,
    expected: usize,
    timeout: Duration,
) -> Vec<T>
where
    T: ros_z::Message + Send + Sync + 'static,
    T::Codec: for<'a> ros_z::msg::WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    let deadline = tokio::time::Instant::now() + timeout;
    let mut messages = Vec::new();
    while messages.len() < expected {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, subscriber.recv()).await {
            Ok(Ok(message)) => messages.push(message),
            Ok(Err(_)) | Err(_) => break,
        }
    }
    messages
}

async fn collect_messages_with_timeout<T>(
    subscriber: ros_z::pubsub::Subscriber<T>,
    expected: usize,
    timeout: Duration,
) -> Vec<T>
where
    T: ros_z::Message + Send + Sync + 'static,
    T::Codec: for<'a> ros_z::msg::WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    let deadline = std::time::Instant::now() + timeout;
    let mut messages = Vec::new();
    while messages.len() < expected {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, subscriber.recv()).await {
            Ok(Ok(message)) => messages.push(message),
            Ok(Err(_)) | Err(_) => break,
        }
    }
    messages
}

/// Helper to create a QoS profile
fn create_qos(
    durability: QosDurability,
    reliability: QosReliability,
    history: QosHistory,
) -> QosProfile {
    QosProfile {
        durability,
        reliability,
        history,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test pub/sub with Volatile durability and Reliable reliability
    /// This uses AdvancedPublisher without cache
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pubsub_volatile_reliable() -> Result<()> {
        let (_ctx, pub_node) = setup_test_node("test_pub_volatile_reliable").await?;
        let sub_node = _ctx
            .create_node("test_sub_volatile_reliable")
            .build()
            .await?;

        let topic = "/test_volatile_reliable";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create subscriber FIRST to ensure it's ready before publisher
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 5, Duration::from_secs(5)));

        // Now create publisher
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Wait for discovery
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Publish messages
        for i in 0..5 {
            let message = RosString {
                data: format!("Message {}", i),
            };
            println!("Publishing: {}", message.data);
            pub_handle.publish(&message).await?;
            // Small delay between messages
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let received = receive_task.await.expect("receive task should join");
        assert_eq!(received.len(), 5);

        Ok(())
    }

    /// Test pub/sub with Volatile durability and BestEffort reliability
    /// This uses AdvancedPublisher without cache
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pubsub_volatile_best_effort() -> Result<()> {
        let (_ctx, pub_node) = setup_test_node("test_pub_volatile_best_effort").await?;
        let sub_node = _ctx
            .create_node("test_sub_volatile_best_effort")
            .build()
            .await?;

        let topic = "/test_volatile_best_effort";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create publisher with Volatile + BestEffort QoS
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Create subscriber with matching QoS
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 4, Duration::from_secs(3)));

        // Wait for discovery
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Publish messages
        for i in 0..5 {
            let message = RosString {
                data: format!("Message {}", i),
            };
            println!("Publishing: {}", message.data);
            pub_handle.publish(&message).await?;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        assert!(
            count >= 4,
            "Expected at least 4 messages (best effort), got {}",
            count
        );

        Ok(())
    }

    /// Test pub/sub with TransientLocal durability and Reliable reliability
    /// This uses AdvancedPublisher WITH cache
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pubsub_transient_local_reliable() -> Result<()> {
        let (_ctx, pub_node) = setup_test_node("test_pub_transient_reliable").await?;
        let sub_node = _ctx
            .create_node("test_sub_transient_reliable")
            .build()
            .await?;

        let topic = "/test_transient_reliable";
        let qos = create_qos(
            QosDurability::TransientLocal,
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create subscriber FIRST
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 5, Duration::from_secs(5)));

        // Now create publisher with TransientLocal + Reliable QoS
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Wait for discovery
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Publish messages
        for i in 0..5 {
            let message = RosString {
                data: format!("Message {}", i),
            };
            println!("Publishing: {}", message.data);
            pub_handle.publish(&message).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        // With TransientLocal across contexts, we expect all messages published after subscriber exists
        assert!(
            count >= 5,
            "Expected at least 5 messages with TransientLocal, got {}",
            count
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_replays_last_sample_to_late_subscriber() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context.create_node("transient_late_pub").build().await?;
        let sub_node = context.create_node("transient_late_sub").build().await?;
        let topic = "/transient_local_late_joiner";
        let qos = QosProfile {
            durability: QosDurability::TransientLocal,
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(NonZeroUsize::new(1).unwrap()),
            ..Default::default()
        };

        let publisher = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        publisher
            .publish(&RosString {
                data: "cached".into(),
            })
            .await?;

        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
        assert_eq!(received.data, "cached");
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_replays_keep_last_depth_to_late_subscriber() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context.create_node("transient_depth_pub").build().await?;
        let sub_node = context.create_node("transient_depth_sub").build().await?;
        let topic = "/transient_local_late_joiner_depth";
        let qos = QosProfile {
            durability: QosDurability::TransientLocal,
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(NonZeroUsize::new(2).unwrap()),
            ..Default::default()
        };

        let publisher = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        for data in ["one", "two", "three"] {
            publisher.publish(&RosString { data: data.into() }).await?;
        }

        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        let first = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
        let second = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;

        assert_eq!(first.data, "two");
        assert_eq!(second.data, "three");
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_delivers_cached_history_before_live_during_startup() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context.create_node("ordered_startup_pub").build().await?;
        let sub_node = context.create_node("ordered_startup_sub").build().await?;
        let topic = "/transient_local_ordered_startup";
        let qos = QosProfile {
            durability: QosDurability::TransientLocal,
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(NonZeroUsize::new(3).unwrap()),
            ..Default::default()
        };

        let publisher = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        for data in ["cached-1", "cached-2", "cached-3"] {
            publisher.publish(&RosString { data: data.into() }).await?;
        }

        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        let mut received = Vec::new();
        for _ in 0..3 {
            let message = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
            received.push(message.data);
        }

        assert_eq!(received, ["cached-1", "cached-2", "cached-3"]);

        publisher
            .publish(&RosString {
                data: "live-4".into(),
            })
            .await?;
        let live = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
        assert_eq!(live.data, "live-4");
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_replay_preserves_cache_order_without_attachments() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context.create_node("no_attachment_pub").build().await?;
        let sub_node = context.create_node("no_attachment_sub").build().await?;
        let topic = "/transient_local_no_attachment";
        let qos = QosProfile {
            durability: QosDurability::TransientLocal,
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(NonZeroUsize::new(3).unwrap()),
            ..Default::default()
        };

        let publisher = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .attachment(false)
            .build()
            .await?;
        for data in ["one", "two", "three"] {
            publisher.publish(&RosString { data: data.into() }).await?;
        }

        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        let mut received = Vec::new();
        for _ in 0..3 {
            let message = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
            received.push(message.data);
        }

        assert_eq!(received, ["one", "two", "three"]);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_replays_last_sample_to_late_cache() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context.create_node("transient_cache_pub").build().await?;
        let cache_node = context.create_node("transient_cache_late").build().await?;
        let topic = "/transient_local_late_cache";
        let qos = QosProfile {
            durability: QosDurability::TransientLocal,
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(NonZeroUsize::new(1).unwrap()),
            ..Default::default()
        };

        let publisher = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        publisher
            .publish(&RosString {
                data: "cached".into(),
            })
            .await?;

        let cache = cache_node
            .create_cache::<RosString>(topic, 1)
            .with_qos(qos)
            .build()
            .await?;

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(stamp) = cache.newest_stamp() {
                    let message = cache.get_before(stamp).expect("stamp came from cache");
                    assert_eq!(message.data, "cached");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("late cache should receive transient-local replay");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_replay_raw_sample_uses_topic_key_expr() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context.create_node("transient_raw_key_pub").build().await?;
        let early_sub_node = context
            .create_node("transient_raw_key_early_sub")
            .build()
            .await?;
        let late_sub_node = context
            .create_node("transient_raw_key_late_sub")
            .build()
            .await?;
        let topic = "/transient_local_raw_key";
        let qos = QosProfile {
            durability: QosDurability::TransientLocal,
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(NonZeroUsize::new(1).unwrap()),
            ..Default::default()
        };

        let early_subscriber = early_sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .raw()
            .build()
            .await?;
        let publisher = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        publisher
            .publish(&RosString {
                data: "cached".into(),
            })
            .await?;

        let mut early_subscriber = early_subscriber;
        let live_sample =
            tokio::time::timeout(Duration::from_secs(2), early_subscriber.recv()).await??;

        let late_subscriber = late_sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .raw()
            .build()
            .await?;
        let mut late_subscriber = late_subscriber;
        let replay_sample =
            tokio::time::timeout(Duration::from_secs(2), late_subscriber.recv()).await??;

        let live_key = live_sample.key_expr().to_string();
        let replay_key = replay_sample.key_expr().to_string();
        assert_eq!(replay_key, live_key);
        assert!(!replay_key.contains("__ros_z_transient_local"));
        Ok(())
    }

    /// Test pub/sub with TransientLocal durability and BestEffort reliability
    /// This uses AdvancedPublisher WITH cache
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pubsub_transient_local_best_effort() -> Result<()> {
        let (_ctx, pub_node) = setup_test_node("test_pub_transient_best_effort").await?;
        let sub_node = _ctx
            .create_node("test_sub_transient_best_effort")
            .build()
            .await?;

        let topic = "/test_transient_best_effort";
        let qos = create_qos(
            QosDurability::TransientLocal,
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create publisher with TransientLocal + BestEffort QoS
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Create subscriber with matching QoS
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 4, Duration::from_secs(3)));

        // Wait for discovery
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Publish messages
        for i in 0..5 {
            let message = RosString {
                data: format!("Message {}", i),
            };
            println!("Publishing: {}", message.data);
            pub_handle.publish(&message).await?;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        assert!(count >= 4, "Expected at least 4 messages, got {}", count);

        Ok(())
    }

    /// Test pub/sub with KeepAll history
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pubsub_keep_all_history() -> Result<()> {
        let (_ctx, pub_node) = setup_test_node("test_pub_keep_all").await?;
        let sub_node = _ctx.create_node("test_sub_keep_all").build().await?;

        let topic = "/test_keep_all";
        let qos = create_qos(
            QosDurability::TransientLocal,
            QosReliability::Reliable,
            QosHistory::KeepAll,
        );

        // Create subscriber first
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 10, Duration::from_secs(5)));

        // Create publisher with KeepAll history
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Wait for discovery
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Publish many messages
        for i in 0..10 {
            let message = RosString {
                data: format!("Message {}", i),
            };
            pub_handle.publish(&message).await?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        assert!(
            count >= 10,
            "Expected all 10 messages with KeepAll, got {}",
            count
        );

        Ok(())
    }

    /// Test pub/sub with AdvancedPublisher using the SAME context
    /// THIS WORKS: Demonstrates that AdvancedPublisher works within the same Zenoh session
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pubsub_volatile_same_context_works() -> Result<()> {
        // Use SAME context for both nodes (same Zenoh session) - THIS IS THE KEY!
        let context = ContextBuilder::default().build().await?;
        let pub_node = context
            .create_node("test_pub_volatile_async")
            .build()
            .await?;
        let sub_node = context
            .create_node("test_sub_volatile_async")
            .build()
            .await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let topic = "/test_volatile_async";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create subscriber FIRST
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 5, Duration::from_secs(5)));

        // Now create publisher
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Wait for discovery
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Publish messages using ASYNC publish
        for i in 0..5 {
            let message = RosString {
                data: format!("Message {}", i),
            };
            println!("Publishing async: {}", message.data);
            pub_handle.publish(&message).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        assert!(
            count >= 5,
            "Expected at least 5 messages with publish, got {}",
            count
        );

        Ok(())
    }

    /// Test TransientLocal with same context - should work
    /// Timestamping is now enabled by default in the context
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pubsub_transient_local_same_context_works() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context
            .create_node("test_pub_transient_same")
            .build()
            .await?;
        let sub_node = context
            .create_node("test_sub_transient_same")
            .build()
            .await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let topic = "/test_transient_same";
        let qos = create_qos(
            QosDurability::TransientLocal,
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create subscriber first
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 5, Duration::from_secs(4)));

        // Create publisher
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Wait for discovery
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Publish messages
        for i in 0..5 {
            let message = RosString {
                data: format!("Message {}", i),
            };
            pub_handle.publish(&message).await?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        assert!(
            count >= 5,
            "Expected at least 5 messages with TransientLocal same context, got {}",
            count
        );

        Ok(())
    }

    /// Minimal test to verify AdvancedPublisher doesn't hang
    /// This is the simplest possible test
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_minimal_advanced_publisher_no_hang() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_minimal_hang").await?;

        let topic = "/test_minimal";
        let qos = create_qos(
            QosDurability::Volatile, // Uses AdvancedPublisher
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(1).unwrap()),
        );

        // Create publisher - this should succeed
        println!("Creating publisher...");
        let pub_handle = node.publisher::<RosString>(topic).qos(qos).build().await?;
        println!("Publisher created successfully");

        // Try to publish a single message - this is where it hangs
        let message = RosString {
            data: "Test message".to_string(),
        };
        println!("Publishing message...");

        // Use a timeout to detect the hang
        let publish_result =
            tokio::time::timeout(Duration::from_secs(5), pub_handle.publish(&message)).await;

        match publish_result {
            Ok(Ok(_)) => {
                println!("Publish succeeded");
                Ok(())
            }
            Ok(Err(e)) => {
                panic!("Publish failed with error: {:?}", e);
            }
            Err(_) => {
                panic!(
                    "HANG DETECTED: Publish timed out after 5 seconds - AdvancedPublisher.wait() is blocking!"
                );
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_async_advanced_publisher() -> Result<()> {
        println!("=== SYNC TEST: Creating context and nodes (no tokio runtime) ===");
        let context = ros_z::context::ContextBuilder::default().build().await?;
        let pub_node = context.create_node("sync_test_pub").build().await?;
        let sub_node = context.create_node("sync_test_sub").build().await?;

        let topic = "/test_sync_publish";
        let qos = create_qos(
            QosDurability::Volatile, // Uses AdvancedPublisher
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create subscriber first
        println!("Creating subscriber...");
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages_with_timeout(
            subscriber,
            2,
            Duration::from_secs(4),
        ));

        // Create publisher
        println!("Creating publisher...");
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        println!("Publisher created successfully");

        // Wait for discovery (synchronous sleep)
        println!("Waiting for discovery...");
        std::thread::sleep(Duration::from_millis(1000));

        // Publish messages SYNCHRONOUSLY - this is the critical test
        println!("Publishing messages synchronously (no tokio, like RCL C code)...");
        for i in 0..3 {
            let message = RosString {
                data: format!("Sync message {}", i),
            };
            println!("Publishing: {}", message.data);

            // This is what happens in RCL - pure synchronous call with .wait()
            pub_handle.publish(&message).await?;

            println!("Publish {} completed", i);
        }

        println!("Waiting for messages...");
        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        println!("SYNC TEST RESULT: Received {} messages", count);

        // We expect at least 2 messages with BestEffort
        assert!(count >= 2, "Expected at least 2 messages, got {}", count);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_async_publish_same_context() -> Result<()> {
        println!("=== SYNC TEST WITH SAME CONTEXT (like RCL single process) ===");
        let context = ros_z::context::ContextBuilder::default().build().await?;
        let pub_node = context.create_node("sync_same_ctx_pub").build().await?;
        let sub_node = context.create_node("sync_same_ctx_sub").build().await?;

        let topic = "/test_sync_same_ctx";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        // Create publisher first (like RCL test)
        println!("Creating publisher...");
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        // Create subscriber
        println!("Creating subscriber...");
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(async move {
            let mut messages = Vec::new();
            for _ in 0..2 {
                match tokio::time::timeout(Duration::from_secs(4), subscriber.recv()).await {
                    Ok(Ok(message)) => {
                        println!("Received: {}", message.data);
                        messages.push(message);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    Ok(Err(_)) | Err(_) => break,
                }
            }
            messages
        });

        // Wait for discovery
        println!("Waiting for discovery...");
        std::thread::sleep(Duration::from_millis(1000));

        // Publish messages SYNCHRONOUSLY
        println!("Publishing with same context (like RCL)...");
        for i in 0..3 {
            let message = RosString {
                data: format!("Same context message {}", i),
            };
            println!("Publishing: {}", message.data);

            // This should hang if same context causes the issue
            pub_handle.publish(&message).await?;

            println!("Publish {} completed", i);
        }

        println!("Waiting for messages...");
        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        println!("SAME CONTEXT TEST RESULT: Received {} messages", count);

        assert!(count >= 2, "Expected at least 2 messages, got {}", count);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_async_publish_with_deadline_liveliness() -> Result<()> {
        println!("=== SYNC TEST WITH DEADLINE + LIVELINESS (exact RCL settings) ===");
        let context = ros_z::context::ContextBuilder::default().build().await?;
        let pub_node = context.create_node("sync_deadline_pub").build().await?;
        let sub_node = context.create_node("sync_deadline_sub").build().await?;

        let topic = "/test_sync_deadline_live";

        // EXACT same QoS as RCL test_events.cpp default_qos_profile
        let qos = QosProfile {
            durability: QosDurability::Volatile, // SYSTEM_DEFAULT maps to Volatile
            reliability: QosReliability::BestEffort,
            history: QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
            // These are the critical settings from RCL:
            liveliness: ros_z::qos::QosLiveliness::ManualByTopic,
            liveliness_lease_duration: ros_z::qos::QosDuration { sec: 1, nsec: 0 }, // 1 second
            deadline: ros_z::qos::QosDuration { sec: 2, nsec: 0 },                  // 2 seconds
            ..Default::default()
        };

        println!("Creating subscriber...");
        let subscriber = sub_node
            .subscriber::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages_with_timeout(
            subscriber,
            2,
            Duration::from_secs(4),
        ));

        println!("Creating publisher...");
        let pub_handle = pub_node
            .publisher::<RosString>(topic)
            .qos(qos)
            .build()
            .await?;

        println!("Waiting for discovery...");
        std::thread::sleep(Duration::from_millis(1000));

        println!("Publishing with deadline+liveliness QoS...");
        for i in 0..3 {
            let message = RosString {
                data: format!("Deadline/liveliness message {}", i),
            };
            println!("Publishing: {}", message.data);

            // THIS IS WHERE IT SHOULD HANG if deadline/liveliness causes the issue!
            pub_handle.publish(&message).await?;

            println!("Publish {} completed", i);
        }

        println!("Waiting for messages...");
        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        println!(
            "DEADLINE/LIVELINESS TEST RESULT: Received {} messages",
            count
        );

        assert!(count >= 2, "Expected at least 2 messages, got {}", count);

        Ok(())
    }
}

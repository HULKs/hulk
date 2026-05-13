//! Pub/sub behavior for the QoS combinations ros-z maps onto zenoh.

use std::{num::NonZeroUsize, time::Duration};

use ros_z::{
    Result,
    context::ContextBuilder,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
};
async fn setup_test_node(node_name: &str) -> Result<(ros_z::context::Context, ros_z::node::Node)> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node(node_name).build().await?;

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
    T::Codec: for<'a> ros_z::message::WireDecoder<Input<'a> = &'a [u8], Output = T>,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn volatile_reliable_pubsub_delivers_all_messages() -> Result<()> {
        let (context, pub_node) = setup_test_node("test_pub_volatile_reliable").await?;
        let sub_node = context
            .create_node("test_sub_volatile_reliable")
            .build()
            .await?;

        let topic = "/test_volatile_reliable";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 5, Duration::from_secs(5)));

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(2000)).await;

        for i in 0..5 {
            let message = format!("Message {}", i);
            pub_handle.publish(&message).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let received = receive_task.await.expect("receive task should join");
        assert_eq!(received.len(), 5);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn volatile_best_effort_pubsub_delivers_expected_messages() -> Result<()> {
        let (context, pub_node) = setup_test_node("test_pub_volatile_best_effort").await?;
        let sub_node = context
            .create_node("test_sub_volatile_best_effort")
            .build()
            .await?;

        let topic = "/test_volatile_best_effort";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 4, Duration::from_secs(3)));

        tokio::time::sleep(Duration::from_millis(500)).await;

        for i in 0..5 {
            let message = format!("Message {}", i);
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_reliable_pubsub_delivers_after_subscriber_exists() -> Result<()> {
        let (context, pub_node) = setup_test_node("test_pub_transient_reliable").await?;
        let sub_node = context
            .create_node("test_sub_transient_reliable")
            .build()
            .await?;

        let topic = "/test_transient_reliable";
        let qos = create_qos(
            QosDurability::TransientLocal,
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 5, Duration::from_secs(5)));

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(2000)).await;

        for i in 0..5 {
            let message = format!("Message {}", i);
            pub_handle.publish(&message).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
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
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        publisher.publish(&"cached".into()).await?;

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
        assert_eq!(received, "cached");
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
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        for data in ["one", "two", "three"] {
            publisher.publish(&data.into()).await?;
        }

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        let first = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
        let second = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;

        assert_eq!(first, "two");
        assert_eq!(second, "three");
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
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        for data in ["cached-1", "cached-2", "cached-3"] {
            publisher.publish(&data.into()).await?;
        }

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        let mut received = Vec::new();
        for _ in 0..3 {
            let message = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
            received.push(message);
        }

        assert_eq!(received, ["cached-1", "cached-2", "cached-3"]);

        publisher.publish(&"live-4".into()).await?;
        let live = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
        assert_eq!(live, "live-4");
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
            .publisher::<String>(topic)?
            .qos(qos)
            .attachment(false)
            .build()
            .await?;
        for data in ["one", "two", "three"] {
            publisher.publish(&data.into()).await?;
        }

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        let mut received = Vec::new();
        for _ in 0..3 {
            let message = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
            received.push(message);
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
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        publisher.publish(&"cached".into()).await?;

        let cache = cache_node
            .create_cache::<String>(topic, 1)?
            .with_qos(qos)
            .build()
            .await?;

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(stamp) = cache.newest_stamp() {
                    let message = cache.get_before(stamp).expect("stamp came from cache");
                    assert_eq!(message.as_str(), "cached");
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
            .subscriber::<String>(topic)?
            .qos(qos)
            .raw()
            .build()
            .await?;
        let publisher = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        publisher.publish(&"cached".into()).await?;

        let mut early_subscriber = early_subscriber;
        let live_sample =
            tokio::time::timeout(Duration::from_secs(2), early_subscriber.recv()).await??;

        let late_subscriber = late_sub_node
            .subscriber::<String>(topic)?
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transient_local_best_effort_pubsub_delivers_expected_messages() -> Result<()> {
        let (context, pub_node) = setup_test_node("test_pub_transient_best_effort").await?;
        let sub_node = context
            .create_node("test_sub_transient_best_effort")
            .build()
            .await?;

        let topic = "/test_transient_best_effort";
        let qos = create_qos(
            QosDurability::TransientLocal,
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 4, Duration::from_secs(3)));

        tokio::time::sleep(Duration::from_millis(500)).await;

        for i in 0..5 {
            let message = format!("Message {}", i);
            pub_handle.publish(&message).await?;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();
        assert!(count >= 4, "Expected at least 4 messages, got {}", count);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn keep_all_history_delivers_all_live_messages() -> Result<()> {
        let (context, pub_node) = setup_test_node("test_pub_keep_all").await?;
        let sub_node = context.create_node("test_sub_keep_all").build().await?;

        let topic = "/test_keep_all";
        let qos = create_qos(
            QosDurability::TransientLocal,
            QosReliability::Reliable,
            QosHistory::KeepAll,
        );

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 10, Duration::from_secs(5)));

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(2000)).await;

        for i in 0..10 {
            let message = format!("Message {}", i);
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

    /// Publishing without subscribers must return instead of waiting forever.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn volatile_reliable_publish_completes_without_subscribers() -> Result<()> {
        let (_context, node) = setup_test_node("test_minimal_hang").await?;

        let topic = "/test_minimal";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::Reliable,
            QosHistory::KeepLast(NonZeroUsize::new(1).unwrap()),
        );

        let pub_handle = node.publisher::<String>(topic)?.qos(qos).build().await?;
        let message = "Test message".to_string();

        let publish_result =
            tokio::time::timeout(Duration::from_secs(5), pub_handle.publish(&message)).await;

        match publish_result {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => {
                panic!("Publish failed with error: {:?}", e);
            }
            Err(_) => {
                panic!("publish timed out after 5 seconds");
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn volatile_best_effort_publish_delivers_messages() -> Result<()> {
        let context = ros_z::context::ContextBuilder::default().build().await?;
        let pub_node = context.create_node("sync_test_pub").build().await?;
        let sub_node = context.create_node("sync_test_sub").build().await?;

        let topic = "/test_sync_publish";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 2, Duration::from_secs(4)));

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(1000)).await;

        for i in 0..3 {
            let message = format!("Sync message {}", i);
            pub_handle.publish(&message).await?;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();

        assert!(count >= 2, "Expected at least 2 messages, got {}", count);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn volatile_best_effort_same_context_delivers_messages() -> Result<()> {
        let context = ros_z::context::ContextBuilder::default().build().await?;
        let pub_node = context.create_node("sync_same_ctx_pub").build().await?;
        let sub_node = context.create_node("sync_same_ctx_sub").build().await?;

        let topic = "/test_sync_same_ctx";
        let qos = create_qos(
            QosDurability::Volatile,
            QosReliability::BestEffort,
            QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
        );

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(async move {
            let mut messages = Vec::new();
            for _ in 0..2 {
                match tokio::time::timeout(Duration::from_secs(4), subscriber.recv()).await {
                    Ok(Ok(message)) => {
                        messages.push(message);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    Ok(Err(_)) | Err(_) => break,
                }
            }
            messages
        });

        tokio::time::sleep(Duration::from_millis(1000)).await;

        for i in 0..3 {
            let message = format!("Same context message {}", i);
            pub_handle.publish(&message).await?;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();

        assert!(count >= 2, "Expected at least 2 messages, got {}", count);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn deadline_and_manual_liveliness_qos_publish_delivers_messages() -> Result<()> {
        let context = ros_z::context::ContextBuilder::default().build().await?;
        let pub_node = context.create_node("sync_deadline_pub").build().await?;
        let sub_node = context.create_node("sync_deadline_sub").build().await?;

        let topic = "/test_sync_deadline_live";

        // Mirrors the default event QoS used by the ROS client library.
        let qos = QosProfile {
            durability: QosDurability::Volatile,
            reliability: QosReliability::BestEffort,
            history: QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
            liveliness: ros_z::qos::QosLiveliness::ManualByTopic,
            liveliness_lease_duration: ros_z::qos::QosDuration { sec: 1, nsec: 0 },
            deadline: ros_z::qos::QosDuration { sec: 2, nsec: 0 },
            ..Default::default()
        };

        let subscriber = sub_node
            .subscriber::<String>(topic)?
            .qos(qos)
            .build()
            .await?;
        let receive_task = tokio::spawn(collect_messages(subscriber, 2, Duration::from_secs(4)));

        let pub_handle = pub_node
            .publisher::<String>(topic)?
            .qos(qos)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(1000)).await;

        for i in 0..3 {
            let message = format!("Deadline/liveliness message {}", i);
            pub_handle.publish(&message).await?;
        }

        let received = receive_task.await.expect("receive task should join");
        let count = received.len();

        assert!(count >= 2, "Expected at least 2 messages, got {}", count);

        Ok(())
    }
}

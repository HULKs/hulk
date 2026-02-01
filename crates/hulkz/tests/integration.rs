//! Integration tests for hulkz core features.
//!
//! These tests use in-process Zenoh sessions (no network required).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use hulkz::{BufferBuilder, Session};

/// Test message type for pub/sub tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestMessage {
    value: i32,
    name: String,
}

/// Helper to create a unique namespace for test isolation.
fn test_namespace(name: &str) -> String {
    format!("test_{}_{}", name, std::process::id())
}

mod pubsub {
    use super::*;

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

        let publisher = node
            .advertise::<i32>("counter")
            .build()
            .await
            .unwrap();

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
}

mod buffer {
    use super::*;

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
}

// Note: Graph plane liveliness tests require a public discovery API.
// This will be added in Phase 2 when we implement node discovery features.

mod graph_plane {
    use super::*;

    use hulkz::{NodeEvent, PublisherEvent, Scope, SessionEvent};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn session_has_unique_id() {
        let session1 = Session::create(test_namespace("session_id1")).await.unwrap();
        let session2 = Session::create(test_namespace("session_id2")).await.unwrap();

        // Session IDs should be in format {uuid}@{hostname}
        assert!(session1.id().contains('@'));
        assert!(session2.id().contains('@'));

        // Session IDs should be unique
        assert_ne!(session1.id(), session2.id());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn list_sessions_discovers_self() {
        let namespace = test_namespace("list_sessions");
        let session = Session::create(&namespace).await.unwrap();

        // Give liveliness time to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;

        let sessions = session.list_sessions().await.unwrap();

        // Should find our own session
        assert!(sessions.contains(&session.id().to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn list_nodes_discovers_created_nodes() {
        let namespace = test_namespace("list_nodes");
        let session = Session::create(&namespace).await.unwrap();

        // Create some nodes
        let _node1 = session.create_node("navigation").build().await.unwrap();
        let _node2 = session.create_node("vision").build().await.unwrap();

        // Give liveliness time to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;

        let nodes = session.list_nodes().await.unwrap();

        assert!(nodes.contains(&"navigation".to_string()));
        assert!(nodes.contains(&"vision".to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn list_publishers_discovers_advertised_topics() {
        let namespace = test_namespace("list_publishers");
        let session = Session::create(&namespace).await.unwrap();
        let node = session.create_node("sensor_node").build().await.unwrap();

        // Advertise some topics
        let _pub1 = node
            .advertise::<i32>("camera/front")
            .build()
            .await
            .unwrap();
        let _pub2 = node
            .advertise::<i32>("~/debug/internal")
            .build()
            .await
            .unwrap();

        // Give liveliness time to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;

        let publishers = session.list_publishers().await.unwrap();

        // Should find both publishers
        assert_eq!(publishers.len(), 2);

        // Find the local publisher
        let local_pub = publishers
            .iter()
            .find(|p| p.path == "camera/front")
            .expect("should find camera/front publisher");
        assert_eq!(local_pub.node, "sensor_node");
        assert_eq!(local_pub.scope, Scope::Local);

        // Find the private publisher
        let private_pub = publishers
            .iter()
            .find(|p| p.path == "debug/internal")
            .expect("should find debug/internal publisher");
        assert_eq!(private_pub.node, "sensor_node");
        assert_eq!(private_pub.scope, Scope::Private);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watch_nodes_receives_join_event() {
        let namespace = test_namespace("watch_nodes");
        let session = Session::create(&namespace).await.unwrap();

        // Set up the watcher before creating nodes
        let (mut watcher, driver) = session.watch_nodes().await.unwrap();
        let driver_handle = tokio::spawn(driver);

        // Give watcher time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create a node
        let _node = session.create_node("late_joiner").build().await.unwrap();

        // Should receive join event
        let event = timeout(Duration::from_secs(1), watcher.recv())
            .await
            .expect("timeout waiting for node event")
            .expect("watcher closed");

        assert!(matches!(event, NodeEvent::Joined(name) if name == "late_joiner"));

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watch_nodes_receives_leave_event() {
        let namespace = test_namespace("watch_nodes_leave");
        let session = Session::create(&namespace).await.unwrap();

        // Set up the watcher
        let (mut watcher, driver) = session.watch_nodes().await.unwrap();
        let driver_handle = tokio::spawn(driver);

        // Give watcher time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create and then drop a node
        {
            let _node = session.create_node("ephemeral").build().await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // Node is dropped here

        // Should receive join then leave event
        let mut events = Vec::new();
        for _ in 0..2 {
            if let Ok(Some(event)) = timeout(Duration::from_secs(1), watcher.recv()).await {
                events.push(event);
            }
        }

        assert!(events
            .iter()
            .any(|e| matches!(e, NodeEvent::Joined(n) if n == "ephemeral")));
        assert!(events
            .iter()
            .any(|e| matches!(e, NodeEvent::Left(n) if n == "ephemeral")));

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watch_sessions_receives_events() {
        let namespace = test_namespace("watch_sessions");
        let session = Session::create(&namespace).await.unwrap();

        // Set up the watcher
        let (mut watcher, driver) = session.watch_sessions().await.unwrap();
        let driver_handle = tokio::spawn(driver);

        // Give watcher time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create another session in the same namespace
        let session2 = Session::create(&namespace).await.unwrap();
        let expected_id = session2.id().to_string();

        // Should receive join event for the new session
        let event = timeout(Duration::from_secs(1), watcher.recv())
            .await
            .expect("timeout waiting for session event")
            .expect("watcher closed");

        assert!(matches!(event, SessionEvent::Joined(id) if id == expected_id));

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watch_publishers_receives_events() {
        let namespace = test_namespace("watch_publishers");
        let session = Session::create(&namespace).await.unwrap();
        let node = session.create_node("publisher_node").build().await.unwrap();

        // Set up the watcher
        let (mut watcher, driver) = session.watch_publishers().await.unwrap();
        let driver_handle = tokio::spawn(driver);

        // Give watcher time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Advertise a topic
        let _pub = node.advertise::<i32>("sensor/data").build().await.unwrap();

        // Should receive advertised event
        let event = timeout(Duration::from_secs(1), watcher.recv())
            .await
            .expect("timeout waiting for publisher event")
            .expect("watcher closed");

        match event {
            PublisherEvent::Advertised(info) => {
                assert_eq!(info.node, "publisher_node");
                assert_eq!(info.path, "sensor/data");
                assert_eq!(info.scope, Scope::Local);
            }
            _ => panic!("expected Advertised event"),
        }

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cross_namespace_discovery() {
        let namespace1 = test_namespace("cross_ns1");
        let namespace2 = test_namespace("cross_ns2");

        let session1 = Session::create(&namespace1).await.unwrap();
        let session2 = Session::create(&namespace2).await.unwrap();

        let _node1 = session1.create_node("node_in_ns1").build().await.unwrap();
        let _node2 = session2.create_node("node_in_ns2").build().await.unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Session1 can discover nodes in namespace2
        let nodes_in_ns2 = session1.list_nodes_in_namespace(&namespace2).await.unwrap();
        assert!(nodes_in_ns2.contains(&"node_in_ns2".to_string()));

        // Session1's own namespace shouldn't include namespace2's nodes
        let nodes_in_ns1 = session1.list_nodes().await.unwrap();
        assert!(nodes_in_ns1.contains(&"node_in_ns1".to_string()));
        assert!(!nodes_in_ns1.contains(&"node_in_ns2".to_string()));
    }
}

mod parameter {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Config {
        threshold: f64,
        count: i32,
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn local_parameter() {
        let session = Session::create(test_namespace("param")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Local parameter (robot-scoped) - no prefix
        let (param, driver) = node
            .declare_parameter::<f64>("max_speed")
            .build()
            .await
            .unwrap();

        let driver_handle = tokio::spawn(async move {
            let _ = driver.await;
        });

        let value = param.get().await;
        assert!((1.5 - *value).abs() < f64::EPSILON);

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn global_parameter() {
        let session = Session::create(test_namespace("param_global")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Global parameter (fleet-wide) - "/" prefix
        let (param, driver) = node
            .declare_parameter::<String>("/fleet_id")
            .build()
            .await
            .unwrap();

        let driver_handle = tokio::spawn(async move {
            let _ = driver.await;
        });

        let value = param.get().await;
        assert_eq!(*value, "test_fleet");

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn private_parameter() {
        let session = Session::create(test_namespace("param_private")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Private parameter (node-scoped) - "~/" prefix
        let (param, driver) = node
            .declare_parameter::<i32>("~/debug_level")
            .build()
            .await
            .unwrap();

        let driver_handle = tokio::spawn(async move {
            let _ = driver.await;
        });

        let value = param.get().await;
        assert_eq!(*value, 2);

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn private_nested_parameter() {
        let session = Session::create(test_namespace("param_nested")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Private nested parameter
        let (param, driver) = node
            .declare_parameter::<Config>("~/config")
            .build()
            .await
            .unwrap();

        let driver_handle = tokio::spawn(async move {
            let _ = driver.await;
        });

        let value = param.get().await;
        assert!((0.1 - value.threshold).abs() < f64::EPSILON);
        assert_eq!(value.count, 42);

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parameter_with_default() {
        let session = Session::create(test_namespace("param_default")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Parameter not in config, but has default
        let (param, driver) = node
            .declare_parameter::<i32>("nonexistent")
            .default(42)
            .build()
            .await
            .unwrap();

        let driver_handle = tokio::spawn(async move {
            let _ = driver.await;
        });

        let value = param.get().await;
        assert_eq!(*value, 42);

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn config_overrides_default() {
        let session = Session::create(test_namespace("param_override")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Config value should override default
        let (param, driver) = node
            .declare_parameter::<f64>("max_speed")
            .default(999.0)
            .build()
            .await
            .unwrap();

        let driver_handle = tokio::spawn(async move {
            let _ = driver.await;
        });

        // Should be config value (1.5), not default (999.0)
        let value = param.get().await;
        assert!((1.5 - *value).abs() < f64::EPSILON);

        driver_handle.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn missing_parameter_no_default_error() {
        let session = Session::create(test_namespace("param_missing")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // No config value and no default = error
        let result = node
            .declare_parameter::<f64>("nonexistent_param")
            .build()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parameter_validation_initial() {
        let session = Session::create(test_namespace("param_validate")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Validation passes
        let result = node
            .declare_parameter::<f64>("max_speed")
            .validate(|v| *v > 0.0 && *v < 10.0)
            .build()
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parameter_validation_fails() {
        let session = Session::create(test_namespace("param_validate_fail")).await.unwrap();
        let node = session.create_node("test_node").build().await.unwrap();

        // Validation fails (max_speed is 1.5, but we require > 100)
        let result = node
            .declare_parameter::<f64>("max_speed")
            .validate(|v| *v > 100.0)
            .build()
            .await;

        assert!(result.is_err());
    }
}

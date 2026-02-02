//! Graph plane discovery integration tests.
//!
//! Tests for liveliness-based discovery of sessions, nodes, publishers, and parameters.

mod common;

use std::time::Duration;

use tokio::time::timeout;

use common::test_namespace;
use hulkz::{NodeEvent, ParameterEvent, PublisherEvent, Scope, Session, SessionEvent};

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_parameters_discovers_declared_parameters() {
    let namespace = test_namespace("list_params");
    let session = Session::create(&namespace).await.unwrap();
    let node = session.create_node("param_node").build().await.unwrap();

    // Declare some parameters
    let (_param1, driver1) = node
        .declare_parameter::<f64>("max_speed")
        .default(1.5)
        .build()
        .await
        .unwrap();
    let (_param2, driver2) = node
        .declare_parameter::<i32>("~/debug_level")
        .default(0)
        .build()
        .await
        .unwrap();

    let driver_handle1 = tokio::spawn(driver1);
    let driver_handle2 = tokio::spawn(driver2);

    // Give liveliness time to propagate
    tokio::time::sleep(Duration::from_millis(100)).await;

    let parameters = session.list_parameters().await.unwrap();

    // Should find both parameters
    assert_eq!(parameters.len(), 2);

    // Find the local parameter
    let local_param = parameters
        .iter()
        .find(|p| p.path == "max_speed")
        .expect("should find max_speed parameter");
    assert_eq!(local_param.node, "param_node");
    assert_eq!(local_param.scope, Scope::Local);

    // Find the private parameter
    let private_param = parameters
        .iter()
        .find(|p| p.path == "debug_level")
        .expect("should find debug_level parameter");
    assert_eq!(private_param.node, "param_node");
    assert_eq!(private_param.scope, Scope::Private);

    driver_handle1.abort();
    driver_handle2.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watch_parameters_receives_declared_event() {
    let namespace = test_namespace("watch_params");
    let session = Session::create(&namespace).await.unwrap();
    let node = session.create_node("param_node").build().await.unwrap();

    // Set up the watcher
    let (mut watcher, watcher_driver) = session.watch_parameters().await.unwrap();
    let watcher_handle = tokio::spawn(watcher_driver);

    // Give watcher time to establish
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Declare a parameter
    let (_param, param_driver) = node
        .declare_parameter::<f64>("speed")
        .default(1.0)
        .build()
        .await
        .unwrap();
    let param_handle = tokio::spawn(param_driver);

    // Should receive declared event
    let event = timeout(Duration::from_secs(1), watcher.recv())
        .await
        .expect("timeout waiting for parameter event")
        .expect("watcher closed");

    match event {
        ParameterEvent::Declared(info) => {
            assert_eq!(info.node, "param_node");
            assert_eq!(info.path, "speed");
            assert_eq!(info.scope, Scope::Local);
        }
        _ => panic!("expected Declared event"),
    }

    watcher_handle.abort();
    param_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watch_parameters_receives_undeclared_event() {
    let namespace = test_namespace("watch_params_undecl");
    let session = Session::create(&namespace).await.unwrap();
    let node = session.create_node("param_node").build().await.unwrap();

    // Set up the watcher
    let (mut watcher, watcher_driver) = session.watch_parameters().await.unwrap();
    let watcher_handle = tokio::spawn(watcher_driver);

    // Give watcher time to establish
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Declare a parameter (in a scope so we can drop it)
    {
        let (_param, param_driver) = node
            .declare_parameter::<f64>("temp_param")
            .default(42.0)
            .build()
            .await
            .unwrap();
        let param_handle = tokio::spawn(param_driver);

        // Should receive declared event
        let event = timeout(Duration::from_secs(1), watcher.recv())
            .await
            .expect("timeout waiting for declared event")
            .expect("watcher closed");

        match event {
            ParameterEvent::Declared(info) => {
                assert_eq!(info.path, "temp_param");
            }
            _ => panic!("expected Declared event"),
        }

        // Abort the driver to undeclare the parameter
        param_handle.abort();
    }

    // Should receive undeclared event
    let event = timeout(Duration::from_secs(1), watcher.recv())
        .await
        .expect("timeout waiting for undeclared event")
        .expect("watcher closed");

    match event {
        ParameterEvent::Undeclared(info) => {
            assert_eq!(info.path, "temp_param");
            assert_eq!(info.node, "param_node");
        }
        _ => panic!("expected Undeclared event"),
    }

    watcher_handle.abort();
}

use super::encoding::decode_payload;
use super::lifecycle::session_storage_path;
use super::parameters::parameter_access_parts;
use super::run_worker;
use super::streams::{
    parse_source_path_expression, to_discovered_parameter, to_discovered_publisher,
};
use crate::model::{
    ParameterReference, ViewerConfig, WorkerCommand, WorkerEvent, WorkerEventEnvelope,
};
use hulkz::{ParameterInfo, PublisherInfo, Scope, Session};
use hulkz_stream::PlaneKind;
use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Serialize)]
struct TestOdometry {
    x: f64,
    y: f64,
    theta: f64,
}

fn unique_namespace(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{nanos}")
}

async fn recv_event_matching<F>(
    rx: &mut mpsc::Receiver<WorkerEventEnvelope>,
    timeout: Duration,
    mut predicate: F,
) -> Option<WorkerEvent>
where
    F: FnMut(&WorkerEvent) -> bool,
{
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return None;
        }
        let remaining = deadline - now;
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(envelope)) => {
                if predicate(&envelope.event) {
                    return Some(envelope.event);
                }
            }
            Ok(None) | Err(_) => return None,
        }
    }
}

async fn shutdown_worker_task(
    command_tx: mpsc::Sender<WorkerCommand>,
    cancel: CancellationToken,
    task: tokio::task::JoinHandle<()>,
) {
    let _ = command_tx.send(WorkerCommand::Shutdown).await;
    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(3), task).await;
}

#[test]
fn valid_json_decodes_to_pretty_output() {
    let payload = br#"{"x":1,"y":2}"#;
    let (json, fallback) = decode_payload("application/json", payload);

    assert!(json.is_some());
    assert!(fallback.is_none());
    assert!(json.unwrap().contains("\"x\": 1"));
}

#[test]
fn malformed_json_falls_back_to_text() {
    let payload = br#"{not-json"#;
    let (json, fallback) = decode_payload("application/json", payload);

    assert!(json.is_none());
    assert!(fallback.is_some());
}

#[test]
fn non_json_payload_uses_hex_fallback_when_not_utf8() {
    let payload = [0xff, 0x10, 0xab, 0x00];
    let (json, fallback) = decode_payload("application/cdr", &payload);

    assert!(json.is_none());
    let fallback = fallback.expect("fallback should exist");
    assert!(fallback.contains("hex preview"));
    assert!(fallback.contains("ff"));
}

#[test]
fn parses_local_path_expression() {
    let (path, node) = parse_source_path_expression("odometry").expect("must parse");
    assert_eq!(path.scope(), Scope::Local);
    assert_eq!(path.path(), "odometry");
    assert!(node.is_none());
}

#[test]
fn parses_global_path_expression() {
    let (path, node) = parse_source_path_expression("/fleet/topic").expect("must parse");
    assert_eq!(path.scope(), Scope::Global);
    assert_eq!(path.path(), "fleet/topic");
    assert!(node.is_none());
}

#[test]
fn parses_private_node_override_expression() {
    let (path, node) = parse_source_path_expression("~planner/debug").expect("must parse");
    assert_eq!(path.scope(), Scope::Private);
    assert_eq!(path.path(), "debug");
    assert_eq!(node.as_deref(), Some("planner"));
}

#[test]
fn discovered_private_publisher_uses_node_prefixed_expression() {
    let info = PublisherInfo {
        namespace: "demo".to_string(),
        node: "planner".to_string(),
        scope: Scope::Private,
        path: "debug/topic".to_string(),
    };
    let publisher = to_discovered_publisher(info);
    assert_eq!(publisher.path_expression, "~planner/debug/topic");
}

#[test]
fn discovered_global_parameter_uses_global_expression() {
    let info = ParameterInfo {
        namespace: "demo".to_string(),
        node: "config".to_string(),
        scope: Scope::Global,
        path: "fleet/id".to_string(),
    };
    let parameter = to_discovered_parameter(info);
    assert_eq!(parameter.path_expression, "/fleet/id");
}

#[test]
fn parameter_access_parts_normalize_private_path() {
    let target = ParameterReference {
        namespace: "demo".to_string(),
        node: "planner".to_string(),
        path_expression: "~planner/debug/level".to_string(),
    };
    let (namespace, node, path) = parameter_access_parts(&target).expect("must parse");
    assert_eq!(namespace, "demo");
    assert_eq!(node, "planner");
    assert_eq!(path, "~/debug/level");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_worker_binds_and_receives_live_updates() {
    let namespace = unique_namespace("viewer-live");
    let (command_tx, command_rx) = mpsc::channel(128);
    let (event_tx, mut event_rx) = mpsc::channel(512);
    let cancel = CancellationToken::new();
    let config = ViewerConfig {
        namespace: namespace.clone(),
        source_expression: "odometry".to_string(),
        storage_path: Some(session_storage_path()),
        ..ViewerConfig::default()
    };
    let worker_task = tokio::spawn(run_worker(
        config,
        command_rx,
        event_tx,
        cancel.clone(),
        None,
    ));

    let ready = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::Ready)
    })
    .await;
    assert!(ready.is_some(), "worker did not emit Ready");

    command_tx
        .send(WorkerCommand::BindStream {
            stream_id: 1,
            request: crate::model::SourceBindingRequest {
                namespace: namespace.clone(),
                plane: PlaneKind::View,
                path_expression: "odometry".to_string(),
            },
        })
        .await
        .expect("bind command send failed");

    let bound = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::SourceBound { stream_id: 1, .. })
    })
    .await;
    assert!(bound.is_some(), "worker did not emit SourceBound");

    let publisher_session = Session::create(&namespace)
        .await
        .expect("publisher session create failed");
    let publisher_node = publisher_session
        .create_node("viewer-test-publisher")
        .build()
        .await
        .expect("publisher node build failed");
    let publisher = publisher_node
        .advertise::<TestOdometry>("odometry")
        .build()
        .await
        .expect("publisher build failed");

    for i in 0..5 {
        publisher
            .put(
                &TestOdometry {
                    x: i as f64,
                    y: i as f64,
                    theta: i as f64,
                },
                &publisher_session.now(),
            )
            .await
            .expect("publish failed");
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    let records_event = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(
            event,
            WorkerEvent::StreamRecordsChunk {
                stream_id: 1,
                records,
                ..
            } if !records.is_empty()
        )
    })
    .await;
    assert!(
        records_event.is_some(),
        "worker did not emit live stream chunk"
    );

    shutdown_worker_task(command_tx, cancel, worker_task).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_discovery_snapshot_includes_sessions() {
    let namespace = unique_namespace("viewer-discovery");
    let (command_tx, command_rx) = mpsc::channel(128);
    let (event_tx, mut event_rx) = mpsc::channel(512);
    let cancel = CancellationToken::new();
    let config = ViewerConfig {
        namespace: namespace.clone(),
        source_expression: "odometry".to_string(),
        storage_path: Some(session_storage_path()),
        ..ViewerConfig::default()
    };
    let worker_task = tokio::spawn(run_worker(
        config,
        command_rx,
        event_tx,
        cancel.clone(),
        None,
    ));

    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::Ready)
    })
    .await
    .expect("worker did not emit Ready");

    command_tx
        .send(WorkerCommand::SetDiscoveryNamespace(namespace.clone()))
        .await
        .expect("set discovery namespace failed");

    let discovery = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(
            event,
            WorkerEvent::DiscoveryPatch {
                op: crate::model::DiscoveryOp::SessionUpsert(_)
            }
        ) || matches!(event, WorkerEvent::DiscoverySnapshot { sessions, .. } if !sessions.is_empty())
        })
        .await;
    assert!(
        discovery.is_some(),
        "expected discovery patch with at least one session"
    );

    shutdown_worker_task(command_tx, cancel, worker_task).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_rebind_replays_history_snapshot() {
    let namespace = unique_namespace("viewer-rebind");
    let (command_tx, command_rx) = mpsc::channel(128);
    let (event_tx, mut event_rx) = mpsc::channel(512);
    let cancel = CancellationToken::new();
    let config = ViewerConfig {
        namespace: namespace.clone(),
        source_expression: "odometry".to_string(),
        storage_path: Some(session_storage_path()),
        ..ViewerConfig::default()
    };
    let worker_task = tokio::spawn(run_worker(
        config,
        command_rx,
        event_tx,
        cancel.clone(),
        None,
    ));

    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::Ready)
    })
    .await
    .expect("worker did not emit Ready");

    command_tx
        .send(WorkerCommand::BindStream {
            stream_id: 2,
            request: crate::model::SourceBindingRequest {
                namespace: namespace.clone(),
                plane: PlaneKind::View,
                path_expression: "odometry".to_string(),
            },
        })
        .await
        .expect("bind command failed");
    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::SourceBound { stream_id: 2, .. })
    })
    .await
    .expect("source was not bound");

    let publisher_session = Session::create(&namespace)
        .await
        .expect("publisher session create failed");
    let publisher_node = publisher_session
        .create_node("viewer-rebind-publisher")
        .build()
        .await
        .expect("publisher node build failed");
    let publisher = publisher_node
        .advertise::<TestOdometry>("odometry")
        .build()
        .await
        .expect("publisher build failed");

    for i in 0..8 {
        publisher
            .put(
                &TestOdometry {
                    x: i as f64,
                    y: i as f64,
                    theta: i as f64,
                },
                &publisher_session.now(),
            )
            .await
            .expect("publish failed");
        tokio::time::sleep(Duration::from_millis(35)).await;
    }

    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(
            event,
            WorkerEvent::StreamRecordsChunk {
                stream_id: 2,
                records,
                ..
            } if !records.is_empty()
        )
    })
    .await
    .expect("no live records before rebind");

    let durable_ready = recv_event_matching(&mut event_rx, Duration::from_secs(8), |event| {
        matches!(
            event,
            WorkerEvent::StreamStats { stream_id: 2, source } if source.durable_len > 0
        )
    })
    .await;
    assert!(
        durable_ready.is_some(),
        "expected durable history before rebind"
    );

    command_tx
        .send(WorkerCommand::RemoveStream { stream_id: 2 })
        .await
        .expect("remove stream failed");
    tokio::time::sleep(Duration::from_millis(200)).await;

    command_tx
        .send(WorkerCommand::BindStream {
            stream_id: 2,
            request: crate::model::SourceBindingRequest {
                namespace: namespace.clone(),
                plane: PlaneKind::View,
                path_expression: "odometry".to_string(),
            },
        })
        .await
        .expect("rebind command failed");
    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::SourceBound { stream_id: 2, .. })
    })
    .await
    .expect("rebind did not emit SourceBound");

    let replay = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(
            event,
            WorkerEvent::StreamRecordsChunk {
                stream_id: 2,
                source: crate::model::RecordChunkSource::History,
                records,
                ..
            } if !records.is_empty()
        )
    })
    .await;
    assert!(replay.is_some(), "expected history replay after rebind");

    shutdown_worker_task(command_tx, cancel, worker_task).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn integration_ingest_toggle_pauses_and_resumes_updates() {
    let namespace = unique_namespace("viewer-ingest-toggle");
    let (command_tx, command_rx) = mpsc::channel(128);
    let (event_tx, mut event_rx) = mpsc::channel(512);
    let cancel = CancellationToken::new();
    let config = ViewerConfig {
        namespace: namespace.clone(),
        source_expression: "odometry".to_string(),
        storage_path: Some(session_storage_path()),
        ..ViewerConfig::default()
    };
    let worker_task = tokio::spawn(run_worker(
        config,
        command_rx,
        event_tx,
        cancel.clone(),
        None,
    ));

    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::Ready)
    })
    .await
    .expect("worker did not emit Ready");

    command_tx
        .send(WorkerCommand::BindStream {
            stream_id: 4,
            request: crate::model::SourceBindingRequest {
                namespace: namespace.clone(),
                plane: PlaneKind::View,
                path_expression: "odometry".to_string(),
            },
        })
        .await
        .expect("bind command failed");
    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::SourceBound { stream_id: 4, .. })
    })
    .await
    .expect("source was not bound");

    let publisher_session = Session::create(&namespace)
        .await
        .expect("publisher session create failed");
    let publisher_node = publisher_session
        .create_node("viewer-ingest-toggle-publisher")
        .build()
        .await
        .expect("publisher node build failed");
    let publisher = publisher_node
        .advertise::<TestOdometry>("odometry")
        .build()
        .await
        .expect("publisher build failed");

    publisher
        .put(
            &TestOdometry {
                x: 1.0,
                y: 1.0,
                theta: 1.0,
            },
            &publisher_session.now(),
        )
        .await
        .expect("baseline publish failed");
    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(
            event,
            WorkerEvent::StreamRecordsChunk {
                stream_id: 4,
                records,
                ..
            } if !records.is_empty()
        )
    })
    .await
    .expect("expected baseline live update");

    command_tx
        .send(WorkerCommand::SetIngestEnabled(false))
        .await
        .expect("disable ingest command failed");
    tokio::time::sleep(Duration::from_millis(300)).await;

    for i in 0..3 {
        publisher
            .put(
                &TestOdometry {
                    x: 10.0 + i as f64,
                    y: 10.0 + i as f64,
                    theta: 10.0 + i as f64,
                },
                &publisher_session.now(),
            )
            .await
            .expect("publish while disabled failed");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    let paused_records = recv_event_matching(&mut event_rx, Duration::from_millis(900), |event| {
        matches!(
            event,
            WorkerEvent::StreamRecordsChunk {
                stream_id: 4,
                records,
                ..
            } if !records.is_empty()
        )
    })
    .await;
    assert!(
        paused_records.is_none(),
        "received live updates while ingest was disabled"
    );

    command_tx
        .send(WorkerCommand::SetIngestEnabled(true))
        .await
        .expect("enable ingest command failed");
    tokio::time::sleep(Duration::from_millis(250)).await;

    for i in 0..3 {
        publisher
            .put(
                &TestOdometry {
                    x: 20.0 + i as f64,
                    y: 20.0 + i as f64,
                    theta: 20.0 + i as f64,
                },
                &publisher_session.now(),
            )
            .await
            .expect("publish after resume failed");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    let resumed_records = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(
            event,
            WorkerEvent::StreamRecordsChunk {
                stream_id: 4,
                records,
                ..
            } if !records.is_empty()
        )
    })
    .await;
    assert!(
        resumed_records.is_some(),
        "expected live updates after ingest resume"
    );

    shutdown_worker_task(command_tx, cancel, worker_task).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "manual soak run; executes a longer live ingest loop"]
async fn soak_worker_stays_healthy_under_continuous_stream() {
    let namespace = unique_namespace("viewer-soak");
    let (command_tx, command_rx) = mpsc::channel(128);
    let (event_tx, mut event_rx) = mpsc::channel(512);
    let cancel = CancellationToken::new();
    let config = ViewerConfig {
        namespace: namespace.clone(),
        source_expression: "odometry".to_string(),
        storage_path: Some(session_storage_path()),
        ..ViewerConfig::default()
    };
    let worker_task = tokio::spawn(run_worker(
        config,
        command_rx,
        event_tx,
        cancel.clone(),
        None,
    ));

    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::Ready)
    })
    .await
    .expect("worker did not emit Ready");

    command_tx
        .send(WorkerCommand::BindStream {
            stream_id: 7,
            request: crate::model::SourceBindingRequest {
                namespace: namespace.clone(),
                plane: PlaneKind::View,
                path_expression: "odometry".to_string(),
            },
        })
        .await
        .expect("bind command failed");
    let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(event, WorkerEvent::SourceBound { stream_id: 7, .. })
    })
    .await
    .expect("source was not bound");

    let publisher_session = Session::create(&namespace)
        .await
        .expect("publisher session create failed");
    let publisher_node = publisher_session
        .create_node("viewer-soak-publisher")
        .build()
        .await
        .expect("publisher node build failed");
    let publisher = publisher_node
        .advertise::<TestOdometry>("odometry")
        .build()
        .await
        .expect("publisher build failed");

    let start = tokio::time::Instant::now();
    let mut sent = 0_u64;
    while start.elapsed() < Duration::from_secs(15) {
        publisher
            .put(
                &TestOdometry {
                    x: sent as f64,
                    y: sent as f64,
                    theta: sent as f64,
                },
                &publisher_session.now(),
            )
            .await
            .expect("publish failed");
        sent = sent.saturating_add(1);
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let records_event = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
        matches!(
            event,
            WorkerEvent::StreamRecordsChunk {
                stream_id: 7,
                records,
                ..
            } if !records.is_empty()
        )
    })
    .await;
    assert!(records_event.is_some(), "expected live records during soak");
    assert!(sent > 100, "soak sent too few samples: {sent}");

    shutdown_worker_task(command_tx, cancel, worker_task).await;
}

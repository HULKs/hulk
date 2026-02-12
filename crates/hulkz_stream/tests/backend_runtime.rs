use std::{
    num::NonZeroU128,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hulkz::{Scope, ScopedPath, Session};
use hulkz_stream::{
    NamespaceBinding, OpenMode, PlaneKind, SourceHandle, SourceSpec, StreamBackend,
    StreamBackendBuilder, StreamDriver,
};
use tokio::{
    sync::broadcast::{self, error::RecvError},
    task::JoinHandle,
    time::timeout,
};
use tokio_util::sync::CancellationToken;

fn ts(nanos: u64) -> hulkz::Timestamp {
    let id: zenoh::time::TimestampId = NonZeroU128::new(1).expect("non-zero").into();
    hulkz::Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

fn to_nanos(timestamp: &hulkz::Timestamp) -> u64 {
    timestamp.get_time().as_nanos()
}

fn unique_namespace(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after epoch")
        .as_nanos();
    format!("hulkz_stream_{prefix}_{}_{}", std::process::id(), nanos)
}

fn follow_target_data_spec(path: &str) -> SourceSpec {
    SourceSpec {
        plane: PlaneKind::Data,
        path: ScopedPath::new(Scope::Local, path),
        node_override: None,
        namespace_binding: NamespaceBinding::FollowTarget,
    }
}

fn pinned_data_spec(path: &str, namespace: &str) -> SourceSpec {
    SourceSpec {
        plane: PlaneKind::Data,
        path: ScopedPath::new(Scope::Local, path),
        node_override: None,
        namespace_binding: NamespaceBinding::Pinned(namespace.to_string()),
    }
}

async fn spawn_backend(
    namespace: &str,
    storage_root: &Path,
    max_segment_bytes: u64,
) -> (StreamBackend, JoinHandle<hulkz_stream::Result<()>>) {
    let session = Session::create(namespace.to_string())
        .await
        .expect("create backend session");
    let (backend, driver): (StreamBackend, StreamDriver) = StreamBackendBuilder::new(session)
        .open_mode(OpenMode::ReadWrite)
        .storage_path(storage_root.to_path_buf())
        .max_segment_bytes(max_segment_bytes)
        .build()
        .await
        .expect("build backend");
    let driver_task = tokio::spawn(driver);
    (backend, driver_task)
}

async fn spawn_backend_with_writer_queue(
    namespace: &str,
    storage_root: &Path,
    max_segment_bytes: u64,
    write_queue_capacity: usize,
) -> (StreamBackend, JoinHandle<hulkz_stream::Result<()>>) {
    let session = Session::create(namespace.to_string())
        .await
        .expect("create backend session");
    let (backend, driver): (StreamBackend, StreamDriver) = StreamBackendBuilder::new(session)
        .open_mode(OpenMode::ReadWrite)
        .storage_path(storage_root.to_path_buf())
        .max_segment_bytes(max_segment_bytes)
        .write_queue_capacity(write_queue_capacity)
        .build()
        .await
        .expect("build backend");
    let driver_task = tokio::spawn(driver);
    (backend, driver_task)
}

async fn wait_for_effective_namespace(
    source: &SourceHandle,
    expected_namespace: &str,
    timeout_after: Duration,
) -> hulkz_stream::StreamRecord {
    let start = tokio::time::Instant::now();
    loop {
        if let Some(record) = source.latest().await.expect("latest query should work") {
            if record.effective_namespace.as_deref() == Some(expected_namespace) {
                return record;
            }
        }
        assert!(
            start.elapsed() < timeout_after,
            "timed out waiting for namespace {expected_namespace}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn wait_for_live_record(
    updates: &mut broadcast::Receiver<hulkz_stream::StreamRecord>,
    timeout_after: Duration,
) -> hulkz_stream::StreamRecord {
    let started = tokio::time::Instant::now();
    loop {
        let remaining = timeout_after.saturating_sub(started.elapsed());
        assert!(
            !remaining.is_zero(),
            "timed out waiting for live record after {timeout_after:?}"
        );

        match timeout(remaining, updates.recv()).await {
            Ok(Ok(record)) => return record,
            Ok(Err(RecvError::Lagged(_))) => continue,
            Ok(Err(RecvError::Closed)) => panic!("live update channel closed"),
            Err(_) => panic!("timed out waiting for live record after {timeout_after:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn backend_deduplicates_equivalent_sources() {
    let namespace = unique_namespace("dedup");
    let temp = tempfile::tempdir().expect("temp dir");
    let (backend, driver_task) = spawn_backend(&namespace, temp.path(), 1024 * 1024).await;

    let spec = follow_target_data_spec("sensor/data");
    let handle_a = backend
        .source(spec.clone())
        .await
        .expect("acquire source A");
    let handle_b = backend.source(spec).await.expect("acquire source B");

    tokio::time::sleep(Duration::from_millis(100)).await;
    let active = backend.stats_snapshot();
    assert_eq!(active.active_sources, 1);
    assert_eq!(active.active_subscribers, 1);

    drop(handle_a);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let after_one_drop = backend.stats_snapshot();
    assert_eq!(after_one_drop.active_sources, 1);
    assert_eq!(after_one_drop.active_subscribers, 1);

    drop(handle_b);
    tokio::time::sleep(Duration::from_millis(150)).await;
    let after_all_drop = backend.stats_snapshot();
    assert_eq!(after_all_drop.active_sources, 0);
    assert_eq!(after_all_drop.active_subscribers, 0);

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn follow_target_namespace_switch_keeps_handle_valid() {
    let namespace_a = unique_namespace("switch_a");
    let namespace_b = unique_namespace("switch_b");
    let temp = tempfile::tempdir().expect("temp dir");

    let (backend, driver_task) = spawn_backend(&namespace_a, temp.path(), 1024 * 1024).await;
    let source = backend
        .source(follow_target_data_spec("sensor/data"))
        .await
        .expect("acquire source");

    let pub_session_a = Session::create(namespace_a.clone())
        .await
        .expect("publisher session A");
    let pub_node_a = pub_session_a
        .create_node("publisher_a")
        .build()
        .await
        .expect("publisher node A");
    let publisher_a = pub_node_a
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher A");

    tokio::time::sleep(Duration::from_millis(150)).await;
    publisher_a
        .put(&1, &pub_session_a.now())
        .await
        .expect("publish in namespace A");

    let first = wait_for_effective_namespace(&source, &namespace_a, Duration::from_secs(3)).await;

    backend
        .set_target_namespace(namespace_b.clone())
        .await
        .expect("set target namespace");

    let pub_session_b = Session::create(namespace_b.clone())
        .await
        .expect("publisher session B");
    let pub_node_b = pub_session_b
        .create_node("publisher_b")
        .build()
        .await
        .expect("publisher node B");
    let publisher_b = pub_node_b
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher B");

    tokio::time::sleep(Duration::from_millis(200)).await;
    for value in [2_i32, 3, 4] {
        publisher_b
            .put(&value, &pub_session_b.now())
            .await
            .expect("publish in namespace B");
    }

    let second = wait_for_effective_namespace(&source, &namespace_b, Duration::from_secs(3)).await;
    let historical = source
        .before_or_equal(first.timestamp)
        .await
        .expect("historical query should work")
        .expect("historical record exists");

    assert_eq!(
        historical.effective_namespace.as_deref(),
        Some(namespace_a.as_str())
    );
    assert_eq!(
        second.effective_namespace.as_deref(),
        Some(namespace_b.as_str())
    );

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_query_during_ingest_is_stable() {
    let namespace = unique_namespace("concurrent");
    let temp = tempfile::tempdir().expect("temp dir");

    let (backend, driver_task) = spawn_backend(&namespace, temp.path(), 1024 * 1024).await;
    let source = backend
        .source(pinned_data_spec("sensor/data", &namespace))
        .await
        .expect("acquire source");
    let source_for_queries = source.clone();

    let pub_session = Session::create(namespace.clone())
        .await
        .expect("publisher session");
    let pub_node = pub_session
        .create_node("publisher")
        .build()
        .await
        .expect("publisher node");
    let publisher = pub_node
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let publish_task = tokio::spawn(async move {
        for value in 0..100_i32 {
            publisher
                .put(&value, &pub_session.now())
                .await
                .expect("publish message");
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    });

    let query_task = tokio::spawn(async move {
        for _ in 0..200 {
            let latest = source_for_queries.latest().await.expect("latest query");
            if let Some(record) = latest {
                let _ = source_for_queries
                    .before_or_equal(record.timestamp)
                    .await
                    .expect("before_or_equal query");
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });

    publish_task.await.expect("join publish task");
    query_task.await.expect("join query task");

    let latest = source.latest().await.expect("latest query");
    assert!(latest.is_some(), "expected at least one ingested record");

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_updates_are_delivered_without_polling_queries() {
    let namespace = unique_namespace("live_updates");
    let temp = tempfile::tempdir().expect("temp dir");

    let (backend, driver_task) = spawn_backend(&namespace, temp.path(), 1024 * 1024).await;
    let spec = pinned_data_spec("sensor/data", &namespace);
    let source_a = backend
        .source(spec.clone())
        .await
        .expect("acquire source A");
    let source_b = backend.source(spec).await.expect("acquire source B");

    let mut updates_a = source_a.live_updates();
    let mut updates_b = source_b.live_updates();

    let pub_session = Session::create(namespace.clone())
        .await
        .expect("publisher session");
    let pub_node = pub_session
        .create_node("publisher")
        .build()
        .await
        .expect("publisher node");
    let publisher = pub_node
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher");

    tokio::time::sleep(Duration::from_millis(150)).await;
    for value in 0..4_i32 {
        publisher
            .put(&value, &pub_session.now())
            .await
            .expect("publish message");
    }

    let record_a = wait_for_live_record(&mut updates_a, Duration::from_secs(3)).await;
    let record_b = wait_for_live_record(&mut updates_b, Duration::from_secs(3)).await;

    assert_eq!(record_a.source.plane, PlaneKind::Data);
    assert_eq!(record_b.source.plane, PlaneKind::Data);
    assert_eq!(
        record_a.effective_namespace.as_deref(),
        Some(namespace.as_str())
    );
    assert_eq!(
        record_b.effective_namespace.as_deref(),
        Some(namespace.as_str())
    );
    assert!(to_nanos(&record_a.timestamp) > 0);
    assert!(to_nanos(&record_b.timestamp) > 0);

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ingest_frontier_can_lead_durable_frontier() {
    let namespace = unique_namespace("lag");
    let temp = tempfile::tempdir().expect("temp dir");

    // Small segments amplify durable writer overhead and make ingest-vs-durable lag observable.
    let (backend, driver_task) = spawn_backend(&namespace, temp.path(), 1).await;
    let source = backend
        .source(pinned_data_spec("sensor/data", &namespace))
        .await
        .expect("acquire source");
    let mut stats_rx = source.stats_watch();

    let pub_session = Session::create(namespace.clone())
        .await
        .expect("publisher session");
    let pub_node = pub_session
        .create_node("publisher")
        .build()
        .await
        .expect("publisher node");
    let publisher = pub_node
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let publish_task = tokio::spawn(async move {
        for value in 0..400_i32 {
            publisher
                .put(&value, &pub_session.now())
                .await
                .expect("publish message");
        }
    });

    let mut saw_lag = false;
    let started = tokio::time::Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        let changed = timeout(Duration::from_millis(100), stats_rx.changed()).await;
        if changed.is_ok() && changed.expect("timeout result").is_ok() {
            let snapshot = stats_rx.borrow().clone();
            if let Some(ingest_frontier) = snapshot.ingest_frontier {
                match snapshot.durable_frontier {
                    None => {
                        saw_lag = true;
                        break;
                    }
                    Some(durable_frontier) if ingest_frontier > durable_frontier => {
                        saw_lag = true;
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    publish_task.await.expect("join publish task");
    assert!(
        saw_lag,
        "expected ingest frontier to lead durable frontier at least once"
    );

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ingest_gating_pauses_and_resumes_source_updates() {
    let namespace = unique_namespace("gate");
    let temp = tempfile::tempdir().expect("temp dir");
    let (backend, driver_task) = spawn_backend(&namespace, temp.path(), 1024 * 1024).await;
    let source = backend
        .source(pinned_data_spec("sensor/data", &namespace))
        .await
        .expect("acquire source");

    let pub_session = Session::create(namespace.clone())
        .await
        .expect("publisher session");
    let pub_node = pub_session
        .create_node("publisher")
        .build()
        .await
        .expect("publisher node");
    let publisher = pub_node
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher");

    tokio::time::sleep(Duration::from_millis(150)).await;
    backend
        .set_ingest_enabled(false)
        .await
        .expect("disable ingest");
    publisher
        .put(&1, &pub_session.now())
        .await
        .expect("publish while disabled");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        source.latest().await.expect("latest query").is_none(),
        "ingest should pause while disabled"
    );

    backend
        .set_ingest_enabled(true)
        .await
        .expect("enable ingest");
    let started = tokio::time::Instant::now();
    let mut resumed = false;
    while started.elapsed() < Duration::from_secs(3) {
        publisher
            .put(&2, &pub_session.now())
            .await
            .expect("publish while enabled");
        tokio::time::sleep(Duration::from_millis(40)).await;
        if let Some(record) = source.latest().await.expect("latest query") {
            if record.effective_namespace.as_deref() == Some(namespace.as_str()) {
                resumed = true;
                break;
            }
        }
    }
    assert!(resumed, "ingest should resume when re-enabled");

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writer_backpressure_metrics_are_observable() {
    let namespace = unique_namespace("backpressure");
    let temp = tempfile::tempdir().expect("temp dir");
    let (backend, driver_task) =
        spawn_backend_with_writer_queue(&namespace, temp.path(), 1, 1).await;
    let source = backend
        .source(pinned_data_spec("sensor/data", &namespace))
        .await
        .expect("acquire source");

    let pub_session = Session::create(namespace.clone())
        .await
        .expect("publisher session");
    let pub_node = pub_session
        .create_node("publisher")
        .build()
        .await
        .expect("publisher node");
    let publisher = pub_node
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher");

    tokio::time::sleep(Duration::from_millis(150)).await;
    for value in 0..256_i32 {
        publisher
            .put(&value, &pub_session.now())
            .await
            .expect("publish message");
    }

    // Wait until writer drains enough for stats to settle.
    tokio::time::sleep(Duration::from_millis(400)).await;
    let stats = backend.stats_snapshot();
    assert!(
        stats.writer_queue_high_watermark >= 1,
        "queue should reach non-zero depth"
    );
    assert!(
        stats.writer_backpressure_events > 0,
        "full queue should produce backpressure events"
    );

    assert!(source.latest().await.expect("latest query").is_some());

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn source_timeline_is_available_during_active_ingest() {
    let namespace = unique_namespace("timeline");
    let temp = tempfile::tempdir().expect("temp dir");
    let (backend, driver_task) = spawn_backend(&namespace, temp.path(), 1024 * 1024).await;
    let source = backend
        .source(pinned_data_spec("sensor/data", &namespace))
        .await
        .expect("acquire source");

    let pub_session = Session::create(namespace.clone())
        .await
        .expect("publisher session");
    let pub_node = pub_session
        .create_node("publisher")
        .build()
        .await
        .expect("publisher node");
    let publisher = pub_node
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher");

    tokio::time::sleep(Duration::from_millis(150)).await;
    for value in 0..16_i32 {
        publisher
            .put(&value, &pub_session.now())
            .await
            .expect("publish");
    }

    tokio::time::sleep(Duration::from_millis(150)).await;
    let now = pub_session.now();
    let end_ns = to_nanos(&now);
    let start_ns = end_ns.saturating_sub(Duration::from_secs(5).as_nanos() as u64);
    let timeline = source
        .timeline(
            ts(start_ns),
            ts(end_ns.saturating_add(Duration::from_secs(1).as_nanos() as u64)),
            8,
        )
        .await
        .expect("timeline query");
    assert_eq!(timeline.buckets.len(), 8);
    assert!(
        timeline
            .buckets
            .iter()
            .any(|bucket| bucket.message_count > 0),
        "expected at least one non-empty bucket"
    );

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prefetch_range_cancellable_respects_cancel_token() {
    let namespace = unique_namespace("prefetch_cancel");
    let temp = tempfile::tempdir().expect("temp dir");
    let (backend, driver_task) = spawn_backend(&namespace, temp.path(), 1).await;
    let source = backend
        .source(pinned_data_spec("sensor/data", &namespace))
        .await
        .expect("acquire source");

    let pub_session = Session::create(namespace.clone())
        .await
        .expect("publisher session");
    let pub_node = pub_session
        .create_node("publisher")
        .build()
        .await
        .expect("publisher node");
    let publisher = pub_node
        .advertise::<i32>("sensor/data")
        .build()
        .await
        .expect("publisher");

    tokio::time::sleep(Duration::from_millis(150)).await;
    for value in 0..80_i32 {
        publisher
            .put(&value, &pub_session.now())
            .await
            .expect("publish");
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    let token = CancellationToken::new();
    token.cancel();
    let now = pub_session.now();
    let now_ns = to_nanos(&now);
    let inserted = source
        .prefetch_range_cancellable(
            ts(now_ns.saturating_sub(Duration::from_secs(20).as_nanos() as u64)),
            now,
            128,
            &token,
        )
        .await
        .expect("prefetch call");
    assert_eq!(inserted, 0, "cancelled prefetch should insert no chunks");

    backend.shutdown().await.expect("shutdown backend");
    driver_task.await.expect("join driver").expect("driver ok");
}

use std::{collections::BTreeMap, num::NonZeroU128, sync::Arc, time::Duration};

use hulkz::{Scope, ScopedPath};
use hulkz_stream::{
    storage::Storage, NamespaceBinding, OpenMode, PlaneKind, SourceSpec, StreamRecord,
};
use mcap::{records::MessageHeader, write::WriteOptions};
use tokio::time::timeout;

fn ts(nanos: u64) -> hulkz::Timestamp {
    let id: zenoh::time::TimestampId = NonZeroU128::new(1).expect("non-zero").into();
    hulkz::Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

fn spec() -> SourceSpec {
    SourceSpec {
        plane: PlaneKind::Data,
        path: ScopedPath::new(Scope::Local, "camera/front"),
        node_override: None,
        namespace_binding: NamespaceBinding::Pinned("robot".to_string()),
    }
}

fn record(nanos: u64, payload: &'static [u8]) -> StreamRecord {
    StreamRecord {
        source: spec(),
        effective_namespace: Some("robot".to_string()),
        timestamp: ts(nanos),
        encoding: zenoh::bytes::Encoding::APPLICATION_CDR,
        payload: Arc::from(payload),
    }
}

#[tokio::test]
async fn storage_query_operators_roundtrip() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenMode::ReadWrite, temp.path().to_path_buf(), 1024)
        .await
        .unwrap();

    storage.append(record(100, b"a")).await.unwrap();
    storage.append(record(200, b"b")).await.unwrap();
    storage.append(record(300, b"c")).await.unwrap();

    let latest = storage.query_latest(&spec()).await.unwrap().unwrap();
    assert_eq!(latest.timestamp, ts(300));

    let before = storage
        .query_before_or_equal(&spec(), ts(250))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before.timestamp, ts(200));

    let nearest = storage
        .query_nearest(&spec(), ts(260))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(nearest.timestamp, ts(300));

    let range = storage
        .query_range_inclusive(&spec(), ts(100), ts(220))
        .await
        .unwrap();
    assert_eq!(range.len(), 2);

    storage.shutdown().await.unwrap();

    let readonly = Storage::open(OpenMode::ReadOnly, temp.path().to_path_buf(), 1024)
        .await
        .unwrap();
    let latest_after_reopen = readonly.query_latest(&spec()).await.unwrap().unwrap();
    assert_eq!(latest_after_reopen.timestamp, ts(300));
}

#[tokio::test]
async fn indexed_queries_remain_correct_after_segment_roll_and_restart() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenMode::ReadWrite, temp.path().to_path_buf(), 1)
        .await
        .unwrap();

    for ts in [10_u64, 20, 30, 40, 50] {
        storage.append(record(ts, b"x")).await.unwrap();
    }

    storage.shutdown().await.unwrap();

    let reopened = Storage::open(OpenMode::ReadOnly, temp.path().to_path_buf(), 1)
        .await
        .unwrap();

    let latest = reopened.query_latest(&spec()).await.unwrap().unwrap();
    assert_eq!(latest.timestamp, ts(50));

    let before = reopened
        .query_before_or_equal(&spec(), ts(34))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before.timestamp, ts(30));

    let nearest = reopened
        .query_nearest(&spec(), ts(36))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(nearest.timestamp, ts(40));

    let range = reopened
        .query_range_inclusive(&spec(), ts(20), ts(40))
        .await
        .unwrap();
    assert_eq!(range.len(), 3);
}

#[tokio::test]
async fn reopen_after_ungraceful_drop_recovers_latest_data() {
    let temp = tempfile::tempdir().unwrap();
    {
        let storage = Storage::open(OpenMode::ReadWrite, temp.path().to_path_buf(), 1024)
            .await
            .unwrap();
        storage.append(record(10, b"a")).await.unwrap();
        storage.append(record(20, b"b")).await.unwrap();
        // Intentionally skip shutdown to exercise recovery path on next open.
    }

    let reopened = Storage::open(OpenMode::ReadOnly, temp.path().to_path_buf(), 1024)
        .await
        .unwrap();
    let latest = reopened.query_latest(&spec()).await.unwrap().unwrap();
    assert_eq!(latest.timestamp, ts(20));
}

#[tokio::test]
async fn indexed_query_smoke_with_many_segments() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenMode::ReadWrite, temp.path().to_path_buf(), 1)
        .await
        .unwrap();

    for ts in 0..400_u64 {
        storage.append(record(ts, b"x")).await.unwrap();
    }
    storage.shutdown().await.unwrap();

    let reopened = Storage::open(OpenMode::ReadOnly, temp.path().to_path_buf(), 1)
        .await
        .unwrap();

    timeout(Duration::from_secs(2), async {
        for query in [15_u64, 120, 255, 399] {
            let nearest = reopened
                .query_nearest(&spec(), ts(query))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(nearest.timestamp, ts(query));
        }

        let range = reopened
            .query_range_inclusive(&spec(), ts(100), ts(199))
            .await
            .unwrap();
        assert_eq!(range.len(), 100);
    })
    .await
    .expect("indexed queries should finish promptly on segmented recordings");
}

#[tokio::test]
async fn load_external_mcap_best_effort() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("external.mcap");

    {
        let mut writer = WriteOptions::default()
            .create(std::io::BufWriter::new(
                std::fs::File::create(&file).unwrap(),
            ))
            .unwrap();

        let channel = writer
            .add_channel(
                0,
                "hulkz/data/local/robot/camera/front",
                "application/cdr",
                &BTreeMap::new(),
            )
            .unwrap();

        writer
            .write_to_known_channel(
                &MessageHeader {
                    channel_id: channel,
                    sequence: 0,
                    log_time: 42,
                    publish_time: 42,
                },
                b"payload",
            )
            .unwrap();
        writer.finish().unwrap();
    }

    let storage = Storage::open(OpenMode::ReadOnly, file.clone(), 1024)
        .await
        .unwrap();

    let records = storage.query_range_all(ts(0), ts(100)).await.unwrap();

    assert_eq!(records.len(), 1);
    let rec = &records[0];
    assert_eq!(rec.source.plane, PlaneKind::Data);
    assert_eq!(rec.source.path.scope(), Scope::Local);
    assert_eq!(rec.source.path.path(), "camera/front");
}

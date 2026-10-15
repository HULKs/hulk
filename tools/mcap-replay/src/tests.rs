use super::*;
use ros_z::{Message, message::WireEncoder, time::Time};
use ros_z_debug::query::{
    plan::{
        self, Evaluation, Frame, InputSpec, QueryPlan, Sample, Selection, Source, State,
        TopicSource, Value,
    },
    *,
};
use std::{borrow::Cow, collections::BTreeMap, num::NonZeroUsize, sync::Arc, time::Duration};

fn fixture(message_indexes: bool) -> tempfile::NamedTempFile {
    fixture_records(
        message_indexes,
        true,
        &[
            (0, 10, 110, 210),
            (0, 20, 120, 220),
            (0, 30, 130, 230),
            (1, 20, 520, 920),
            (1, 10, 510, 930),
        ],
    )
}
fn channels<T: Message>(topics: &[&str]) -> Vec<Arc<mcap::Channel<'static>>> {
    let bundle = T::schema();
    let hash = ros_z_schema::compute_hash(&bundle).unwrap();
    let schema = Arc::new(mcap::Schema {
        id: 1,
        name: T::type_name(),
        encoding: "ros-z-schema-json".into(),
        data: Cow::Owned(serde_json::to_vec(&bundle).unwrap()),
    });
    topics
        .iter()
        .enumerate()
        .map(|(id, topic)| {
            Arc::new(mcap::Channel {
                id: id as u16 + 1,
                topic: (*topic).into(),
                schema: Some(schema.clone()),
                message_encoding: "ros-z-cdr".into(),
                metadata: BTreeMap::from([
                    ("ros_z.type_name".into(), T::type_name()),
                    ("ros_z.schema_hash".into(), hash.to_hash_string()),
                ]),
            })
        })
        .collect()
}

pub(crate) fn fixture_records(
    message_indexes: bool,
    flush_each: bool,
    records: &[(usize, u64, u64, u64)],
) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    let channels = channels::<Time>(&["images", "detections"]);
    let mut writer = mcap::WriteOptions::new()
        .chunk_size(None)
        .emit_message_indexes(message_indexes)
        .create(file.reopen().unwrap())
        .unwrap();
    for (sequence, &(channel, stamp, source, log)) in records.iter().enumerate() {
        writer
            .write(&mcap::Message {
                channel: channels[channel].clone(),
                sequence: sequence as u32,
                log_time: log,
                publish_time: source,
                data: Cow::Owned(
                    <Time as Message>::Codec::serialize(
                        &Time::zero().saturating_add(Duration::from_nanos(stamp)),
                    )
                    .unwrap(),
                ),
            })
            .unwrap();
        if flush_each {
            writer.flush().unwrap();
        }
    }
    writer.finish().unwrap();
    file
}
fn open(file: &std::path::Path) -> QueryRecording {
    QueryRecording::new(
        Recording::open(file, "/replay").unwrap(),
        "fixture".into(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}
fn projected(topic: &str) -> TopicSource {
    TopicSource::new(
        format!("/replay/{topic}"),
        TimeProjection::field("", TimeEncoding::RosZTime).unwrap(),
    )
}
fn image_plan(selection: Selection, matching: Selection) -> QueryPlan {
    let input = InputSpec {
        source: projected("detections").into(),
        selection: matching,
    };
    Source::from(projected("images"))
        .select(selection, plan::cursor())
        .requiring([input.clone()])
        .attach([input])
        .plan()
}
/// Collect the actual progressive plan path; never prebuild indexes in tests.
fn finish(recording: &mut QueryRecording, query: &QueryPlan, time: Time) -> Evaluation {
    let mut result = None;
    recording
        .query_plan(query, time, |evaluation| {
            result = Some(evaluation);
            true
        })
        .unwrap();
    let result = result.unwrap();
    assert!(result.settled());
    result
}
fn frames(result: &Evaluation) -> &[Frame] {
    let State::Ready(Value::Frames(frames)) = &result.state else {
        panic!("expected frames: {result:?}")
    };
    frames
}
fn records(result: &Evaluation) -> &[Sample] {
    let State::Ready(Value::Records(records)) = &result.state else {
        panic!("expected records: {result:?}")
    };
    records
}
fn input(frame: &Frame, index: usize) -> &[Sample] {
    let State::Ready(records) = &frame.inputs[index] else {
        panic!("unresolved input")
    };
    records
}

#[test]
fn publication_ids_are_scoped_to_recording_instances_and_stable_across_queries() {
    let file = fixture(true);
    let mut first = open(file.path());
    let mut second = open(file.path());
    let plan = image_plan(Selection::latest(), Selection::Exact);
    let query = |recording: &mut QueryRecording, time| {
        let result = finish(recording, &plan, Time::from_nanos(time));
        let Response::PlanData(result) =
            remote::decode(&encode(&Response::PlanData(result)).unwrap()).unwrap()
        else {
            panic!("plan response")
        };
        let publisher = *uuid::Uuid::parse_str(&recording.info.instance)
            .unwrap()
            .as_bytes();
        let frame = &frames(&result)[0];
        std::iter::once(&frame.primary)
            .chain(input(frame, 0))
            .map(|sample| {
                assert_eq!(
                    sample.record.publication,
                    Some((publisher, sample.record.id as i64))
                );
                let decoded = decode_record::<Time>(&sample.record, &sample.topic).unwrap();
                (sample.record.id, decoded.publication_id, decoded.value)
            })
            .collect::<Vec<_>>()
    };
    let original = query(&mut first, 20);
    let reopened = query(&mut second, 20);
    assert_eq!(original.len(), 2);
    for (a, b) in original.iter().zip(reopened.iter()) {
        assert_eq!(a.0, b.0);
        assert_eq!(a.2, b.2);
        assert_ne!(a.1, b.1);
    }
    assert_ne!(original[0].1, original[1].1);
    assert_ne!(original, query(&mut first, 10));
    assert_eq!(original, query(&mut first, 20));
    assert_eq!(reopened, query(&mut second, 20));
}

#[test]
fn progressive_indexes_preserve_file_order_ties_and_reuse_loaded_chunks() {
    for message_indexes in [false, true] {
        let file = fixture_records(
            message_indexes,
            false,
            &[
                (0, 20, 220, 400),
                (0, 10, 110, 300),
                (1, 10, 800, 100),
                (1, 20, 900, 200),
                (0, 20, 225, 50),
                (1, 20, 905, 25),
            ],
        );
        let mut recording = open(file.path());
        let plan = image_plan(Selection::latest(), Selection::Exact);
        let result = finish(&mut recording, &plan, Time::from_nanos(20));
        let frame = &frames(&result)[0];
        assert_eq!(frame.primary.record.source_time, Time::from_nanos(225));
        assert_eq!(input(frame, 0)[0].record.source_time, Time::from_nanos(905));
        assert_eq!(
            result.bounds,
            Some((Time::from_nanos(10), Time::from_nanos(20)))
        );
        assert_eq!(
            recording.decoded_chunks(),
            3,
            "two progressive index scans, one payload load"
        );
        assert_eq!(recording.index_stats(), (2, 6 * 16));
        finish(&mut recording, &plan, Time::from_nanos(10));
        assert_eq!(
            recording.decoded_chunks(),
            3,
            "both indexes and payload chunk reused"
        );
        assert_eq!(recording.projected_messages, 6);
    }
}

#[test]
fn index_budget_evicts_lru_and_pins_active_plan_sources() {
    let file = fixture(true);
    let raw = TopicSource::new("/replay/images", TimeProjection::SourceTime);
    let images = projected("images");
    let detections = projected("detections");
    let query = |source: TopicSource| Source::from(source).latest(plan::cursor()).plan();
    let time = Time::from_nanos(1000);
    let mut recording = open(file.path()).with_index_budget(96).unwrap();
    for source in [
        images.clone(),
        raw.clone(),
        images.clone(),
        detections.clone(),
    ] {
        finish(&mut recording, &query(source), time);
    }
    assert!(recording.has_index(&images));
    assert!(!recording.has_index(&raw));
    assert!(recording.has_index(&detections));
    assert!(recording.index_stats().1 <= 96);
    let mut recording = open(file.path()).with_index_budget(64).unwrap();
    finish(&mut recording, &query(images.clone()), time);
    let result = finish(
        &mut recording,
        &image_plan(Selection::latest(), Selection::Exact),
        time,
    );
    assert!(matches!(result.state,State::Failed(error) if error.contains("--index-cache-mib")));
    assert!(recording.has_index(&images));
    assert!(!recording.has_index(&detections));
    assert_eq!(recording.index_stats(), (1, 48));
    assert_eq!(
        records(&finish(&mut recording, &query(images.clone()), time)).len(),
        1
    );
    let mut recording = recording.with_index_budget(80).unwrap();
    let spec = InputSpec {
        source: detections.into(),
        selection: Selection::Exact,
    };
    let plan = Source::from(images)
        .select(Selection::latest(), plan::cursor())
        .requiring([spec.clone()])
        .attach([spec.clone(), spec])
        .plan();
    assert_eq!(
        frames(&finish(&mut recording, &plan, time))[0].inputs.len(),
        2
    );
    assert_eq!(
        recording.index_stats(),
        (2, 80),
        "duplicate inputs share an index"
    );
}

#[test]
fn parallel_indexes_match_serial_across_chunks_duplicates_and_rejected_times() {
    let data: Vec<_> = (0..256u64)
        .map(|i| {
            (
                (i % 2) as usize,
                if i % 19 == 0 { u64::MAX } else { i * 17 % 31 },
                1000 + i,
                3000 - i,
            )
        })
        .collect();
    for message_indexes in [false, true] {
        let file = fixture_records(message_indexes, true, &data);
        let mut serial = open(file.path()).with_index_workers(1);
        let mut parallel = open(file.path()).with_index_workers(4);
        let plan = image_plan(
            Selection::AtOrBefore {
                count: NonZeroUsize::new(3).unwrap(),
            },
            Selection::Exact,
        );
        for time in (0..35).chain((0..35).rev()) {
            let expected = finish(&mut serial, &plan, Time::from_nanos(time));
            let mut actual = finish(&mut parallel, &plan, Time::from_nanos(time));
            assert!(
                actual
                    .dependencies
                    .iter()
                    .any(|d| !d.diagnostics.is_empty())
            );
            let publisher = *uuid::Uuid::parse_str(&serial.info.instance)
                .unwrap()
                .as_bytes();
            let State::Ready(Value::Frames(frames)) = &mut actual.state else {
                panic!("frames")
            };
            for frame in frames {
                Arc::make_mut(&mut frame.primary.record)
                    .publication
                    .as_mut()
                    .unwrap()
                    .0 = publisher;
                for input in &mut frame.inputs {
                    if let State::Ready(samples) = input {
                        for sample in samples {
                            Arc::make_mut(&mut sample.record)
                                .publication
                                .as_mut()
                                .unwrap()
                                .0 = publisher;
                        }
                    }
                }
            }
            assert_eq!(
                encode(&Response::PlanData(expected)).unwrap(),
                encode(&Response::PlanData(actual)).unwrap()
            );
        }
        assert_eq!(parallel.projected_messages, serial.projected_messages);
        assert_eq!(parallel.index_stats(), serial.index_stats());
        assert_eq!(parallel.decoded_chunks(), serial.decoded_chunks());
    }
}

#[test]
fn worker_failure_discards_builders_without_cancelling_the_recording() {
    let file = fixture_records(
        true,
        true,
        &[
            (0, 10, 100, 100),
            (0, 20, u64::MAX, 200),
            (0, 30, 300, 300),
            (1, 10, 400, 400),
            (1, 20, 500, 500),
        ],
    );
    let cancel = Arc::new(AtomicBool::new(false));
    let mut recording = QueryRecording::new(
        Recording::open(file.path(), "/replay").unwrap(),
        "failure".into(),
        cancel.clone(),
    )
    .unwrap();
    let result = finish(
        &mut recording,
        &image_plan(Selection::latest(), Selection::Exact),
        Time::from_nanos(1000),
    );
    let State::Failed(error) = result.state else {
        panic!("expected index failure")
    };
    assert!(!error.contains("cancelled"), "originating error: {error}");
    assert!(!cancel.load(Ordering::Relaxed));
    assert_eq!(recording.index_stats(), (0, 0));
    let plan = Source::from(TopicSource::new(
        "/replay/detections",
        TimeProjection::SourceTime,
    ))
    .latest(plan::cursor())
    .plan();
    assert_eq!(
        records(&finish(&mut recording, &plan, Time::from_nanos(1000)))[0]
            .record
            .source_time,
        Time::from_nanos(500)
    );
    cancel.store(true, Ordering::Relaxed);
    assert!(
        recording
            .query_plan(&plan, Time::from_nanos(1000), |_| true)
            .is_err()
    );
}

#[test]
fn exact_and_before_inputs_ignore_publication_time_and_reuse_indexes() {
    for indexes in [false, true] {
        let file = fixture(indexes);
        let mut recording = open(file.path());
        assert_eq!(recording.info.start, Time::from_nanos(210));
        assert_eq!(recording.decoded_chunks(), 0);
        assert_eq!(recording.index_stats(), (0, 0));
        let plan = image_plan(Selection::latest(), Selection::Exact);
        let result = finish(&mut recording, &plan, Time::from_nanos(30));
        assert_eq!(frames(&result)[0].primary.record.time, Time::from_nanos(20));
        assert_eq!(
            input(&frames(&result)[0], 0)[0].record.source_time,
            Time::from_nanos(520)
        );
        let decoded = recording.projected_messages;
        for time in [10, 20, 30, 10] {
            assert_eq!(
                frames(&finish(&mut recording, &plan, Time::from_nanos(time)))[0]
                    .primary
                    .record
                    .time,
                Time::from_nanos(time.min(20))
            );
        }
        assert_eq!(recording.projected_messages, decoded);
        let result = finish(
            &mut recording,
            &image_plan(Selection::latest(), Selection::latest()),
            Time::from_nanos(30),
        );
        assert_eq!(frames(&result)[0].primary.record.time, Time::from_nanos(30));
        assert_eq!(
            input(&frames(&result)[0], 0)[0].record.time,
            Time::from_nanos(20)
        );
        let bad = Source::from(TopicSource::new(
            "/replay/images",
            TimeProjection::field(".missing", TimeEncoding::RosZTime).unwrap(),
        ))
        .latest(plan::cursor())
        .plan();
        assert!(matches!(
            finish(&mut recording, &bad, Time::from_nanos(20)).state,
            State::Failed(_)
        ));
    }
}

#[test]
fn plan_publishes_primary_before_optional_indexes_and_keeps_failures_local() {
    let file = fixture(true);
    let mut recording = open(file.path());
    let detections = Source::from(projected("detections"));
    let invalid = Source::from(TopicSource::new(
        "/replay/detections",
        TimeProjection::field(".missing", TimeEncoding::RosZTime).unwrap(),
    ));
    let query = Source::from(projected("images"))
        .latest(plan::cursor())
        .attach([
            InputSpec {
                source: detections.clone(),
                selection: Selection::Exact,
            },
            InputSpec {
                source: invalid,
                selection: Selection::Exact,
            },
        ])
        .plan();
    let mut partial = false;
    let mut complete = false;
    recording
        .query_plan(&query, Time::from_nanos(15), |result| {
            if let State::Ready(Value::Frames(frames)) = &result.state {
                assert_eq!(frames[0].primary.record.time, Time::from_nanos(10));
                partial |= matches!(frames[0].inputs[0], State::Pending);
                complete |= matches!(&frames[0].inputs[0],State::Ready(records) if records.len()==1)
                    && matches!(frames[0].inputs[1], State::Failed(_));
            }
            true
        })
        .unwrap();
    assert!(partial && complete);
    let chunks = recording.decoded_chunks();
    let result = finish(
        &mut recording,
        &detections.latest(plan::cursor()).effective_time().plan(),
        Time::from_nanos(25),
    );
    assert!(matches!(result.state,State::Ready(Value::Time(Some(t))) if t==Time::from_nanos(20)));
    assert_eq!(
        recording.decoded_chunks(),
        chunks,
        "timestamp expression only reads indexes"
    );
}

async fn ready(observation: &PlanObservation, time: i64) -> Arc<Evaluation> {
    let mut updates = observation.subscribe_updates();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            updates.borrow_and_update();
            let snapshot = observation.snapshot();
            if observation.is_current(&snapshot)
                && snapshot.evaluation.settled()
                && let State::Ready(Value::Frames(frames)) = &snapshot.evaluation.state
                && frames
                    .first()
                    .is_some_and(|frame| frame.primary.record.time == Time::from_nanos(time))
            {
                return snapshot.evaluation.clone();
            }
            updates.changed().await.unwrap();
        }
    })
    .await
    .unwrap_or_else(|_| panic!("query failed: {:?}", observation.snapshot()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_plans_and_independent_cursors_work_without_replay_publishers() {
    let file = fixture(true);
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .unwrap();
    let node = Arc::new(
        context
            .create_node("query_server")
            .with_namespace("/replay")
            .build()
            .await
            .unwrap(),
    );
    let recording = Recording::open(file.path(), "/replay").unwrap();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let server_node = node.clone();
    let server = tokio::spawn(async move {
        serve_queries(
            &server_node,
            recording,
            "test".into(),
            recording::DEFAULT_INDEX_BYTES,
            async {
                let _ = shutdown.await;
            },
        )
        .await
        .unwrap();
    });
    let remote = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(remote) = RemoteMcap::connect(node.clone(), "/replay").await {
                break Arc::new(remote);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(remote.topics().snapshot().topics.len(), 2);
    let first = CursorHandle::new(Cursor::At(Time::from_nanos(10)));
    let a = remote
        .observe(
            first.clone(),
            image_plan(Selection::latest(), Selection::Exact),
        )
        .unwrap();
    let b = remote
        .observe(
            CursorHandle::new(Cursor::At(Time::from_nanos(30))),
            image_plan(Selection::latest(), Selection::Exact),
        )
        .unwrap();
    ready(&a, 10).await;
    ready(&b, 20).await;
    let old = a.snapshot();
    first.set(Cursor::At(Time::from_nanos(30)));
    assert!(!a.is_current(&old));
    ready(&a, 20).await;
    first.set(Cursor::At(Time::from_nanos(10)));
    ready(&a, 10).await;
    ready(&b, 20).await;
    let detections = Source::from(projected("detections"));
    let time = detections
        .latest(plan::cursor())
        .effective_time()
        .observe(
            remote.clone(),
            CursorHandle::new(Cursor::At(Time::from_nanos(30))),
        )
        .unwrap();
    let frame = Source::from(projected("images"))
        .latest(plan::cursor())
        .attach([InputSpec {
            source: detections,
            selection: Selection::Exact,
        }])
        .observe(
            remote.clone(),
            CursorHandle::new(Cursor::At(Time::from_nanos(30))),
        )
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let (State::Ready(Some(time)), State::Ready(Some(frame))) =
                (time.snapshot().state, frame.snapshot().state)
            {
                assert_eq!(time, Time::from_nanos(20));
                assert_eq!(frame.primary.record.time, Time::from_nanos(30));
                assert!(input(&frame, 0).is_empty());
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(node.graph().lock().publishers().count(), 0);
    stop.send(()).unwrap();
    server.await.unwrap();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let server_node = node.clone();
    let recording = Recording::open(file.path(), "/replay").unwrap();
    let server = tokio::spawn(async move {
        serve_queries(
            &server_node,
            recording,
            "replacement".into(),
            recording::DEFAULT_INDEX_BYTES,
            async {
                let _ = shutdown.await;
            },
        )
        .await
        .unwrap();
    });
    first.set(Cursor::At(Time::from_nanos(20)));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !remote.needs_reconnect() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_reuses_ranges_and_detects_replacement_while_paused() {
    let file = fixture(true);
    let mut recording = open(file.path());
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .unwrap();
    let node = Arc::new(context.create_node("cache_test").build().await.unwrap());
    let key = query_key("/replay");
    let queryable = node
        .session()
        .declare_queryable(key.clone())
        .complete(true)
        .await
        .unwrap();
    let (instance, current_instance) = tokio::sync::watch::channel(recording.info.instance.clone());
    let (count, mut requests) = tokio::sync::watch::channel(0usize);
    let (release, delayed) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let mut delayed = Some(delayed);
        while let Ok(incoming) = queryable.recv_async().await {
            let command: Request =
                serde_json::from_slice(&incoming.payload().unwrap().to_bytes()).unwrap();
            let response = match command {
                Request::Plan { cursor, plan, .. } => {
                    count.send_modify(|count| *count += 1);
                    if cursor == Time::from_nanos(20)
                        && let Some(delayed) = delayed.take()
                    {
                        delayed.await.unwrap();
                    }
                    Response::PlanData(finish(&mut recording, &plan, cursor))
                }
                Request::Catalog => {
                    let mut info = recording.info.clone();
                    info.instance = current_instance.borrow().clone();
                    Response::Catalog(info)
                }
            };
            incoming
                .reply(&key, encode(&response).unwrap())
                .await
                .unwrap();
        }
    });
    let remote = RemoteMcap::connect(node, "/replay").await.unwrap();
    let cursor = CursorHandle::new(Cursor::At(Time::from_nanos(10)));
    let observation = remote
        .observe(
            cursor.clone(),
            image_plan(Selection::latest(), Selection::Exact),
        )
        .unwrap();
    let first = ready(&observation, 10).await;
    for time in 11..20 {
        cursor.advance(Time::from_nanos(time));
        tokio::task::yield_now().await;
    }
    for time in [19, 12, 18, 10] {
        cursor.set(Cursor::At(Time::from_nanos(time)));
        assert!(Arc::ptr_eq(&first, &ready(&observation, 10).await));
    }
    assert_eq!(
        *requests.borrow(),
        1,
        "in-range playback must not refetch payloads"
    );
    cursor.set(Cursor::At(Time::from_nanos(20)));
    tokio::time::timeout(Duration::from_secs(5), async {
        while *requests.borrow_and_update() < 2 {
            requests.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    cursor.set(Cursor::At(Time::from_nanos(10)));
    release.send(()).unwrap();
    ready(&observation, 10).await;
    assert_eq!(
        *requests.borrow(),
        3,
        "obsolete seek response must not become current"
    );
    cursor.set(Cursor::At(Time::from_nanos(20)));
    assert!(ready(&observation, 20).await.missing.is_none());
    cursor.set(Cursor::At(Time::from_nanos(30)));
    let incomplete = ready(&observation, 20).await;
    assert_eq!(
        incomplete.missing.as_ref().unwrap().time,
        Time::from_nanos(30)
    );
    assert_eq!(incomplete.valid_for.end, None);
    assert_eq!(*requests.borrow(), 5);
    instance.send_replace("replacement".into());
    tokio::time::timeout(Duration::from_secs(5), async {
        while !remote.needs_reconnect()
            || !matches!(observation.snapshot().evaluation.state, State::Failed(_))
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(*requests.borrow(), 5);
    server.abort();
}

#[test]
fn oversized_cache_chunk_is_loaded_once_for_multiple_selected_records() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let channels = channels::<String>(&["images", "padding"]);
    let mut writer = mcap::WriteOptions::new()
        .chunk_size(None)
        .create(file.reopen().unwrap())
        .unwrap();
    writer
        .write(&mcap::Message {
            channel: channels[1].clone(),
            sequence: 0,
            log_time: 1000,
            publish_time: 1000,
            data: Cow::Owned(
                <String as Message>::Codec::serialize(&"x".repeat(33 * 1024 * 1024)).unwrap(),
            ),
        })
        .unwrap();
    for time in 1..=2 {
        writer
            .write(&mcap::Message {
                channel: channels[0].clone(),
                sequence: time as u32,
                log_time: time,
                publish_time: time,
                data: Cow::Owned(<String as Message>::Codec::serialize(&time.to_string()).unwrap()),
            })
            .unwrap();
    }
    writer.finish().unwrap();
    drop(writer);
    let mut recording = open(file.path());
    let plan = Source::from(TopicSource::new(
        "/replay/images",
        TimeProjection::SourceTime,
    ))
    .select(
        Selection::AtOrBefore {
            count: NonZeroUsize::new(2).unwrap(),
        },
        plan::cursor(),
    )
    .plan();
    assert_eq!(
        records(&finish(&mut recording, &plan, Time::from_nanos(2))).len(),
        2
    );
    assert_eq!(
        recording.decoded_chunks(),
        2,
        "one index scan and one payload load"
    );
    finish(&mut recording, &plan, Time::from_nanos(2));
    assert_eq!(
        recording.decoded_chunks(),
        3,
        "cached index and one grouped payload load"
    );
}

#[test]
fn cli_preserves_router_options() {
    let defaults = Arguments::try_parse_from(["mcap-replay", "test.mcap"]).unwrap();
    assert_eq!(defaults.listen, "tcp/0.0.0.0:7447");
    assert_eq!(defaults.index_cache_mib.get(), 256);
    assert!(
        Arguments::try_parse_from(["mcap-replay", "test.mcap", "--index-cache-mib", "0"]).is_err()
    );
    assert!(
        Arguments::try_parse_from([
            "mcap-replay",
            "test.mcap",
            "--listen",
            "tcp/127.0.0.1:0",
            "--router",
            "tcp/127.0.0.1:7447"
        ])
        .is_err()
    );
}

#[test]
#[ignore = "set MCAP_REPLAY_FILE to measure a real recording"]
fn recorded_file_lazy_index_performance() {
    use std::time::Instant;
    let path = std::env::var_os("MCAP_REPLAY_FILE").expect("MCAP_REPLAY_FILE");
    let start = Instant::now();
    let mut recording = open(std::path::Path::new(&path));
    if let Ok(workers) = std::env::var("MCAP_INDEX_WORKERS") {
        recording = recording.with_index_workers(workers.parse().expect("1..=4 workers"));
    }
    println!(
        "metadata startup {:?}; {} topics; {:?} indexes/bytes",
        start.elapsed(),
        recording.info.topics.len(),
        recording.index_stats()
    );
    assert_eq!(recording.index_stats(), (0, 0));
    assert_eq!(recording.decoded_chunks(), 0);
    let end = recording.info.end;
    let source = Source::from(TopicSource::new(
        "/replay/camera_matrix",
        TimeProjection::SourceTime,
    ))
    .latest(plan::cursor())
    .plan();
    let start = Instant::now();
    assert_eq!(records(&finish(&mut recording, &source, end)).len(), 1);
    println!(
        "source plan {:?}; {:?} indexes/bytes",
        start.elapsed(),
        recording.index_stats()
    );
    let projection = TimeProjection::field(".time", TimeEncoding::RosZTime).unwrap();
    let input = InputSpec {
        source: TopicSource::new("/replay/camera_matrix", projection.clone()).into(),
        selection: Selection::Bracket {
            max_gap: Duration::from_millis(100),
        },
    };
    let plan = Source::from(TopicSource::new("/replay/detected_objects", projection))
        .select(Selection::latest(), plan::cursor())
        .requiring([input.clone()])
        .attach([input])
        .plan();
    let start = Instant::now();
    assert_eq!(frames(&finish(&mut recording, &plan, end)).len(), 1);
    println!(
        "projected plan {:?}; {:?} indexes/bytes; {} extractions",
        start.elapsed(),
        recording.index_stats(),
        recording.projected_messages
    );
    let extractions = recording.projected_messages;
    let mut timings = Vec::new();
    for step in (0..20).chain((0..20).rev()) {
        let start = Instant::now();
        assert_eq!(
            frames(&finish(
                &mut recording,
                &plan,
                end.saturating_sub(Duration::from_millis(step * 20))
            ))
            .len(),
            1
        );
        timings.push(start.elapsed());
    }
    timings.sort_unstable();
    println!(
        "40 seeks median {:?}, max {:?}",
        timings[timings.len() / 2],
        timings.last().unwrap()
    );
    assert_eq!(recording.projected_messages, extractions);
}

#[test]
#[ignore = "set MCAP_REPLAY_FILE to logs/recovered.mcap"]
fn recorded_image_plan_preserves_sparse_detection_gaps() {
    let path = std::env::var_os("MCAP_REPLAY_FILE").expect("MCAP_REPLAY_FILE");
    let mut recording = open(std::path::Path::new(&path));
    let cursor = recording
        .info
        .start
        .saturating_add(Duration::from_millis(1_050_580));
    let images = Source::first_present([
        TopicSource::new(
            "/replay/inputs/left_image",
            TimeProjection::field(".header.stamp", TimeEncoding::RosTime).unwrap(),
        ),
        TopicSource::new(
            "/replay/inputs/stereo_image_pair",
            TimeProjection::field(".left.header.stamp", TimeEncoding::RosTime).unwrap(),
        ),
    ]);
    let projection = TimeProjection::field(".time", TimeEncoding::RosZTime).unwrap();
    let plan = images
        .latest(plan::cursor())
        .attach([
            InputSpec {
                source: TopicSource::new("/replay/detected_objects", projection.clone()).into(),
                selection: Selection::Exact,
            },
            InputSpec {
                source: TopicSource::new("/replay/camera_matrix", projection).into(),
                selection: Selection::Bracket {
                    max_gap: Duration::from_millis(100),
                },
            },
        ])
        .plan();
    let result = finish(&mut recording, &plan, cursor);
    let frame = &frames(&result)[0];
    assert_eq!(frame.primary.topic.name, "/replay/inputs/stereo_image_pair");
    assert!(frame.primary.record.time <= cursor);
    assert!(input(frame, 0).is_empty());
    assert!(!input(frame, 1).is_empty());
    println!(
        "cursor {cursor:?}, image {:?}, exact detections absent, camera matched",
        frame.primary.record.time
    );
}

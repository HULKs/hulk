use std::{borrow::Cow, collections::BTreeMap, sync::Arc, time::Duration};

use ros_z::{Message as _, message::WireEncoder};
use ros_z_debug::{RetentionPolicy, TopicObservation, TopicObserver, TopicObserverOptions};

use super::*;

fn fixture() -> tempfile::NamedTempFile {
    fixture_chunks(&[&[(1, 30), (2, 20)], &[(1, 10), (1, 40)]], true)
}

fn fixture_chunks(chunks: &[&[(u16, u64)]], message_indexes: bool) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    let bundle = String::schema();
    let hash = ros_z_schema::compute_hash(&bundle).unwrap();
    let schema = Arc::new(mcap::Schema {
        id: 1,
        name: String::type_name(),
        encoding: "ros-z-schema-json".into(),
        data: Cow::Owned(serde_json::to_vec(&bundle).unwrap()),
    });
    let channel = |id, topic: &str| {
        Arc::new(mcap::Channel {
            id,
            topic: topic.into(),
            schema: Some(schema.clone()),
            message_encoding: "ros-z-cdr".into(),
            metadata: BTreeMap::from([
                ("ros_z.type_name".into(), String::type_name()),
                ("ros_z.schema_hash".into(), hash.to_hash_string()),
            ]),
        })
    };
    let channels = [channel(1, "a"), channel(2, "b")];
    let mut writer = mcap::WriteOptions::new()
        .chunk_size(None)
        .emit_message_indexes(message_indexes)
        .create(file.reopen().unwrap())
        .unwrap();
    let mut sequence = 0;
    for chunk in chunks {
        for &(id, time) in *chunk {
            writer
                .write(&mcap::Message {
                    channel: channels[usize::from(id - 1)].clone(),
                    sequence,
                    log_time: time,
                    publish_time: time - 1,
                    data: Cow::Owned(
                        <String as ros_z::Message>::Codec::serialize(&time.to_string()).unwrap(),
                    ),
                })
                .unwrap();
            sequence += 1;
        }
        writer.flush().unwrap();
    }
    writer.finish().unwrap();
    file
}

#[test]
fn cached_snapshots_match_indexed_reader_with_ties_and_missing_message_indexes() {
    use mcap::sans_io::indexed_reader::{IndexedReader, IndexedReaderOptions, ReadOrder};
    let chunks: &[&[(u16, u64)]] = &[
        &[(1, 30), (2, 20), (1, 10)],
        &[(1, 5), (1, 30), (1, 30), (2, 50)],
        &[(1, 15), (1, 10)],
        &[(2, 20), (1, 100)],
    ];
    for indexes in [true, false] {
        let file = fixture_chunks(chunks, indexes);
        let summary = mcap::Summary::read(&std::fs::read(file.path()).unwrap())
            .unwrap()
            .unwrap();
        let mut recording = Recording::open(file.path(), "/replay").unwrap();
        for position in (0..=105).chain((0..=105).rev()) {
            let snapshot = recording.snapshot(position).unwrap();
            for (id, topic) in [(1, "a"), (2, "b")] {
                let mut reader = IndexedReader::new_with_options(
                    &summary,
                    IndexedReaderOptions::default()
                        .include_topics([topic])
                        .with_order(ReadOrder::ReverseLogTime)
                        .log_time_before(position + 1),
                )
                .unwrap();
                let expected = recording.next(&mut reader).unwrap();
                let actual = snapshot
                    .iter()
                    .find(|message| message.header.channel_id == id);
                assert_eq!(
                    actual.map(|m| (m.header.log_time, m.header.sequence)),
                    expected
                        .as_ref()
                        .map(|m| (m.header.log_time, m.header.sequence)),
                    "position {position}, channel {id}"
                );
            }
        }
        assert_eq!(
            recording.decoded_chunks,
            chunks.len(),
            "all seeks reuse decoded chunks"
        );
    }
}

#[test]
fn indexed_snapshots_are_inclusive_and_playback_is_ordered() {
    let file = fixture();
    let mut recording = Recording::open(file.path(), "/replay").unwrap();
    assert_eq!((recording.start, recording.end), (10, 40));
    assert!(recording.snapshot(9).unwrap().is_empty());
    let times = |messages: Vec<Message>| {
        messages
            .into_iter()
            .map(|m| m.header.log_time)
            .collect::<Vec<_>>()
    };
    assert_eq!(times(recording.snapshot(10).unwrap()), [10]);
    assert_eq!(times(recording.snapshot(30).unwrap()), [20, 30]);
    assert_eq!(times(recording.snapshot(40).unwrap()), [20, 40]);
    let decoded = recording.decoded_chunks;
    for position in [10, 15, 20, 25, 30, 35, 40, 10] {
        recording.snapshot(position).unwrap();
    }
    assert_eq!(
        recording.decoded_chunks, decoded,
        "scrubbing cached chunks must not decompress again"
    );
    assert_eq!(times(recording.snapshot(10).unwrap()), [10]);
    let mut reader = recording.forward(10).unwrap();
    let mut messages = Vec::new();
    while let Some(message) = recording.next(&mut reader).unwrap() {
        messages.push(message);
    }
    assert_eq!(times(messages), [20, 30, 40]);
}

async fn wait_value(observation: &TopicObservation<String>, expected: &str) {
    let mut updates = observation.subscribe_updates().unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if observation
                .latest()
                .is_some_and(|record| record.value == expected)
            {
                return;
            }
            updates.recv().await.unwrap();
        }
    })
    .await
    .unwrap_or_else(|_| panic!("waiting for {expected}: {:?}", observation.status()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paused_snapshots_rewind_and_reject_other_generations() {
    let file = fixture();
    let mut recording = Recording::open(file.path(), "/replay").unwrap();
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .unwrap();
    let node = Arc::new(
        context
            .create_node("replay_test")
            .with_namespace("/replay")
            .build()
            .await
            .unwrap(),
    );
    let observer = TopicObserver::new(
        node.clone(),
        TopicObserverOptions::with_namespace("/replay").unwrap(),
    );
    let mut status = ReplayStatus {
        instance: "test".into(),
        recording: "test".into(),
        generation: 0,
        start: 10,
        end: 40,
        position: 40,
        playing: false,
        sources: Default::default(),
    };
    let old = seek(&node, &mut recording, 40).await.unwrap();
    update_sources(&mut status, &old);
    observer.set_replay_sources(Some(status.sources.clone()));
    let observation = observer
        .observe_typed::<String>("a")
        .unwrap()
        .retention(RetentionPolicy::time_window(Duration::from_nanos(5)).unwrap())
        .spawn();
    wait_value(&observation, "40").await;
    assert_eq!(
        observation.latest().unwrap().source_time,
        Time::from_nanos(39)
    );

    // New data precedes the manifest; old data follows it.
    let new = seek(&node, &mut recording, 10).await.unwrap();
    update_sources(&mut status, &new);
    observer.set_replay_sources(Some(status.sources.clone()));
    // The receive task may already have installed the new retained snapshot.
    // Only data from the previous generation must be invisible.
    assert!(
        observation
            .latest()
            .is_none_or(|record| record.value == "10")
    );
    assert!(
        observation
            .get_all()
            .iter()
            .all(|record| record.value == "10")
    );
    wait_value(&observation, "10").await;
    for message in recording.snapshot(40).unwrap() {
        publish(&old, message).await.unwrap();
    }
    let late = observer.observe_typed::<String>("a").unwrap().spawn();
    wait_value(&late, "10").await;
    assert_eq!(observation.latest().unwrap().value, "10");
    assert_eq!(
        observation.latest().unwrap().source_time,
        Time::from_nanos(9)
    );
    assert_eq!(observation.get_all().len(), 1);
    let mut raw = node
        .subscriber::<String>("a")
        .publisher(status.sources["/replay/a"].into())
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .raw()
        .build()
        .await
        .unwrap();
    let sample = tokio::time::timeout(Duration::from_secs(3), raw.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        sample.payload().to_bytes().as_ref(),
        <String as ros_z::Message>::Codec::serialize(&"10".to_owned()).unwrap()
    );

    let dynamic = observer.observe_dynamic("a").unwrap().spawn();
    let mut updates = dynamic.subscribe_updates().unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while dynamic.latest_json().is_none() {
            updates.recv().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(dynamic.latest_json().unwrap(), serde_json::json!("10"));

    // No sample before the sparse topic's first message; no old value survives.
    let sparse = observer.observe_typed::<String>("b").unwrap().spawn();
    assert!(sparse.latest().is_none());
    observer.set_replay_sources(Some(Default::default()));
    assert!(observation.latest().is_none());
    assert!(dynamic.latest_json().is_none());
}

async fn control(node: &Node, payload: Option<serde_json::Value>) -> Result<ReplayStatus> {
    let mut query = node
        .session()
        .get(control_key(node.namespace()))
        .timeout(Duration::from_secs(3));
    if let Some(payload) = payload {
        query = query.payload(serde_json::to_vec(&payload)?);
    }
    let replies = query
        .await
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    let reply = replies
        .recv_async()
        .await
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    let sample = reply
        .into_result()
        .map_err(|error| color_eyre::eyre::eyre!("{error:?}"))?;
    Ok(serde_json::from_slice(&sample.payload().to_bytes())?)
}

async fn exercise_controls(recording: Recording) {
    let args =
        Arguments::try_parse_from(["mcap-replay", "test.mcap", "--listen", "tcp/127.0.0.1:0"])
            .unwrap();
    let context = args.connect().await.unwrap();
    let uri = context.session().info().locators().await[0].to_string();
    assert!(!uri.ends_with(":0"));
    let server_node = Arc::new(
        context
            .create_node("control_test")
            .with_namespace("/replay")
            .build()
            .await
            .unwrap(),
    );
    // A separate ROS-Z peer exercises routing and the --router override, rather
    // than relying on same-session delivery inside the embedded router.
    let client_args =
        Arguments::try_parse_from(["mcap-replay", "test.mcap", "--router", &uri]).unwrap();
    let client_context = client_args.connect().await.unwrap();
    let node = Arc::new(
        client_context
            .create_node("control_client")
            .with_namespace("/replay")
            .build()
            .await
            .unwrap(),
    );
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        serve(&server_node, recording, "test.mcap".into(), async {
            let _ = stopped.await;
        })
        .await
    });
    let mut status = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(status) = control(&node, None).await {
                break status;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!status.playing);
    assert_eq!(status.position, status.start);
    let instance = status.instance.clone();
    for position in [
        status.end,
        status.start + (status.end - status.start) / 2,
        status.start,
    ] {
        let before = Instant::now();
        let next = control(
            &node,
            Some(
                serde_json::json!({"command": "seek", "instance": instance, "position": position}),
            ),
        )
        .await
        .unwrap();
        println!(
            "seek {:.3}s: {:?}",
            (position - status.start) as f64 / 1e9,
            before.elapsed()
        );
        assert_eq!(next.position, position);
        assert_eq!(next.generation, status.generation + 1);
        assert_ne!(next.sources, status.sources);
        assert!(!next.playing);
        status = next;
        if position > status.start && position < status.end {
            let observer = TopicObserver::new(
                node.clone(),
                TopicObserverOptions::with_namespace("/replay").unwrap(),
            );
            observer.set_replay_sources(Some(status.sources.clone()));
            let topic = status.sources.keys().next().unwrap();
            let observation = observer.observe_dynamic(topic).unwrap().spawn();
            let mut updates = observation.subscribe_updates().unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                while observation.latest_json().is_none() {
                    updates.recv().await.unwrap();
                }
            })
            .await
            .unwrap_or_else(|_| panic!("dynamic replay {topic}: {:?}", observation.status()));
        }
    }
    assert!(control(&node, Some(serde_json::json!({"command": "seek", "instance": instance, "position": status.end + 1}))).await.is_err());
    assert!(
        control(
            &node,
            Some(serde_json::json!({"command": "play", "instance": "stale"}))
        )
        .await
        .is_err()
    );
    assert_eq!(
        control(&node, None).await.unwrap().generation,
        status.generation
    );
    assert!(
        control(
            &node,
            Some(serde_json::json!({"command": "play", "instance": instance}))
        )
        .await
        .unwrap()
        .playing
    );
    assert!(
        !control(
            &node,
            Some(serde_json::json!({"command": "pause", "instance": instance}))
        )
        .await
        .unwrap()
        .playing
    );
    control(&node, Some(serde_json::json!({"command": "seek", "instance": instance, "position": status.end - 1}))).await.unwrap();
    control(
        &node,
        Some(serde_json::json!({"command": "play", "instance": instance})),
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let status = control(&node, None).await.unwrap();
            if !status.playing {
                assert_eq!(status.position, status.end);
                break;
            }
        }
    })
    .await
    .unwrap();
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[test]
fn cli_defaults_to_embedded_router_and_rejects_conflicting_modes() {
    let args = Arguments::try_parse_from(["mcap-replay", "test.mcap"]).unwrap();
    assert!(args.router.is_none());
    assert_eq!(args.listen, "tcp/0.0.0.0:7447");
    assert!(
        Arguments::try_parse_from([
            "mcap-replay",
            "test.mcap",
            "--router",
            "tcp/127.0.0.1:7447",
            "--listen",
            "tcp/127.0.0.1:0"
        ])
        .is_err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zenoh_controls_seek_pause_reject_stale_commands_and_stop_at_eof() {
    let file = fixture();
    exercise_controls(Recording::open(file.path(), "/replay").unwrap()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "set MCAP_REPLAY_FILE to an indexed ROS-Z recording"]
async fn recorded_file_controls() {
    let path = std::env::var_os("MCAP_REPLAY_FILE").expect("set MCAP_REPLAY_FILE");
    exercise_controls(Recording::open(std::path::Path::new(&path), "/replay").unwrap()).await;
}

/// Run with MCAP_REPLAY_FILE=... cargo test -p mcap-replay scrub_latency -- --ignored --nocapture.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires MCAP_REPLAY_FILE; measures real seek-to-observation latency"]
async fn scrub_latency() {
    let path = std::env::var_os("MCAP_REPLAY_FILE").expect("set MCAP_REPLAY_FILE");
    let mut recording = Recording::open(std::path::Path::new(&path), "/replay").unwrap();
    let args =
        Arguments::try_parse_from(["mcap-replay", "test.mcap", "--listen", "tcp/127.0.0.1:0"])
            .unwrap();
    let context = args.connect().await.unwrap();
    let uri = context.session().info().locators().await[0].to_string();
    let client = ContextBuilder::default()
        .with_router_endpoint(uri)
        .unwrap()
        .build()
        .await
        .unwrap();
    let server_node = context
        .create_node("bench")
        .with_namespace("/replay")
        .build()
        .await
        .unwrap();
    let node = Arc::new(client.create_node("bench_client").build().await.unwrap());
    let observer = TopicObserver::new(
        node,
        TopicObserverOptions::with_namespace("/replay").unwrap(),
    );
    let observation = observer.observe_dynamic("camera_matrix").unwrap().spawn();
    let mut status = ReplayStatus {
        instance: "bench".into(),
        recording: "bench".into(),
        generation: 0,
        start: recording.start,
        end: recording.end,
        position: recording.start,
        playing: false,
        sources: Default::default(),
    };
    let mut retained = BTreeMap::new();
    let mut timings = Vec::new();
    for fraction in [
        0.5, 0.501, 0.502, 0.503, 0.504, 0.505, 0.9, 0.2, 0.7, 0.3, 0.8, 0.4,
    ] {
        let position = status.start + ((status.end - status.start) as f64 * fraction) as u64;
        let start = Instant::now();
        let snapshot = recording.snapshot(position).unwrap();
        let read = start.elapsed();
        let publishers = publishers(&server_node, &recording).await.unwrap();
        let declared = start.elapsed();
        for message in snapshot {
            publish(&publishers, message).await.unwrap();
        }
        retained = publishers;
        update_sources(&mut status, &retained);
        observer.set_replay_sources(Some(status.sources.clone()));
        let mut updates = observation.subscribe_updates().unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while observation.latest_json().is_none() {
                updates.recv().await.unwrap();
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{:?}", observation.status()));
        let elapsed = start.elapsed();
        println!(
            "scrub {fraction:.3}: read {read:?}, declare {:?}, visible {elapsed:?}",
            declared - read
        );
        timings.push(elapsed);
    }
    timings.sort();
    println!(
        "seek-to-decoded median {:?}, max {:?}",
        timings[timings.len() / 2],
        timings.last().unwrap()
    );
    drop(retained);
}

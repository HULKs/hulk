use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use ros_z::{
    Message,
    context::ContextBuilder,
    dynamic::{GetSchema, GetSchemaResponse},
    entity::{EndpointEntity, EndpointKind, Entity, NodeEntity},
    service::ServiceRequest,
    time::Time,
};
use ros_z_debug::query::plan::{self, InputSpec, Selection, Source, State, TopicSource, Value};
use ros_z_debug::query::*;

#[derive(Debug, serde::Serialize, serde::Deserialize, ros_z::Message)]
struct Stamped {
    time: Time,
    value: u32,
}

async fn publish_stamped(
    publisher: &ros_z::pubsub::Publisher<Stamped>,
    stamp: i64,
    source: i64,
    value: u32,
) {
    publisher
        .publish_with_source_time(
            &Stamped {
                time: Time::from_nanos(stamp),
                value,
            },
            Time::from_nanos(source),
        )
        .await
        .unwrap();
}

fn reply_schema<T: Message>(request: ServiceRequest<GetSchema>) {
    request
        .reply(&GetSchemaResponse {
            successful: true,
            failure_reason: String::new(),
            schema_hash: T::schema_hash().to_hash_string(),
            schema: T::schema(),
        })
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_discards_schema_discovery_from_a_replaced_publisher() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .unwrap();
    let node = Arc::new(
        context
            .create_node("query_schema_swap")
            .without_schema_service()
            .build()
            .await
            .unwrap(),
    );
    let mut schemas = node
        .service_server::<GetSchema>("~get_schema")
        .build()
        .await
        .unwrap();
    let stale = node.publisher::<u64>("schema_swap").build().await.unwrap();
    let backend = Live::new(node.clone(), LiveRetention::default());
    let observation = backend
        .observe(
            CursorHandle::new(Cursor::Live),
            Source::from(TopicSource::new("/schema_swap", TimeProjection::SourceTime))
                .latest(plan::cursor())
                .plan(),
        )
        .unwrap();
    let request = tokio::time::timeout(Duration::from_secs(2), schemas.take_request_async())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(request.message().root_type_name, u64::type_name());
    drop(stale);
    let fresh = node
        .publisher::<String>("schema_swap")
        .build()
        .await
        .unwrap();
    reply_schema::<u64>(request);
    let request = tokio::time::timeout(Duration::from_secs(3), schemas.take_request_async())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(request.message().root_type_name, String::type_name());
    reply_schema::<String>(request);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            fresh.publish(&"fresh".to_owned()).await.unwrap();
            if let State::Ready(Value::Records(records)) = &observation.snapshot().evaluation.state
                && let Some(sample) = records.last()
            {
                assert_eq!(
                    decode_record::<String>(&sample.record, &sample.topic)
                        .unwrap()
                        .value,
                    "fresh"
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_join_holds_older_row_without_ui_and_fixed_cursor_accepts_late_data() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .unwrap();
    let node = Arc::new(context.create_node("query_test").build().await.unwrap());
    let images = node.publisher::<Stamped>("images").build().await.unwrap();
    let detections = node
        .publisher::<Stamped>("detections")
        .build()
        .await
        .unwrap();
    let backend = Arc::new(Live::new(Arc::clone(&node), LiveRetention::default()));
    let cursor = CursorHandle::new(Cursor::At(Time::from_nanos(20)));
    let quiet_invalid = backend
        .observe(
            cursor.clone(),
            Source::from(TopicSource::new(
                "/images",
                TimeProjection::field(".missing", TimeEncoding::RosZTime).unwrap(),
            ))
            .latest(plan::cursor())
            .plan(),
        )
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !matches!(quiet_invalid.snapshot().evaluation.state, State::Failed(_)) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let projection = TimeProjection::field(".time", TimeEncoding::RosZTime).unwrap();
    let observer = ros_z_debug::TopicObserver::new(
        Arc::clone(&node),
        ros_z_debug::TopicObserverOptions::default(),
    );
    observer.set_backend(Connection {
        backend: backend.clone(),
        cursor: cursor.clone(),
    });
    let typed = observer
        .observe_typed::<Stamped>("images")
        .unwrap()
        .projection(projection.clone())
        .retention(
            ros_z_debug::RetentionPolicy::time_window_with_max_samples(
                Duration::from_secs(1),
                NonZeroUsize::new(2).unwrap(),
            )
            .unwrap(),
        )
        .spawn();
    let input = InputSpec {
        source: TopicSource::new("/detections", projection.clone()).into(),
        selection: Selection::Exact,
    };
    let query = Source::from(TopicSource::new("/images", projection))
        .select(Selection::latest(), plan::cursor())
        .requiring([input.clone()])
        .attach([input])
        .plan();
    let observation = backend.observe(cursor.clone(), query).unwrap();
    let selected_time = || match &observation.snapshot().evaluation.state {
        State::Ready(Value::Frames(frames)) => frames.last().map(|frame| frame.primary.record.time),
        _ => None,
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        while selected_time() != Some(Time::from_nanos(10)) {
            publish_stamped(&images, 10, 100, 1).await;
            publish_stamped(&detections, 10, 500, 2).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    publish_stamped(&images, 20, 200, 3).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(selected_time(), Some(Time::from_nanos(10)));
    publish_stamped(&detections, 20, 900, 4).await;
    let mut updates = observation.subscribe_updates();
    tokio::time::timeout(Duration::from_secs(3), async {
        while selected_time() != Some(Time::from_nanos(20)) {
            updates.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let snapshot = observation.snapshot();
    let State::Ready(Value::Frames(frames)) = &snapshot.evaluation.state else {
        panic!("frames")
    };
    let State::Ready(records) = &frames[0].inputs[0] else {
        panic!("matched input")
    };
    assert_eq!(records[0].record.source_time, Time::from_nanos(900));
    let mut typed_updates = typed.subscribe_updates().unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while typed
            .latest()
            .is_none_or(|r| r.value.time != Time::from_nanos(20))
        {
            typed_updates.recv().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(typed.latest().unwrap().source_time, Time::from_nanos(200));
    assert_eq!(typed.latest().unwrap().effective_time, Time::from_nanos(20));
    assert!(matches!(
        quiet_invalid.snapshot().evaluation.state,
        State::Failed(_)
    ));
    assert!(
        typed
            .query_status()
            .unwrap()
            .snapshot
            .evaluation
            .dependencies
            .iter()
            .all(|d| d.diagnostics.is_empty()),
        "invalid projections cannot poison another index"
    );
    drop(quiet_invalid);
    cursor.set(Cursor::At(Time::from_nanos(10)));
    assert!(!observation.is_current(&snapshot));
    assert!(
        typed.latest().is_some(),
        "seek retains typed data while its replacement resolves"
    );
    assert!(
        typed.query_status().unwrap().current,
        "Live plans resolve the new cursor synchronously; typed installation remains revision-guarded"
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot = observation.snapshot();
            if observation.is_current(&snapshot) && selected_time() == Some(Time::from_nanos(10)) {
                break;
            }
            updates.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while typed.latest().unwrap().effective_time != Time::from_nanos(10) {
            typed_updates.recv().await.unwrap();
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_fallback_follows_presence_even_while_another_schema_is_unavailable() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .unwrap();
    let node = Arc::new(
        context
            .create_node("fallback_live_test")
            .build()
            .await
            .unwrap(),
    );
    let fallback = node.publisher::<u64>("fallback").build().await.unwrap();
    // This publisher sorts first, but deliberately has no schema service.
    node.graph()
        .add_local_entity(Entity::Endpoint(EndpointEntity {
            id: 999,
            node: NodeEntity::new(node.graph().zid, 999, "absent_schema".into(), "/".into()),
            kind: EndpointKind::Publisher,
            topic: "/aaa_missing_schema".into(),
            type_info: bool::type_info(),
            qos: Default::default(),
        }))
        .unwrap();
    let backend = Arc::new(Live::new(Arc::clone(&node), LiveRetention::default()));
    let catalog = backend.topics();
    let assert_advertisements = |primary_present| {
        let snapshot = catalog.snapshot();
        assert!(!snapshot.loading);
        assert!(snapshot.error.is_none());
        assert!(
            snapshot
                .topics
                .iter()
                .all(|topic| topic.schema.is_none() && topic.error.is_none()),
            "catalog contains advertisements only, even after feeds discover schemas"
        );
        for (name, type_info) in [
            ("/aaa_missing_schema", bool::type_info()),
            ("/fallback", u64::type_info()),
        ] {
            assert!(
                snapshot
                    .topics
                    .iter()
                    .any(|topic| topic.name == name && topic.type_info == type_info)
            );
        }
        assert_eq!(
            snapshot.topics.iter().any(|topic| {
                topic.name == "/primary" && topic.type_info == String::type_info()
            }),
            primary_present
        );
    };
    let observation = Source::first_present([
        TopicSource::new("/primary", TimeProjection::SourceTime),
        TopicSource::new("/fallback", TimeProjection::SourceTime),
    ])
    .latest(plan::cursor())
    .observe(backend, CursorHandle::new(Cursor::Live))
    .unwrap();
    async fn wait_presence(catalog: &TopicList, present: bool) {
        let mut updates = catalog.subscribe_updates();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                updates.borrow_and_update();
                if !catalog.snapshot().loading
                    && catalog
                        .snapshot()
                        .topics
                        .iter()
                        .any(|topic| topic.name == "/primary")
                        == present
                {
                    break;
                }
                updates.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }
    wait_presence(&catalog, false).await;
    assert_advertisements(false);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            fallback.publish(&42u64).await.unwrap();
            if let State::Ready(Some(sample)) = observation.snapshot().state {
                assert_eq!(sample.topic.name, "/fallback");
                assert_eq!(
                    decode_record::<u64>(&sample.record, &sample.topic)
                        .unwrap()
                        .value,
                    42
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let previous = observation.snapshot();
    assert_advertisements(false);
    let primary = node.publisher::<String>("primary").build().await.unwrap();
    wait_presence(&catalog, true).await;
    assert!(
        !matches!(observation.snapshot().state, State::Ready(Some(_))),
        "primary presence wins before its first sample"
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            primary
                .publish(&"primary decoded".to_owned())
                .await
                .unwrap();
            if let State::Ready(Some(sample)) = observation.snapshot().state {
                assert_eq!(sample.topic.name, "/primary");
                assert_eq!(
                    decode_record::<String>(&sample.record, &sample.topic)
                        .unwrap()
                        .value,
                    "primary decoded"
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_advertisements(true);
    drop(primary);
    wait_presence(&catalog, false).await;
    assert_advertisements(false);
    assert!(
        !Arc::ptr_eq(&observation.snapshot().raw, &previous.raw),
        "source changes must reevaluate rather than revive an old snapshot"
    );
    let mut updates = observation.raw().subscribe_updates();
    drop(observation);
    tokio::time::timeout(Duration::from_secs(3), async {
        while updates.changed().await.is_ok() {}
    })
    .await
    .unwrap();
}

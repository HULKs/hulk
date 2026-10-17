use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use ros_z::{
    Message,
    context::ContextBuilder,
    dynamic::{GetSchema, GetSchemaResponse},
    entity::{EndpointEntity, EndpointKind, Entity, NodeEntity},
    node::Node,
    service::ServiceRequest,
    time::Time,
};
use ros_z_debug::query::plan::{self, InputSpec, Selection, Source, State, TopicSource, Value};
use ros_z_debug::query::*;
use tokio::sync::watch;

async fn test_node(name: &str, serve_schema: bool) -> Arc<Node> {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .unwrap();
    let builder = context.create_node(name);
    let builder = if serve_schema {
        builder
    } else {
        builder.without_schema_service()
    };
    Arc::new(builder.build().await.unwrap())
}

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

async fn publish_until_selected<T: Message + PartialEq>(
    publisher: &ros_z::pubsub::Publisher<T>,
    value: &T,
    observation: &TypedPlanObservation<Option<plan::Sample>>,
) -> plan::Sample {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            publisher.publish(value).await.unwrap();
            if let State::Ready(Some(sample)) = observation.snapshot().state
                && decode_record::<T>(&sample.record, &sample.topic)
                    .unwrap()
                    .value
                    == *value
            {
                return sample;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

#[test]
fn external_backends_can_construct_observations() {
    struct Memory;
    impl Backend for Memory {
        fn topics(&self) -> TopicList {
            TopicList::new(watch::channel(Arc::new(TopicSnapshot::default())).1)
        }
        fn observe(
            &self,
            cursor: CursorHandle,
            plan: plan::QueryPlan,
        ) -> ros_z_debug::Result<PlanObservation> {
            let position = cursor.position();
            let Cursor::At(time) = position.cursor else {
                panic!("fixed cursor")
            };
            let snapshot = Arc::new(PlanSnapshot {
                revision: position.revision,
                evaluation: Arc::new(
                    plan.evaluate(time, &[], |_, _| panic!("unexpected payload read"))?,
                ),
            });
            Ok(PlanObservation::new(
                cursor,
                move || snapshot.clone(),
                watch::channel(0).1,
            ))
        }
    }
    let cursor = CursorHandle::new(Cursor::At(Time::zero()));
    let backend = Arc::new(Memory);
    assert!(!backend.topics().snapshot().loading);
    let observation = plan::cursor().observe(backend, cursor.clone()).unwrap();
    let cancel = observation.raw().cancellation_token();
    let _updates = observation.raw().subscribe_updates();
    assert!(
        matches!(observation.snapshot().state, State::Ready(Some(time)) if time == Time::zero())
    );
    drop(observation);
    assert!(cancel.is_cancelled());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_source_time_does_not_require_a_schema_service() {
    let node = test_node("no_schema", false).await;
    let publisher = node.publisher::<u64>("raw").build().await.unwrap();
    let observer = ros_z_debug::TopicObserver::new(node.clone(), Default::default());
    observer.set_backend(Connection {
        backend: Arc::new(Live::new(node.clone(), LiveRetention::default())),
        cursor: CursorHandle::new(Cursor::Live),
    });
    let observation = observer.observe_typed::<u64>("raw").unwrap().spawn();
    let dynamic = observer.observe_dynamic("raw").unwrap().spawn();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            publisher.publish(&42).await.unwrap();
            if observation
                .latest()
                .is_some_and(|sample| sample.value == 42)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(dynamic.latest().is_none());
    let mut schemas = node
        .service_server::<GetSchema>("~get_schema")
        .build()
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(6), async {
        while dynamic.latest().is_none() {
            tokio::select! {
                request = schemas.take_request_async() => reply_schema::<u64>(request.unwrap()),
                _ = tokio::time::sleep(Duration::from_millis(10)) => {},
            }
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_discards_schema_discovery_from_a_replaced_publisher() {
    let node = test_node("query_schema_swap", false).await;
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
    let node = test_node("query_test", true).await;
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
    let node = test_node("fallback_live_test", true).await;
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
    let sample = publish_until_selected(&fallback, &42, &observation).await;
    assert_eq!(sample.topic.name, "/fallback");
    let advertised = catalog.snapshot();
    assert!(advertised.error.is_none());
    assert!(
        advertised
            .topics
            .iter()
            .all(|topic| topic.schema.is_none() && topic.error.is_none()),
        "catalog contains advertisements, not feed discovery results"
    );
    assert!(
        advertised.topics.iter().any(
            |topic| topic.name == "/aaa_missing_schema" && topic.type_info == bool::type_info()
        )
    );
    let primary = node.publisher::<String>("primary").build().await.unwrap();
    wait_presence(&catalog, true).await;
    assert!(
        !matches!(observation.snapshot().state, State::Ready(Some(_))),
        "primary presence wins before its first sample"
    );
    let sample =
        publish_until_selected(&primary, &"primary decoded".to_owned(), &observation).await;
    assert_eq!(sample.topic.name, "/primary");
    drop(primary);
    wait_presence(&catalog, false).await;
    let sample = publish_until_selected(&fallback, &43, &observation).await;
    assert_eq!(sample.topic.name, "/fallback");
    let mut updates = observation.raw().subscribe_updates();
    drop(observation);
    tokio::time::timeout(Duration::from_secs(3), async {
        while updates.changed().await.is_ok() {}
    })
    .await
    .unwrap();
}

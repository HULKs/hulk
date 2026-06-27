use std::sync::Arc;

use ros_z::{prelude::*, time::Time};
use ros_z_debug::{
    JsonRenderPolicy, NonFiniteFloatRenderPolicy, RetentionPolicy, TopicObservationBlockReason,
    TopicObservationStatus, TopicObservationUpdate, TopicObservationUpdateReceiver, TopicObserver,
    TopicObserverOptions,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ros_z::Message)]
#[message(name = "test_msgs::TwixDebugValue")]
struct TwixDebugValue {
    value: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ros_z::Message)]
#[message(name = "test_msgs::FloatDebugValue")]
struct FloatDebugValue {
    value: f64,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_observation_rebuilds_when_inherited_namespace_changes() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("typed_retarget_pub").build().await?;
    let observer_node = Arc::new(
        context
            .create_node("typed_retarget_observer")
            .build()
            .await?,
    );
    let alpha = publisher_node
        .publisher::<String>("/alpha/state")?
        .build()
        .await?;
    let beta = publisher_node
        .publisher::<String>("/beta/state")?
        .build()
        .await?;
    let observer = TopicObserver::new(
        observer_node,
        TopicObserverOptions::with_namespace("/alpha")?,
    );
    let observation = observer
        .observe_typed::<String>("state")?
        .retention(RetentionPolicy::LatestOnly)
        .spawn();

    publish_until_latest_value(&alpha, &observation, "alpha").await?;

    observer.set_namespace("/beta")?;
    publish_until_latest_value(&beta, &observation, "beta").await?;

    assert!(matches!(
        observation.status(),
        TopicObservationStatus::Observing { .. }
    ));
    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_json_observation_exposes_latest_value() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("dynamic_latest_pub").build().await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_latest_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<TwixDebugValue>("/42/debug_value")?
        .build()
        .await?;
    let observer = TopicObserver::new(observer_node, TopicObserverOptions::with_namespace("/42")?);
    let observation = observer
        .observe_dynamic("debug_value")?
        .retention(RetentionPolicy::LatestOnly)
        .spawn();

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&TwixDebugValue { value: 7 }).await?;
            if observation.latest_json() == Some(serde_json::json!({ "value": 7 })) {
                let record = observation
                    .latest_json_record()
                    .expect("latest JSON record should be available");
                assert_eq!(record.value, serde_json::json!({ "value": 7 }));
                assert_eq!(record.metadata.resolved_topic, "/42/debug_value");
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("dynamic observation should expose latest JSON")?;

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_json_observation_exposes_time_window_records() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("dynamic_window_pub").build().await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_window_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<TwixDebugValue>("/42/window_value")?
        .build()
        .await?;
    let observer = TopicObserver::new(observer_node, TopicObserverOptions::with_namespace("/42")?);
    let observation = observer
        .observe_dynamic("window_value")?
        .retention(RetentionPolicy::time_window(
            std::time::Duration::from_secs(10),
        )?)
        .spawn();

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&TwixDebugValue { value: 1 }).await?;
            publisher.publish(&TwixDebugValue { value: 2 }).await?;
            let records = observation.window_json_records(Time::zero(), Time::from_nanos(i64::MAX));
            if records.len() >= 2 {
                assert!(
                    records
                        .iter()
                        .any(|record| record.value == serde_json::json!({ "value": 1 }))
                );
                assert!(
                    records
                        .iter()
                        .any(|record| record.value == serde_json::json!({ "value": 2 }))
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("dynamic observation should expose timestamped window records")?;

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_json_observation_exposes_value_only_window_json() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context
        .create_node("dynamic_value_window_pub")
        .build()
        .await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_value_window_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<TwixDebugValue>("/42/value_window")?
        .build()
        .await?;
    let observer = TopicObserver::new(observer_node, TopicObserverOptions::with_namespace("/42")?);
    let observation = observer
        .observe_dynamic("value_window")?
        .retention(RetentionPolicy::time_window(
            std::time::Duration::from_secs(10),
        )?)
        .spawn();

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&TwixDebugValue { value: 3 }).await?;
            publisher.publish(&TwixDebugValue { value: 4 }).await?;
            let values = observation.window_json(Time::zero(), Time::from_nanos(i64::MAX));
            if values.len() >= 2 {
                assert!(values.contains(&serde_json::json!({ "value": 3 })));
                assert!(values.contains(&serde_json::json!({ "value": 4 })));
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("dynamic observation should expose value-only window JSON")?;

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_json_observation_uses_configured_render_policy() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("dynamic_policy_pub").build().await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_policy_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<FloatDebugValue>("/42/policy_value")?
        .build()
        .await?;
    let observer = TopicObserver::new(observer_node, TopicObserverOptions::with_namespace("/42")?);
    let observation = observer
        .observe_dynamic("policy_value")?
        .json_render_policy(JsonRenderPolicy {
            non_finite_float: NonFiniteFloatRenderPolicy::Null,
            ..JsonRenderPolicy::default()
        })
        .spawn();

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher
                .publish(&FloatDebugValue { value: f64::NAN })
                .await?;
            if observation.latest_json() == Some(serde_json::json!({ "value": null })) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("dynamic observation should use configured JSON render policy")?;

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn private_topic_resolves_delivers_and_preserves_previous_cache_when_later_blocked()
-> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("private_topic_pub").build().await?;
    let observer_node = Arc::new(context.create_node("private_observer").build().await?);
    let publisher = publisher_node
        .publisher::<TwixDebugValue>("/42/behavior_node/trace")?
        .build()
        .await?;
    let observer = TopicObserver::new(observer_node, TopicObserverOptions::with_namespace("/42")?);
    let observation = observer.observe_dynamic("~trace")?.spawn();

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if matches!(
                observation.status(),
                TopicObservationStatus::Blocked {
                    reason: TopicObservationBlockReason::MissingTargetNodeName { .. },
                    ..
                }
            ) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("private observation should report missing node name");

    observation.set_node_name("behavior_node")?;

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&TwixDebugValue { value: 21 }).await?;
            if observation.latest_json() == Some(serde_json::json!({ "value": 21 })) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("private observation should receive resolved topic data")?;

    let record = observation
        .latest_json_record()
        .expect("private observation should retain latest JSON record");
    assert_eq!(record.metadata.resolved_topic, "/42/behavior_node/trace");

    observation.inherit_node_name();

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if let TopicObservationStatus::Blocked {
                previous_cache: Some(previous_cache),
                reason: TopicObservationBlockReason::MissingTargetNodeName { .. },
            } = observation.status()
            {
                assert_eq!(
                    previous_cache.resolved_topic(),
                    Some("/42/behavior_node/trace")
                );
                assert_eq!(
                    observation.latest_json(),
                    Some(serde_json::json!({ "value": 21 }))
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("blocked private observation should preserve previous cache");

    for _ in 0..5 {
        publisher.publish(&TwixDebugValue { value: 22 }).await?;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    assert_eq!(
        observation.latest_json(),
        Some(serde_json::json!({ "value": 21 })),
        "blocked observation should keep a frozen previous cache"
    );

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_observation_freezes_previous_cache_while_retrying_after_retarget()
-> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context
        .create_node("dynamic_retry_freeze_pub")
        .build()
        .await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_retry_freeze_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<TwixDebugValue>("/42/retry_freeze_value")?
        .build()
        .await?;
    let observer = TopicObserver::new(observer_node, {
        let mut options = TopicObserverOptions::with_namespace("/42")?;
        options.set_retry_delay(std::time::Duration::from_secs(30));
        options.set_schema_discovery_timeout(std::time::Duration::from_millis(100));
        options
    });
    let observation = observer.observe_dynamic("retry_freeze_value")?.spawn();

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&TwixDebugValue { value: 1 }).await?;
            if observation.latest_json() == Some(serde_json::json!({ "value": 1 })) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("dynamic observation should receive initial value")?;

    observation.set_topic("retry_freeze_missing")?;

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if matches!(
                observation.status(),
                TopicObservationStatus::Retrying { .. }
            ) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("retargeting to a missing dynamic topic should enter retrying");

    for _ in 0..5 {
        publisher.publish(&TwixDebugValue { value: 2 }).await?;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    assert_eq!(
        observation.latest_json(),
        Some(serde_json::json!({ "value": 1 })),
        "retrying observation should keep a frozen previous cache"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_observer_does_not_close_live_observation_while_handle_remains()
-> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("observer_drop_pub").build().await?;
    let observer_node = Arc::new(
        context
            .create_node("observer_drop_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<String>("/42/observer_drop")?
        .build()
        .await?;
    let observer = TopicObserver::new(
        Arc::clone(&observer_node),
        TopicObserverOptions::with_namespace("/42")?,
    );
    let observation = observer
        .observe_typed::<String>("observer_drop")?
        .retention(RetentionPolicy::LatestOnly)
        .spawn();

    publish_until_latest_value(&publisher, &observation, "alive").await?;
    drop(observer);
    publish_until_latest_value(&publisher, &observation, "still_alive").await?;

    assert!(matches!(
        observation.status(),
        TopicObservationStatus::Observing { .. }
    ));
    assert_eq!(
        observation.latest().map(|record| record.value.clone()),
        Some("still_alive".to_string())
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_observation_cancels_in_flight_dynamic_schema_discovery() -> ros_z_debug::Result<()>
{
    let context = ContextBuilder::default().build().await?;
    let observer_node = Arc::new(context.create_node("dynamic_drop_observer").build().await?);
    let observer = TopicObserver::new(observer_node, {
        let mut options = TopicObserverOptions::with_namespace("/42")?;
        options.set_retry_delay(std::time::Duration::from_secs(30));
        options.set_schema_discovery_timeout(std::time::Duration::from_secs(30));
        options
    });
    let observation = observer.observe_dynamic("never_appears")?.spawn();
    let mut updates = observation.subscribe_updates().unwrap();

    drop(observation);

    tokio::time::timeout(std::time::Duration::from_millis(500), async {
        loop {
            if updates.try_recv().is_err() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("observation drop should cancel schema discovery without waiting for timeout");

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_last_observation_handle_closes_real_spawned_loop() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("drop_handle_pub").build().await?;
    let observer_node = Arc::new(context.create_node("drop_handle_observer").build().await?);
    let publisher = publisher_node
        .publisher::<String>("/42/drop_handle")?
        .build()
        .await?;
    let observer = TopicObserver::new(
        Arc::clone(&observer_node),
        TopicObserverOptions::with_namespace("/42")?,
    );
    let observation = observer
        .observe_typed::<String>("drop_handle")?
        .retention(RetentionPolicy::LatestOnly)
        .spawn();

    publish_until_latest_value(&publisher, &observation, "before_drop").await?;
    wait_for_subscription_count(&observer_node, "/42/drop_handle", 1).await;

    let mut updates = observation.subscribe_updates().unwrap();
    drop(observation);

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if updates.try_recv().is_err()
                && observer_node
                    .graph()
                    .view()
                    .subscription_count_on("/42/drop_handle")
                    == 0
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("dropping last observation handle should close spawned subscription");

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observing_observation_ignores_unrelated_graph_revision() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context.create_node("graph_rebuild_pub").build().await?;
    let observer_node = Arc::new(
        context
            .create_node("graph_rebuild_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<String>("/42/graph_rebuild")?
        .build()
        .await?;
    let observer = TopicObserver::new(
        Arc::clone(&observer_node),
        TopicObserverOptions::with_namespace("/42")?,
    );
    let observation = observer
        .observe_typed::<String>("graph_rebuild")?
        .retention(RetentionPolicy::LatestOnly)
        .spawn();

    publish_until_latest_value(&publisher, &observation, "before_graph_change").await?;
    let mut updates = observation.subscribe_updates().unwrap();

    let _unrelated = publisher_node
        .publisher::<String>("/42/unrelated_graph_change")?
        .build()
        .await?;

    wait_for_publisher_count_without_rebuilding(
        &observer_node,
        "/42/unrelated_graph_change",
        1,
        &mut updates,
    )
    .await;

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_observation_keeps_fresh_graph_baseline_after_late_initial_publisher()
-> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context
        .create_node("dynamic_late_baseline_pub")
        .build()
        .await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_late_baseline_observer")
            .build()
            .await?,
    );
    let observer = TopicObserver::new(Arc::clone(&observer_node), {
        let mut options = TopicObserverOptions::with_namespace("/42")?;
        options.set_retry_delay(std::time::Duration::from_secs(30));
        options.set_schema_discovery_timeout(std::time::Duration::from_secs(2));
        options
    });
    let observation = observer
        .observe_dynamic("late_baseline")?
        .retention(RetentionPolicy::LatestOnly)
        .spawn();

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if matches!(observation.status(), TopicObservationStatus::Building)
                && observer_node
                    .graph()
                    .view()
                    .publisher_count_on("/42/late_baseline")
                    == 0
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("dynamic observation should enter schema discovery before the publisher exists");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let schema = dynamic_int_value_schema("test_msgs::LateBaselineValue");
    let type_info = ros_z::TypeInfo::new(
        "test_msgs::LateBaselineValue",
        ros_z_schema::compute_hash(schema.as_ref()).expect("schema hash"),
    );
    let publisher = publisher_node
        .dynamic_publisher("/42/late_baseline", type_info, Arc::clone(&schema))?
        .build()
        .await?;
    let mut message =
        ros_z::dynamic::DynamicStruct::default_for_schema(&schema).expect("default dynamic struct");
    message.set("value", 31).expect("set value field");
    let payload = ros_z::dynamic::DynamicPayload::from_struct(message).expect("dynamic payload");

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&payload).await?;
            if observation.latest_json() == Some(serde_json::json!({ "value": 31 })) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("dynamic observation should finish initial schema discovery")?;
    let mut updates = observation.subscribe_updates().unwrap();

    let _unrelated = publisher_node
        .publisher::<String>("/42/dynamic_late_baseline_unrelated")?
        .build()
        .await?;

    wait_for_publisher_count_without_rebuilding(
        &observer_node,
        "/42/dynamic_late_baseline_unrelated",
        1,
        &mut updates,
    )
    .await;

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observing_observation_rebuilds_on_relevant_publisher_graph_change()
-> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let publisher_node = context
        .create_node("relevant_graph_rebuild_pub")
        .build()
        .await?;
    let observer_node = Arc::new(
        context
            .create_node("relevant_graph_rebuild_observer")
            .build()
            .await?,
    );
    let publisher = publisher_node
        .publisher::<String>("/42/relevant_graph_rebuild")?
        .build()
        .await?;
    let observer = TopicObserver::new(
        Arc::clone(&observer_node),
        TopicObserverOptions::with_namespace("/42")?,
    );
    let observation = observer
        .observe_typed::<String>("relevant_graph_rebuild")?
        .retention(RetentionPolicy::LatestOnly)
        .spawn();

    publish_until_latest_value(&publisher, &observation, "before_graph_change").await?;
    let mut updates = observation.subscribe_updates().unwrap();

    let _second_publisher = publisher_node
        .publisher::<String>("/42/relevant_graph_rebuild")?
        .build()
        .await?;

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if matches!(
                updates.recv().await,
                Ok(TopicObservationUpdate::StatusChanged(
                    TopicObservationStatus::Rebuilding { .. }
                ))
            ) {
                break;
            }
        }
    })
    .await
    .expect("relevant publisher graph change should wake a rebuild");

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_observation_retries_until_schema_publisher_appears() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_retry_observer")
            .build()
            .await?,
    );
    let observer = TopicObserver::new(observer_node, {
        let mut options = TopicObserverOptions::with_namespace("/42")?;
        options.set_retry_delay(std::time::Duration::from_millis(25));
        options.set_schema_discovery_timeout(std::time::Duration::from_millis(100));
        options
    });
    let observation = observer.observe_dynamic("late_value")?.spawn();

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if matches!(
                observation.status(),
                TopicObservationStatus::Retrying { .. }
            ) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("observation should retry before publisher exists");

    let publisher_node = context.create_node("dynamic_retry_pub").build().await?;
    let publisher = publisher_node
        .publisher::<TwixDebugValue>("/42/late_value")?
        .build()
        .await?;

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&TwixDebugValue { value: 9 }).await?;
            if observation.latest_json() == Some(serde_json::json!({ "value": 9 })) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("dynamic observation should recover after publisher appears")?;

    drop(observer);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_observation_graph_change_wakes_retry_before_retry_delay() -> ros_z_debug::Result<()>
{
    let context = ContextBuilder::default().build().await?;
    let observer_node = Arc::new(
        context
            .create_node("dynamic_graph_wakeup_observer")
            .build()
            .await?,
    );
    let observer = TopicObserver::new(observer_node, {
        let mut options = TopicObserverOptions::with_namespace("/42")?;
        options.set_retry_delay(std::time::Duration::from_secs(30));
        options.set_schema_discovery_timeout(std::time::Duration::from_millis(500));
        options
    });
    let observation = observer.observe_dynamic("graph_wakeup_value")?.spawn();

    let early_retry = tokio::time::timeout(
        std::time::Duration::from_millis(150),
        wait_for_dynamic_retrying(&observation),
    )
    .await;
    assert!(
        early_retry.is_err(),
        "dynamic schema discovery should wait before retrying when no publisher exists"
    );

    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        wait_for_dynamic_retrying(&observation),
    )
    .await
    .expect("observation should retry after schema discovery fails");

    let publisher_node = context
        .create_node("dynamic_graph_wakeup_pub")
        .build()
        .await?;
    let publisher = publisher_node
        .publisher::<TwixDebugValue>("/42/graph_wakeup_value")?
        .build()
        .await?;

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&TwixDebugValue { value: 17 }).await?;
            if observation.latest_json() == Some(serde_json::json!({ "value": 17 })) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("graph change should wake retry before retry delay")?;

    drop(observer);
    Ok(())
}

async fn publish_until_latest_value(
    publisher: &ros_z::pubsub::Publisher<String>,
    observation: &ros_z_debug::TopicObservation<String>,
    expected: &str,
) -> ros_z_debug::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            publisher.publish(&expected.to_string()).await?;
            if observation
                .latest()
                .is_some_and(|record| record.value == expected)
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, ros_z_debug::Error>(())
    })
    .await
    .expect("observation should receive expected value")
}

fn dynamic_int_value_schema(type_name: &str) -> ros_z::dynamic::Schema {
    use ros_z_schema::{
        FieldDef, PrimitiveTypeDef, SchemaBundle, StructDef, TypeDef, TypeDefinition,
        TypeDefinitions, TypeName,
    };

    let name = TypeName::new(type_name).expect("valid dynamic type name");
    Arc::new(SchemaBundle {
        root: TypeDef::Named(name.clone()),
        definitions: TypeDefinitions::from([(
            name,
            TypeDefinition::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "value",
                    TypeDef::Primitive(PrimitiveTypeDef::I32),
                )],
            }),
        )]),
    })
}

async fn wait_for_publisher_count_without_rebuilding(
    node: &Arc<ros_z::node::Node>,
    topic: &str,
    expected: usize,
    updates: &mut TopicObservationUpdateReceiver,
) {
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            drain_non_rebuilding_updates(updates);
            if node.graph().view().publisher_count_on(topic) == expected {
                drain_non_rebuilding_updates(updates);
                break;
            }

            tokio::select! {
                update = updates.recv() => {
                    assert_non_rebuilding_update(update.expect("observation update stream should remain open"));
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
            }
        }
    })
    .await
    .expect("graph should observe publisher without rebuilding");

    assert_no_rebuilding_update_for(std::time::Duration::from_millis(200), updates).await;
}

async fn assert_no_rebuilding_update_for(
    duration: std::time::Duration,
    updates: &mut TopicObservationUpdateReceiver,
) {
    let no_rebuild = tokio::time::timeout(duration, async {
        loop {
            assert_non_rebuilding_update(
                updates
                    .recv()
                    .await
                    .expect("observation update stream should remain open"),
            );
        }
    })
    .await;

    assert!(
        no_rebuild.is_err(),
        "unrelated graph revisions should not rebuild the observation"
    );
}

fn drain_non_rebuilding_updates(updates: &mut TopicObservationUpdateReceiver) {
    while let Some(update) = updates
        .try_recv()
        .expect("observation update stream should remain open")
    {
        assert_non_rebuilding_update(update);
    }
}

fn assert_non_rebuilding_update(update: TopicObservationUpdate) {
    match update {
        TopicObservationUpdate::StatusChanged(TopicObservationStatus::Rebuilding { .. }) => {
            panic!("unrelated graph revisions should not rebuild the observation");
        }
        TopicObservationUpdate::Lagged { dropped } => {
            panic!("missed {dropped} observation updates while checking for rebuild");
        }
        _ => {}
    }
}

async fn wait_for_dynamic_retrying(observation: &ros_z_debug::DynamicTopicObservation) {
    loop {
        if matches!(
            observation.status(),
            TopicObservationStatus::Retrying { .. }
        ) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

async fn wait_for_subscription_count(node: &Arc<ros_z::node::Node>, topic: &str, expected: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if node.graph().view().subscription_count_on(topic) == expected {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("graph should observe subscription count");
}

use std::{sync::Arc, time::Duration};

use ros_z::{
    context::ContextBuilder,
    qos::{QosDurability, QosProfile},
};
use ros_z_debug::{
    CachedSubscriptionBuilder, ObservationPolicy, TopicObserver, TopicObserverOptions,
};

async fn wait_until(check: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("observation should reach the expected state");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cached_and_observed_subscriptions_replay_retained_values_unless_overridden()
-> ros_z_debug::Result<()> {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_connect_endpoints(Vec::<String>::new())
        .with_listen_endpoints(Vec::<String>::new())
        .build()
        .await?;
    let robot = context.create_node("robot").build().await?;
    let twix = Arc::new(context.create_node("twix").build().await?);
    let mut cached = Box::pin(
        CachedSubscriptionBuilder::new(twix.clone(), "layout")?.build_json(Default::default()),
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut cached)
            .await
            .is_err()
    );
    let publisher = robot
        .publisher::<i32>("layout")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    assert!(publisher.publish_if_subscribed(|| async { 42 }).await?);
    let cached = cached.await?;

    let observer = TopicObserver::new(twix, TopicObserverOptions::default());
    let typed = observer.observe_typed::<i32>("layout")?.spawn();
    let dynamic = observer.observe_dynamic("layout")?.spawn();
    let volatile = observer
        .observe_typed::<i32>("layout")?
        .policy(ObservationPolicy::default().with_subscriber_qos(QosProfile::default()))
        .spawn();

    wait_until(|| {
        typed.latest().is_some_and(|record| record.value == 42)
            && dynamic.latest_json() == Some(serde_json::json!(42))
            && cached.latest_json() == Some(serde_json::json!(42))
            && publisher.subscriber_count() == 4
    })
    .await;
    assert_eq!(
        robot
            .graph()
            .lock()
            .subscriptions_on("/layout")
            .filter(|endpoint| {
                QosProfile::try_from(endpoint.qos).unwrap().durability == QosDurability::Volatile
            })
            .count(),
        1
    );
    assert!(volatile.latest().is_none());

    publisher.publish(&43).await?;
    wait_until(|| volatile.latest().is_some_and(|record| record.value == 43)).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observations_adapt_durability_when_publishers_change() -> ros_z_debug::Result<()> {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_connect_endpoints(Vec::<String>::new())
        .with_listen_endpoints(Vec::<String>::new())
        .build()
        .await?;
    let robot = context.create_node("robot").build().await?;
    let twix = Arc::new(context.create_node("twix").build().await?);
    let observer = TopicObserver::new(twix, TopicObserverOptions::default());
    let typed = observer.observe_typed::<i32>("layout")?.spawn();
    let dynamic = observer.observe_dynamic("layout")?.spawn();

    let subscriptions_have_durability = |durability| {
        let graph = robot.graph().lock();
        let subscriptions = graph.subscriptions_on("/layout").collect::<Vec<_>>();
        subscriptions.len() == 2
            && subscriptions.iter().all(|endpoint| {
                QosProfile::try_from(endpoint.qos).unwrap().durability == durability
            })
    };
    let volatile = robot.publisher::<i32>("layout").build().await?;
    wait_until(|| subscriptions_have_durability(QosDurability::Volatile)).await;

    let retained = robot
        .publisher::<i32>("layout")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    drop(volatile);
    retained.publish(&2).await?;
    wait_until(|| {
        subscriptions_have_durability(QosDurability::TransientLocal)
            && typed.latest().is_some_and(|record| record.value == 2)
            && dynamic.latest_json() == Some(serde_json::json!(2))
    })
    .await;

    let volatile = robot.publisher::<i32>("layout").build().await?;
    wait_until(|| subscriptions_have_durability(QosDurability::Volatile)).await;
    volatile.publish(&3).await?;
    wait_until(|| {
        typed.latest().is_some_and(|record| record.value == 3)
            && dynamic.latest_json() == Some(serde_json::json!(3))
    })
    .await;
    Ok(())
}

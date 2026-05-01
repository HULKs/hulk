use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ros_z::{
    Message, Result, ServiceTypeInfo,
    context::ContextBuilder,
    lifecycle::{CallbackReturn, LifecycleClient, LifecycleState},
};
use ros_z_msgs::ros::std_msgs::String as RosString;

static TEST_NAMESPACE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_test_namespace() -> String {
    format!(
        "/lifecycle_{}",
        TEST_NAMESPACE_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[test]
fn lifecycle_inline_message_identities_are_native_ros_z() {
    use ros_z::lifecycle::msgs::{LcState, LcTransition, LcTransitionEvent};

    assert_eq!(LcState::type_name(), "ros_z::lifecycle::State");
    assert_eq!(LcTransition::type_name(), "ros_z::lifecycle::Transition");
    assert_eq!(
        LcTransitionEvent::type_name(),
        "ros_z::lifecycle::TransitionEvent"
    );
}

#[test]
fn lifecycle_service_identities_are_native_ros_z() {
    use ros_z::lifecycle::msgs::{
        ChangeState, GetAvailableStates, GetAvailableTransitions, GetState,
    };

    assert_eq!(
        ChangeState::service_type_info().name,
        "ros_z::lifecycle::ChangeState"
    );
    assert_eq!(
        GetState::service_type_info().name,
        "ros_z::lifecycle::GetState"
    );
    assert_eq!(
        GetAvailableStates::service_type_info().name,
        "ros_z::lifecycle::GetAvailableStates"
    );
    assert_eq!(
        GetAvailableTransitions::service_type_info().name,
        "ros_z::lifecycle::GetAvailableTransitions"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn lifecycle_graph_uses_native_service_and_event_channels() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = context
        .create_lifecycle_node("native_lifecycle_graph_target")
        .with_namespace(namespace.clone())
        .build()
        .await?;

    let graph = lifecycle_node.inner.graph();
    let service_names = [
        format!("{namespace}/native_lifecycle_graph_target/_ros_z_lifecycle/change_state"),
        format!("{namespace}/native_lifecycle_graph_target/_ros_z_lifecycle/get_state"),
        format!("{namespace}/native_lifecycle_graph_target/_ros_z_lifecycle/get_available_states"),
        format!(
            "{namespace}/native_lifecycle_graph_target/_ros_z_lifecycle/get_available_transitions"
        ),
    ];
    let topic_name =
        format!("{namespace}/native_lifecycle_graph_target/_ros_z_lifecycle/transition_event");

    let discovered_services = graph.get_service_names_and_types();
    for service_name in &service_names {
        assert!(
            discovered_services
                .iter()
                .any(|(name, _)| name == service_name),
            "expected {service_name} in {discovered_services:?}"
        );
    }

    let discovered_topics = graph.get_topic_names_and_types();
    assert!(
        discovered_topics
            .iter()
            .any(|(name, _)| name == &topic_name),
        "expected {topic_name} in {discovered_topics:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn lifecycle_publisher_publish_respects_activation_state() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let mut lifecycle_node = context
        .create_lifecycle_node("lifecycle_publisher_async")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let observer_node = context
        .create_node("lifecycle_async_observer")
        .with_namespace(namespace)
        .build()
        .await?;

    let publisher = lifecycle_node
        .create_publisher::<RosString>("lifecycle_async_topic")
        .await?;
    let subscriber = observer_node
        .subscriber::<RosString>("lifecycle_async_topic")
        .build()
        .await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    publisher
        .publish(&RosString {
            data: "dropped while inactive".into(),
        })
        .await?;

    assert!(
        tokio::time::timeout(Duration::from_millis(200), subscriber.recv())
            .await
            .is_err(),
        "inactive lifecycle publisher should drop async publications"
    );

    lifecycle_node.configure().await?;
    lifecycle_node.activate().await?;

    let message = RosString {
        data: "delivered while active".into(),
    };
    publisher.publish(&message).await?;

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("timed out waiting for active lifecycle async publication")?;
    assert_eq!(received.data, message.data);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn lifecycle_transition_event_has_nonzero_timestamp() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let mut lifecycle_node = context
        .create_lifecycle_node("timestamp_source")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let observer = context
        .create_node("timestamp_observer")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let topic = format!("{namespace}/timestamp_source/_ros_z_lifecycle/transition_event");
    let subscriber = observer
        .subscriber::<ros_z::lifecycle::msgs::LcTransitionEvent>(&topic)
        .build()
        .await?;
    assert!(
        subscriber
            .wait_for_publishers(1, Duration::from_secs(5))
            .await
    );

    lifecycle_node.configure().await?;
    let event = tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await??;
    assert!(event.timestamp.sec != 0 || event.timestamp.nanosec != 0);
    Ok(())
}

#[test]
fn direct_transition_publishes_transition_event_on_current_thread_runtime() -> Result<()> {
    let (mut lifecycle_node, subscriber) = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .expect("failed to build setup runtime")
        .block_on(async {
            let context = ContextBuilder::default().build().await?;
            let namespace = next_test_namespace();
            let lifecycle_node = context
                .create_lifecycle_node("current_thread_transition_event")
                .with_namespace(namespace.clone())
                .build()
                .await?;
            let observer = context
                .create_node("current_thread_transition_event_observer")
                .with_namespace(namespace.clone())
                .build()
                .await?;
            let topic = format!(
                "{namespace}/current_thread_transition_event/_ros_z_lifecycle/transition_event"
            );
            let subscriber = observer
                .subscriber::<ros_z::lifecycle::msgs::LcTransitionEvent>(&topic)
                .build()
                .await?;
            assert!(
                subscriber
                    .wait_for_publishers(1, Duration::from_secs(5))
                    .await
            );
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((lifecycle_node, subscriber))
        })?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build current_thread runtime");

    let event = runtime.block_on(async {
        lifecycle_node.configure().await?;
        tokio::time::timeout(Duration::from_secs(2), subscriber.recv()).await?
    })?;

    assert_eq!(lifecycle_node.get_current_state(), LifecycleState::Inactive);
    assert_eq!(event.start_state.id, LifecycleState::Unconfigured.id());
    assert_eq!(event.goal_state.id, LifecycleState::Inactive.id());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_change_state_invokes_on_configure_callback() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = context
        .create_lifecycle_node("remote_lifecycle_target")
        .with_namespace(namespace.clone())
        .build()
        .await?;

    let called = Arc::new(AtomicUsize::new(0));
    let called_clone = called.clone();
    lifecycle_node.set_on_configure(move |_| {
        called_clone.fetch_add(1, Ordering::SeqCst);
        CallbackReturn::Success
    });

    let manager = context
        .create_node("remote_lifecycle_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/remote_lifecycle_target");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;

    assert!(client.configure(Duration::from_secs(2)).await?);
    assert_eq!(called.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn lifecycle_node_rejects_invalid_direct_transition() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let mut lifecycle_node = context
        .create_lifecycle_node("invalid_direct_transition")
        .build()
        .await?;

    let error = lifecycle_node
        .activate()
        .await
        .expect_err("activate from unconfigured is invalid");
    assert!(error.to_string().contains("invalid lifecycle transition"));
    assert_eq!(
        lifecycle_node.get_current_state(),
        LifecycleState::Unconfigured
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn lifecycle_node_rejects_shutdown_after_finalized() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let mut lifecycle_node = context
        .create_lifecycle_node("invalid_shutdown_after_finalized")
        .build()
        .await?;

    assert_eq!(lifecycle_node.shutdown().await?, LifecycleState::Finalized);
    let error = lifecycle_node
        .shutdown()
        .await
        .expect_err("shutdown from finalized is invalid");
    assert!(error.to_string().contains("invalid lifecycle transition"));
    assert_eq!(
        lifecycle_node.get_current_state(),
        LifecycleState::Finalized
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_transition_errors_while_remote_transition_is_in_progress() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let mut lifecycle_node = context
        .create_lifecycle_node("direct_during_remote_target")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let transition_gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let transition_gate_clone = transition_gate.clone();
    lifecycle_node.set_on_configure(move |_| {
        let (lock, cvar) = &*transition_gate_clone;
        let mut state = lock.lock().unwrap();
        state.0 = true;
        cvar.notify_all();
        while !state.1 {
            state = cvar.wait(state).unwrap();
        }
        CallbackReturn::Success
    });

    let manager = context
        .create_node("direct_during_remote_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/direct_during_remote_target");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;
    let configure = tokio::spawn(async move { client.configure(Duration::from_secs(2)).await });

    let entered = tokio::time::timeout(Duration::from_secs(5), async {
        let (lock, _) = &*transition_gate;
        loop {
            if lock.lock().unwrap().0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    if entered.is_err() {
        let (lock, cvar) = &*transition_gate;
        let mut state = lock.lock().unwrap();
        state.1 = true;
        cvar.notify_all();
    }
    entered.expect("remote configure did not enter callback");

    let direct_result = lifecycle_node.activate().await;

    {
        let (lock, cvar) = &*transition_gate;
        let mut state = lock.lock().unwrap();
        state.1 = true;
        cvar.notify_all();
    }
    assert!(configure.await.unwrap()?);

    let error = direct_result
        .expect_err("direct transition should fail while remote transition is in progress");
    assert!(error.to_string().contains("transition already in progress"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_change_state_callback_failure_rejects_transition() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = context
        .create_lifecycle_node("remote_lifecycle_failure_target")
        .with_namespace(namespace.clone())
        .build()
        .await?;

    let called = Arc::new(AtomicUsize::new(0));
    let called_clone = called.clone();
    lifecycle_node.set_on_configure(move |_| {
        called_clone.fetch_add(1, Ordering::SeqCst);
        CallbackReturn::Failure
    });

    let manager = context
        .create_node("remote_lifecycle_failure_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/remote_lifecycle_failure_target");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;

    assert!(!client.configure(Duration::from_secs(2)).await?);
    assert_eq!(called.load(Ordering::SeqCst), 1);
    assert_eq!(
        client.get_state(Duration::from_secs(2)).await?,
        LifecycleState::Unconfigured
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_change_state_callback_error_reports_failure_after_error_processing() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = context
        .create_lifecycle_node("remote_lifecycle_error_target")
        .with_namespace(namespace.clone())
        .build()
        .await?;

    lifecycle_node.set_on_configure(|_| CallbackReturn::Error);

    let manager = context
        .create_node("remote_lifecycle_error_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/remote_lifecycle_error_target");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;

    assert!(!client.configure(Duration::from_secs(2)).await?);
    assert_eq!(
        client.get_state(Duration::from_secs(2)).await?,
        LifecycleState::Finalized
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_activate_enables_lifecycle_publisher() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = context
        .create_lifecycle_node("remote_lifecycle_publisher_target")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let observer_node = context
        .create_node("remote_lifecycle_publisher_observer")
        .with_namespace(namespace.clone())
        .build()
        .await?;

    let publisher = lifecycle_node
        .create_publisher::<RosString>("remote_lifecycle_topic")
        .await?;
    let subscriber = observer_node
        .subscriber::<RosString>("remote_lifecycle_topic")
        .build()
        .await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    publisher
        .publish(&RosString {
            data: "dropped before remote activation".into(),
        })
        .await?;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), subscriber.recv())
            .await
            .is_err(),
        "inactive lifecycle publisher should drop async publications"
    );

    let manager = context
        .create_node("remote_lifecycle_publisher_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/remote_lifecycle_publisher_target");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;

    assert!(client.configure(Duration::from_secs(2)).await?);
    assert!(client.activate(Duration::from_secs(2)).await?);

    let message = RosString {
        data: "delivered after remote activation".into(),
    };
    publisher.publish(&message).await?;

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("timed out waiting for remote lifecycle publication")?;
    assert_eq!(received.data, message.data);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configure_callback_can_read_current_state_without_deadlocking() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = Arc::new(
        context
            .create_lifecycle_node("callback_state_reader")
            .with_namespace(namespace.clone())
            .build()
            .await?,
    );

    let lifecycle_node_clone = lifecycle_node.clone();
    lifecycle_node.set_on_configure(move |_| {
        let _ = lifecycle_node_clone.get_current_state();
        CallbackReturn::Success
    });

    let manager = context
        .create_node("callback_state_reader_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/callback_state_reader");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;

    let configured = tokio::time::timeout(
        Duration::from_secs(2),
        client.configure(Duration::from_secs(2)),
    )
    .await
    .expect("configure callback deadlocked")?;
    assert!(configured);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configure_callback_observes_configuring_state() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = Arc::new(
        context
            .create_lifecycle_node("callback_configuring_state_reader")
            .with_namespace(namespace.clone())
            .build()
            .await?,
    );
    let observed_state = Arc::new(std::sync::Mutex::new(None));

    let lifecycle_node_clone = lifecycle_node.clone();
    let observed_state_clone = observed_state.clone();
    lifecycle_node.set_on_configure(move |_| {
        *observed_state_clone.lock().unwrap() = Some(lifecycle_node_clone.get_current_state());
        CallbackReturn::Success
    });

    let manager = context
        .create_node("callback_configuring_state_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/callback_configuring_state_reader");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;

    assert!(client.configure(Duration::from_secs(2)).await?);
    assert_eq!(
        *observed_state.lock().unwrap(),
        Some(LifecycleState::Configuring)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lifecycle_publisher_created_after_remote_activation_is_active() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let namespace = next_test_namespace();
    let lifecycle_node = context
        .create_lifecycle_node("remote_active_late_publisher_target")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let observer_node = context
        .create_node("remote_active_late_publisher_observer")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let manager = context
        .create_node("remote_active_late_publisher_manager")
        .with_namespace(namespace.clone())
        .build()
        .await?;
    let target_fqn = format!("{namespace}/remote_active_late_publisher_target");
    let client = LifecycleClient::new(&manager, &target_fqn).await?;

    assert!(client.configure(Duration::from_secs(2)).await?);
    assert!(client.activate(Duration::from_secs(2)).await?);

    let publisher = lifecycle_node
        .create_publisher::<RosString>("late_remote_lifecycle_topic")
        .await?;
    let subscriber = observer_node
        .subscriber::<RosString>("late_remote_lifecycle_topic")
        .build()
        .await?;
    assert!(
        subscriber
            .wait_for_publishers(1, Duration::from_secs(5))
            .await
    );

    let message = RosString {
        data: "delivered by late publisher".into(),
    };
    publisher.publish(&message).await?;

    let received = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("timed out waiting for late lifecycle publication")?;
    assert_eq!(received.data, message.data);

    Ok(())
}

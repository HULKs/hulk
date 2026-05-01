use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ros_z::{Result, context::ContextBuilder, define_action};
use serde::{Deserialize, Serialize};
use tokio::{task::JoinHandle, time::timeout};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestGoal {
    order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestResult {
    value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestFeedback {
    progress: i32,
}

struct TestAction;

define_action! {
    TestAction,
    action_name: "test_action",
    Goal: TestGoal,
    Result: TestResult,
    Feedback: TestFeedback,
}

#[test]
fn manual_action_default_protocol_type_info_uses_native_names() {
    assert_eq!(
        <TestAction as ros_z::action::Action>::send_goal_type_info().name,
        "test_action::SendGoal"
    );
    assert_eq!(
        <TestAction as ros_z::action::Action>::get_result_type_info().name,
        "test_action::GetResult"
    );
    assert_eq!(
        <TestAction as ros_z::action::Action>::cancel_goal_type_info().name,
        "ros_z::action::CancelGoal"
    );
    assert_eq!(
        <TestAction as ros_z::action::Action>::feedback_type_info().name,
        "test_action::FeedbackMessage"
    );
    assert_eq!(
        <TestAction as ros_z::action::Action>::status_type_info().name,
        "ros_z::action::GoalStatusArray"
    );
}

static TEST_NAMESPACE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_test_namespace() -> String {
    format!(
        "/action_api_alignment_{}",
        TEST_NAMESPACE_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

async fn setup_action_api_alignment() -> Result<(
    ros_z::context::Context,
    ros_z::node::Node,
    ros_z::action::client::ActionClient<TestAction>,
    ros_z::action::server::ActionServer<TestAction>,
)> {
    let context = ContextBuilder::default().build().await?;
    let node = context
        .create_node("action_api_alignment_node")
        .with_namespace(next_test_namespace())
        .build()
        .await?;

    let client = node
        .create_action_client::<TestAction>("test_action_alignment")
        .build()
        .await?;
    let server = node
        .create_action_server::<TestAction>("test_action_alignment")
        .build()
        .await?;

    assert!(client.wait_for_server_async(Duration::from_secs(2)).await);

    Ok((context, node, client, server))
}

fn assert_manual_action_receive_signatures(
    _server: &ros_z::action::server::ActionServer<TestAction>,
) {
    let _: fn(
        &ros_z::action::server::ActionServer<TestAction>,
    ) -> ros_z::Result<ros_z::action::server::CancelRequest> =
        ros_z::action::server::ActionServer::<TestAction>::receive_cancel;
    let _: fn(
        &ros_z::action::server::ActionServer<TestAction>,
    ) -> ros_z::Result<ros_z::action::server::ResultRequestHandle<TestAction>> =
        ros_z::action::server::ActionServer::<TestAction>::receive_result_request;
}

#[allow(dead_code)]
async fn assert_manual_cancel_reply_api(
    request: ros_z::action::server::CancelRequest,
) -> Result<()> {
    let _goal_id = request.goal_info().goal_id;
    request
        .reply_async(&ros_z::action::messages::CancelGoalServiceResponse {
            return_code: 0,
            goals_canceling: vec![],
        })
        .await
}

#[allow(dead_code)]
async fn assert_manual_result_reply_api(
    request: ros_z::action::server::ResultRequestHandle<TestAction>,
) -> Result<()> {
    let _goal_id = *request.goal_id();
    request
        .reply_async(&ros_z::action::messages::GetResultResponse::<TestAction> {
            status: ros_z::action::GoalStatus::Succeeded as i8,
            result: TestResult { value: 0 },
        })
        .await
}

async fn spawn_manual_success_server(
    server: ros_z::action::server::ActionServer<TestAction>,
    expected_order: i32,
    result_value: i32,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let requested = server.receive_goal_async().await?;
        assert_eq!(requested.goal().order, expected_order);

        let executing = requested.accept().execute();
        tokio::time::sleep(Duration::from_millis(200)).await;
        executing.succeed(TestResult {
            value: result_value,
        })?;

        Ok(())
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn action_api_alignment_smoke_test() -> Result<()> {
    let (_ctx, _node, client, server) = setup_action_api_alignment().await?;
    assert_manual_action_receive_signatures(&server);

    let server_for_result = server.clone();
    let result_task = tokio::spawn(async move {
        let requested = server_for_result.receive_goal_async().await?;
        assert_eq!(requested.goal().order, 7);
        requested
            .accept()
            .execute()
            .succeed(TestResult { value: 14 })?;
        Ok::<(), zenoh::Error>(())
    });

    let goal_handle = client.send_goal_async(TestGoal { order: 7 }).await?;
    let goal_id = goal_handle.id();
    result_task.await??;

    let result = client.get_result_async(goal_id).await?;
    assert_eq!(result.value, 14);

    let server_for_cancel = server.clone();
    let cancel_task = tokio::spawn(async move {
        let requested = server_for_cancel.receive_goal_async().await?;
        assert_eq!(requested.goal().order, 9);
        let executing = requested.accept().execute();

        loop {
            if executing.try_process_cancel() {
                executing.canceled(TestResult { value: -1 })?;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok::<(), zenoh::Error>(())
    });

    let goal_handle = client.send_goal_async(TestGoal { order: 9 }).await?;
    let goal_id = goal_handle.id();
    let cancel_response = client.cancel_goal_async(goal_id).await?;
    assert_eq!(cancel_response.goals_canceling.len(), 1);

    let canceled_result = goal_handle.result_async().await?;
    assert_eq!(canceled_result.value, -1);

    cancel_task.await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_goal_handle_result_waits_for_terminal_status() -> Result<()> {
    let (_ctx, _node, client, server) = setup_action_api_alignment().await?;

    let server_task = spawn_manual_success_server(server.clone(), 11, 22).await;
    let goal_handle = client.send_goal_async(TestGoal { order: 11 }).await?;
    let goal_handle_task = tokio::task::spawn_blocking(move || goal_handle.result());
    let goal_handle_result = timeout(Duration::from_secs(2), goal_handle_task)
        .await
        .expect("blocking GoalHandle::result() timed out")
        .expect("blocking GoalHandle::result() panicked")?;
    assert_eq!(goal_handle_result.value, 22);
    server_task.await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_client_get_result_waits_for_terminal_status() -> Result<()> {
    let (_ctx, _node, client, server) = setup_action_api_alignment().await?;

    let server_task = spawn_manual_success_server(server.clone(), 12, 24).await;
    let goal_handle = client.send_goal_async(TestGoal { order: 12 }).await?;
    let goal_id = goal_handle.id();
    let client_for_result = client.clone();
    let client_result_task =
        tokio::task::spawn_blocking(move || client_for_result.get_result(goal_id));
    let client_result = timeout(Duration::from_secs(2), client_result_task)
        .await
        .expect("blocking ActionClient::get_result() timed out")
        .expect("blocking ActionClient::get_result() panicked")?;
    assert_eq!(client_result.value, 24);
    server_task.await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_goal_handle_result_returns_error_in_current_thread_runtime() -> Result<()> {
    let (_ctx, _node, client, server) = setup_action_api_alignment().await?;

    let server_task = spawn_manual_success_server(server.clone(), 13, 26).await;
    let goal_handle = client.send_goal_async(TestGoal { order: 13 }).await?;

    let join = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create current_thread runtime for regression test")
            .block_on(async move { goal_handle.result() })
    });

    let error = join
        .join()
        .expect("current_thread regression test should not panic")
        .expect_err("blocking GoalHandle::result() should error on current_thread runtimes");

    assert!(
        error.to_string().contains("current_thread"),
        "expected current_thread runtime guidance, got: {error}"
    );

    server_task.await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn manual_result_recv_replies_through_typed_handle() -> Result<()> {
    let (_ctx, _node, client, server) = setup_action_api_alignment().await?;

    let server_task = tokio::spawn(async move {
        let result_server = server.clone();
        let result_request_task =
            tokio::spawn(async move { result_server.receive_result_request_async().await });

        let requested = server.receive_goal_async().await?;
        assert_eq!(requested.goal().order, 21);
        let executing = requested.accept().execute();
        let goal_id = executing.info().goal_id;

        let request: ros_z::action::server::ResultRequestHandle<TestAction> =
            timeout(Duration::from_secs(2), result_request_task)
                .await
                .expect("timed out waiting for manual result request")??;
        assert_eq!(*request.goal_id(), goal_id);

        let response = ros_z::action::messages::GetResultResponse::<TestAction> {
            status: ros_z::action::GoalStatus::Succeeded as i8,
            result: TestResult { value: 42 },
        };
        request.reply_async(&response).await?;
        executing.succeed(TestResult { value: 42 })?;

        Ok::<(), zenoh::Error>(())
    });

    let goal_id = client.send_goal_async(TestGoal { order: 21 }).await?.id();
    let result = timeout(Duration::from_secs(2), client.get_result_async(goal_id))
        .await
        .expect("timed out waiting for client result task")?;
    assert_eq!(result.value, 42);
    server_task.await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_server_receive_result_request_returns_error_in_current_thread_runtime()
-> Result<()> {
    let (_ctx, _node, _client, server) = setup_action_api_alignment().await?;

    let join = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create current_thread runtime for regression test")
            .block_on(async move { server.receive_result_request() })
    });

    let error = match join
        .join()
        .expect("current_thread regression test should not panic")
    {
        Ok(_) => panic!("blocking ActionServer::receive_result_request() should error"),
        Err(error) => error,
    };

    let error_text = error.to_string();
    assert!(
        error_text.contains("current_thread"),
        "expected current_thread runtime guidance, got: {error_text}"
    );
    assert!(
        error_text.contains("receive_result_request_async()"),
        "expected async receive guidance, got: {error_text}"
    );

    Ok(())
}

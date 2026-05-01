use super::{isolated_context, unique_namespace};
use ros_z::{Result, define_action};
use serde::{Deserialize, Serialize};

// Define test action messages (similar to Fibonacci)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGoal {
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFeedback {
    pub progress: i32,
}

// Define the action type
pub struct TestAction;

define_action! {
    TestAction,
    action_name: "test_action",
    Goal: TestGoal,
    Result: TestResult,
    Feedback: TestFeedback,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn next_test_namespace() -> String {
        unique_namespace("action_server")
    }

    // Helper function to create test setup
    async fn setup_test() -> Result<(
        ros_z::node::Node,
        ros_z::action::client::ActionClient<TestAction>,
        ros_z::action::server::ActionServer<TestAction>,
    )> {
        let context = isolated_context("action_server").build().await?;
        let node = context
            .create_node("test_action_server_node")
            .with_namespace(next_test_namespace())
            .build()
            .await?;

        let client = node
            .create_action_client::<TestAction>("test_action_server_name")
            .build()
            .await?;

        let server = node
            .create_action_server::<TestAction>("test_action_server_name")
            .build()
            .await?;

        // Wait for discovery
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok((node, client, server))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_action_accept_new_goal() -> Result<()> {
        let (_node, client, server) = setup_test().await?;

        // Spawn server task to accept the goal
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let requested = server_clone.receive_goal_async().await?;
            assert_eq!(requested.goal.order, 10);
            let _accepted = requested.accept();
            Ok::<(), zenoh::Error>(())
        });

        // Send a goal request
        let goal_handle = client.send_goal_async(TestGoal { order: 10 }).await?;
        let goal_id = goal_handle.id();

        // Wait for server to finish
        server_task.await??;

        // Verify goal ID is valid (not all zeros)
        assert!(goal_id.is_valid());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_action_server_notify_goal_done() -> Result<()> {
        let (_node, client, server) = setup_test().await?;

        // Spawn server task to handle the goal
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let requested = server_clone.receive_goal_async().await?;
            let accepted = requested.accept();
            let executing = accepted.execute();
            executing.succeed(TestResult { value: 42 })?;
            Ok::<(), zenoh::Error>(())
        });

        // Send goal and get result
        let goal_handle = client.send_goal_async(TestGoal { order: 10 }).await?;

        // Wait for server to finish
        server_task.await??;

        // Get the result to verify completion
        let result = goal_handle.result_async().await?;
        assert_eq!(result.value, 42);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_action_process_cancel_request() -> Result<()> {
        let (_node, client, server) = setup_test().await?;

        // Spawn server task to handle the goal and cancel
        let server_clone = server.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let server_task = tokio::spawn(async move {
            let requested = server_clone.receive_goal_async().await?;
            let executing = requested.accept().execute();
            let goal_id = executing.info().goal_id;

            // Signal that goal is accepted
            let _ = tx.send(());

            loop {
                if executing.try_process_cancel() {
                    executing.canceled(TestResult { value: -1 })?;
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }

            Ok::<_, zenoh::Error>(goal_id)
        });

        // Wait for server to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Send and accept a goal
        let goal_handle = client.send_goal_async(TestGoal { order: 10 }).await?;

        // Wait for server to accept
        rx.await.expect("server task ended prematurely");

        // Send cancel request
        let cancel_response = goal_handle.cancel_async().await?;
        assert_eq!(cancel_response.goals_canceling.len(), 1);

        // Wait for server to process cancel
        let canceled_goal_id =
            tokio::time::timeout(tokio::time::Duration::from_secs(2), server_task)
                .await
                .expect("timeout waiting for server task")??;
        assert_eq!(canceled_goal_id, goal_handle.id());

        // Basic verification that cancel was received
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn async_build_preserves_background_action_tasks() -> Result<()> {
        let (_node, client, server) = setup_test().await?;

        assert!(client.wait_for_server_async(Duration::from_secs(5)).await);

        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let requested = server_clone.receive_goal_async().await?;
            let executing = requested.accept().execute();
            executing.publish_feedback(TestFeedback { progress: 7 })?;
            executing.succeed(TestResult { value: 42 })
        });

        let mut goal = client.send_goal_async(TestGoal { order: 1 }).await?;
        let mut feedback = goal.feedback().expect("feedback receiver should exist");
        let received = tokio::time::timeout(Duration::from_secs(5), feedback.recv())
            .await
            .expect("feedback timed out; background tasks likely stopped")
            .expect("feedback stream closed before feedback arrived");
        assert_eq!(received.progress, 7);
        let result = goal.result_async().await?;
        assert_eq!(result.value, 42);

        server_task
            .await
            .expect("server task panicked while handling action")?;
        Ok(())
    }
}

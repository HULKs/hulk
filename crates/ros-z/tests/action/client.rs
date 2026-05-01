use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use super::{isolated_context, unique_action_name, unique_namespace};
use ros_z::{Result, define_action};
use serde::{Deserialize, Serialize};
use serial_test::serial;

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

// Helper function to create test setup
async fn setup_test_base() -> Result<(ros_z::node::Node,)> {
    let context = isolated_context("action_client").build().await?;
    let node = context
        .create_node("test_action_client_node")
        .with_namespace(unique_namespace("action_client"))
        .build()
        .await?;

    // Wait for discovery
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok((node,))
}

// Helper function to create test setup with client
async fn setup_test_with_client() -> Result<(
    ros_z::node::Node,
    std::sync::Arc<ros_z::action::client::ActionClient<TestAction>>,
    String,
)> {
    let (node,) = setup_test_base().await?;
    let action_name = unique_action_name("test_action_client_name");

    let client = Arc::new(
        node.create_action_client::<TestAction>(&action_name)
            .build()
            .await?,
    );

    Ok((node, client, action_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_action_server_is_available() -> Result<()> {
        let (node, _client, action_name) = setup_test_with_client().await?;

        // Create a server to verify availability detection
        let _server = node
            .create_action_server::<TestAction>(&action_name)
            .build()
            .await?;

        // Wait for discovery
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        // Verify server is discoverable through graph
        let server_names_types = node
            .graph()
            .get_action_server_names_and_types_by_node(ros_z::entity::node_key(node.node_entity()));
        assert!(!server_names_types.is_empty());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_action_client_get_action_name() -> Result<()> {
        let (node, _client, action_name) = setup_test_with_client().await?;

        // Verify action name through graph introspection
        let client_names_types = node
            .graph()
            .get_action_client_names_and_types_by_node(ros_z::entity::node_key(node.node_entity()));

        // Should find the action client with the expected name
        let action_found = client_names_types
            .iter()
            .any(|(name, _)| name == &action_name);
        assert!(action_found);

        Ok(())
    }

    #[serial]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_action_client_wait_for_server_async() -> Result<()> {
        let context = isolated_context("action_wait_client").build().await?;
        let action_name = unique_action_name("wait_for_action");
        let client_node = context.create_node("action_wait_client").build().await?;
        let client = client_node
            .create_action_client::<TestAction>(&action_name)
            .build()
            .await?;

        let server_ctx = context.clone();
        let server_task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            let server_node = server_ctx.create_node("action_wait_server").build().await?;
            let _server = server_node
                .create_action_server::<TestAction>(&action_name)
                .build()
                .await?;

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            Result::<()>::Ok(())
        });

        assert!(
            client
                .wait_for_server_async(std::time::Duration::from_secs(3))
                .await
        );
        server_task.await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn send_goal_with_timeout_async_returns_timeout_without_goal_response() -> Result<()> {
        let timeout = Duration::from_millis(100);
        let context = isolated_context("goal_timeout_client_async")
            .build()
            .await?;
        let action_name = unique_action_name("missing_goal_timeout_async");
        let node = context
            .create_node("goal_timeout_client_async")
            .build()
            .await?;
        let _server = node
            .create_action_server::<TestAction>(&action_name)
            .build()
            .await?;
        let client = node
            .create_action_client::<TestAction>(&action_name)
            .build()
            .await?;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let started = Instant::now();
        let result = client
            .send_goal_with_timeout_async(TestGoal { order: 1 }, timeout)
            .await;
        let elapsed = started.elapsed();

        let error = match result {
            Ok(_) => panic!("goal unexpectedly succeeded without a goal response"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("Service call timed out"),
            "expected timeout error, got: {error}"
        );
        assert!(elapsed >= timeout, "returned before timeout: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(2),
            "timeout took too long: {elapsed:?}"
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn send_goal_with_timeout_returns_timeout_without_goal_response() -> Result<()> {
        let timeout = Duration::from_millis(100);
        let context = isolated_context("goal_timeout_client_blocking")
            .build()
            .await?;
        let action_name = unique_action_name("missing_goal_timeout_blocking");
        let node = context
            .create_node("goal_timeout_client_blocking")
            .build()
            .await?;
        let _server = node
            .create_action_server::<TestAction>(&action_name)
            .build()
            .await?;
        let client = node
            .create_action_client::<TestAction>(&action_name)
            .build()
            .await?;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let (result, elapsed) = tokio::task::spawn_blocking(move || {
            let started = Instant::now();
            let result = client.send_goal_with_timeout(TestGoal { order: 1 }, timeout);
            (result, started.elapsed())
        })
        .await?;

        let error = match result {
            Ok(_) => panic!("goal unexpectedly succeeded without a goal response"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("Service call timed out"),
            "expected timeout error, got: {error}"
        );
        assert!(elapsed >= timeout, "returned before timeout: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(2),
            "timeout took too long: {elapsed:?}"
        );
        Ok(())
    }

    // TODO: Additional tests would cover:
    // - Server availability checks
    // - Introspection configuration
    // - Fault injection tests (memory allocation failures)
    // These would require more complex setup and are deferred for now
}

use super::{isolated_context, unique_action_name, unique_namespace};
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

// Helper function to create test setup with multiple clients and servers
async fn setup_multiple_clients_single_server(
    num_clients: usize,
) -> Result<(
    ros_z::node::Node,
    ros_z::action::server::ActionServer<TestAction>,
    Vec<ros_z::action::client::ActionClient<TestAction>>,
)> {
    let context = isolated_context("interaction").build().await?;
    let action_name = unique_action_name("test_interaction_action");
    let node = context
        .create_node("test_interaction_node")
        .with_namespace(unique_namespace("interaction"))
        .build()
        .await?;

    let server = node
        .create_action_server::<TestAction>(&action_name)
        .build()
        .await?;

    let mut clients = Vec::new();
    for _i in 0..num_clients {
        let client = node
            .create_action_client::<TestAction>(&action_name)
            .build()
            .await?;
        clients.push(client);
    }

    // Wait for discovery
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok((node, server, clients))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTION_INTERACTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_multiple_clients_single_server() -> Result<()> {
        let (_node, server, clients) = setup_multiple_clients_single_server(3).await?;

        // Set up server handler
        let _server_handle = server.with_handler(|executing| async move {
            let order = executing.goal.order;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            executing.succeed(TestResult { value: order * 2 }).unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for client in &clients {
            assert!(
                tokio::time::timeout(
                    ACTION_INTERACTION_TIMEOUT,
                    client.wait_for_server_async(std::time::Duration::from_secs(5)),
                )
                .await
                .expect("timed out while waiting for action server discovery"),
                "action server should be discoverable before sending goals"
            );
        }

        // Send goals from multiple clients
        let mut handles = Vec::new();
        for (i, client) in clients.iter().enumerate() {
            let client_clone = client.clone();
            let goal_order = (i + 1) as i32 * 10;
            let handle = tokio::spawn(async move {
                let goal_handle = tokio::time::timeout(
                    ACTION_INTERACTION_TIMEOUT,
                    client_clone.send_goal_async(TestGoal { order: goal_order }),
                )
                .await
                .expect("timed out sending goal")
                .unwrap();
                tokio::time::timeout(ACTION_INTERACTION_TIMEOUT, goal_handle.result_async())
                    .await
                    .expect("timed out waiting for action result")
                    .unwrap()
            });
            handles.push(handle);
        }

        // Wait for all results and verify
        for (i, handle) in handles.into_iter().enumerate() {
            let result = tokio::time::timeout(ACTION_INTERACTION_TIMEOUT, handle)
                .await
                .expect("timed out waiting for client task")
                .unwrap();
            let expected_value = (i + 1) as i32 * 20; // order * 2
            assert_eq!(
                result.value, expected_value,
                "Result mismatch for client {}",
                i
            );
        }

        Ok(())
    }

    // TODO: Additional tests would cover:
    // - Concurrent goal execution
    // - Server discovery and availability
    // - Load balancing across multiple servers
    // These would require more complex setup and are deferred for now
}

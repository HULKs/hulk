//! Action communication tests
//!
//! These tests verify the low-level protocol communication between action clients and servers.

use std::sync::atomic::Ordering;
use std::time::Duration;

use super::{isolated_context, unique_namespace};
use ros_z::{Result, define_action};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

// Test action messages (equivalent to test_msgs/action/Fibonacci)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestGoal {
    pub order: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestFeedback {
    pub sequence: Vec<i32>,
}

pub struct TestAction;

define_action! {
    TestAction,
    action_name: "test_action_comm",
    Goal: TestGoal,
    Result: TestResult,
    Feedback: TestFeedback,
}

/// Helper to setup test fixtures
async fn setup_test() -> Result<(
    ros_z::context::Context,
    ros_z::node::Node,
    ros_z::action::client::ActionClient<TestAction>,
    ros_z::action::server::ActionServer<TestAction>,
)> {
    let context = isolated_context("action_comm").build().await?;
    let namespace = unique_namespace("action_comm");
    let node = context
        .create_node("test_action_comm_node")
        .with_namespace(namespace)
        .build()
        .await?;

    let server = node
        .create_action_server::<TestAction>("test_action_comm")
        .build()
        .await?;

    let client = node
        .create_action_client::<TestAction>("test_action_comm")
        .build()
        .await?;

    // Longer delay to allow Zenoh discovery
    // Server needs to be fully initialized before client can connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok((context, node, client, server))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests basic goal request/response communication
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_valid_goal_comm() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        // Spawn server processing task
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            // Server should receive the goal request
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;

            // Extract values before accepting (which moves requested)
            let goal_order = requested.goal.order;
            let goal_id = requested.info.goal_id;

            // Accept the goal (sends goal response)
            let _accepted = requested.accept();

            Ok::<_, zenoh::Error>((goal_order, goal_id))
        });

        // Create and send goal request
        let outgoing_goal = TestGoal { order: 10 };
        let goal_handle = timeout(
            Duration::from_secs(5),
            client.send_goal_async(outgoing_goal.clone()),
        )
        .await
        .expect("timeout sending goal")?;

        // Wait for server to process
        let (goal_order, goal_id) = server_task.await.expect("server task failed")?;

        // Verify goal data matches
        assert_eq!(goal_order, outgoing_goal.order);
        assert_eq!(goal_id, goal_handle.id());

        // Client should receive the acceptance response
        // This happens automatically in send_goal, but we can verify the goal is valid
        assert_ne!(goal_handle.id(), ros_z::action::GoalId::default());

        // Clean shutdown
        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    /// Tests cancel request/response communication
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_valid_cancel_comm() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        // Spawn both goal and cancel handling together
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;
            let goal_id = requested.info.goal_id;
            let accepted = requested.accept();
            let _executing = accepted.execute();

            // Server should receive cancel request
            let cancel_request =
                timeout(Duration::from_secs(5), server_clone.receive_cancel_async())
                    .await
                    .expect("timeout receiving cancel")?;

            // Verify cancel request has correct goal ID
            assert_eq!(cancel_request.goal_info().goal_id, goal_id);

            // Send cancel response
            let cancel_resp = ros_z::action::messages::CancelGoalServiceResponse {
                return_code: 0, // ERROR_NONE
                goals_canceling: vec![ros_z::action::GoalInfo {
                    goal_id,
                    stamp: ros_z::action::Time::zero(), // Current time in nanoseconds
                }],
            };

            // Respond to the cancel request
            cancel_request.reply_async(&cancel_resp).await?;

            Ok::<_, zenoh::Error>(())
        });

        // Send and accept a goal first
        let goal = TestGoal { order: 10 };
        let goal_handle = timeout(Duration::from_secs(5), client.send_goal_async(goal))
            .await
            .expect("timeout sending goal")?;

        // Send cancel request
        let cancel_response = timeout(Duration::from_secs(5), goal_handle.cancel_async())
            .await
            .expect("timeout sending cancel")?;

        // Wait for server to process everything
        server_task.await.expect("server task failed")?;

        // Verify client received response
        assert_eq!(cancel_response.return_code, 0); // ERROR_NONE

        // Clean shutdown
        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    /// Tests result request/response communication
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_valid_result_comm() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        // Spawn server processing
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;
            let accepted = requested.accept();
            let executing = accepted.execute();

            // Complete the goal with result
            let outgoing_result = TestResult { value: 42 };
            executing.succeed(outgoing_result.clone())?;

            Ok::<_, zenoh::Error>(outgoing_result)
        });

        // Send goal
        let goal = TestGoal { order: 10 };
        let goal_handle = timeout(Duration::from_secs(5), client.send_goal_async(goal))
            .await
            .expect("timeout sending goal")?;

        let outgoing_result = server_task.await.expect("server task failed")?;

        // Client requests result
        let incoming_result = timeout(Duration::from_secs(5), goal_handle.result_async())
            .await
            .expect("timeout getting result")?;

        // Verify result data matches
        assert_eq!(incoming_result.value, outgoing_result.value);

        // Clean shutdown
        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    /// Tests feedback publishing/subscription
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_valid_feedback_comm() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        // Spawn server processing first
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;
            let accepted = requested.accept();
            let executing = accepted.execute();

            // Publish feedback
            let outgoing_feedback = TestFeedback {
                sequence: vec![0, 1, 1, 2, 3, 5, 8, 13],
            };
            executing.publish_feedback(outgoing_feedback.clone())?;

            // Wait a bit for client to receive feedback
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Complete goal
            executing.succeed(TestResult { value: 13 })?;

            Ok::<_, zenoh::Error>(outgoing_feedback)
        });

        // Send goal
        let goal = TestGoal { order: 10 };
        let mut goal_handle = timeout(Duration::from_secs(5), client.send_goal_async(goal))
            .await
            .expect("timeout sending goal")?;

        // Get feedback stream after goal is sent
        let mut feedback_rx = goal_handle
            .feedback()
            .expect("failed to get feedback stream");

        // Client receives feedback
        let incoming_feedback = timeout(Duration::from_secs(5), feedback_rx.recv())
            .await
            .expect("timeout receiving feedback")
            .expect("feedback channel closed");

        let outgoing_feedback = server_task.await.expect("server task failed")?;

        // Verify feedback data matches
        assert_eq!(incoming_feedback.sequence, outgoing_feedback.sequence);

        // Clean shutdown
        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    /// Tests status publishing/subscription
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_valid_status_comm() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        // Spawn server processing task
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            // Accept and execute goal on server
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;
            let accepted = requested.accept();

            // Give client time to observe Accepted status
            tokio::time::sleep(Duration::from_millis(300)).await;

            let executing = accepted.execute();

            // Wait for client to observe EXECUTING status
            tokio::time::sleep(Duration::from_millis(300)).await;

            // Complete the goal
            executing.succeed(TestResult { value: 42 })?;

            Ok::<_, zenoh::Error>(())
        });

        // Send goal
        let goal = TestGoal { order: 10 };
        let goal_handle = timeout(Duration::from_secs(5), client.send_goal_async(goal))
            .await
            .expect("timeout sending goal")?;

        // Watch status for this goal
        let mut status_watch = client
            .status_watch(goal_handle.id())
            .expect("failed to watch status");

        // Wait for status updates - should eventually see Succeeded
        // Status transitions: Accepted -> Executing -> Succeeded
        let mut final_status = *status_watch.borrow();
        let mut iterations = 0;
        while final_status != ros_z::action::GoalStatus::Succeeded && iterations < 10 {
            match timeout(Duration::from_millis(600), status_watch.changed()).await {
                Ok(Ok(_)) => {
                    final_status = *status_watch.borrow();
                }
                Ok(Err(_)) => break, // Watch closed
                Err(_) => {
                    // Timeout - break to check status
                    final_status = *status_watch.borrow();
                    break;
                }
            }
            iterations += 1;
        }

        // Verify we reached Succeeded status
        assert_eq!(
            final_status,
            ros_z::action::GoalStatus::Succeeded,
            "Expected Succeeded status after {} iterations",
            iterations
        );

        // Wait for server task to complete
        server_task.await.expect("server task failed")?;

        // Clean shutdown
        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    /// Regression test: cancel for goal B must not be silently dropped when goal A polls first.
    ///
    /// With the old `try_process_cancel` implementation, if goal A polled the shared cancel
    /// queue and found a request for goal B, it would discard the message (ID mismatch) and
    /// return false. Goal B's subsequent poll would find an empty queue and also return false —
    /// the cancel was lost.
    ///
    /// The fix introduces `CancelDispatcher`: both handles call `drain()` which routes every
    /// pending message to the correct per-goal channel, so each handle only sees its own cancels.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_try_process_cancel_multi_goal() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        // Send two goals and get handles to both
        let goal1 = TestGoal { order: 1 };
        let goal2 = TestGoal { order: 2 };

        // Server must accept each goal immediately so the client can proceed to send the next one.
        // (client.send_goal blocks until the server sends an accept response)
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let req1 = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal 1")?;
            // Accept immediately so the client unblocks and sends goal 2
            let handle1 = req1.accept().execute();

            let req2 = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal 2")?;
            let handle2 = req2.accept().execute();

            Ok::<_, zenoh::Error>((handle1, handle2))
        });

        let goal_handle1 = timeout(Duration::from_secs(5), client.send_goal_async(goal1))
            .await
            .expect("timeout sending goal 1")?;
        let goal_handle2 = timeout(Duration::from_secs(5), client.send_goal_async(goal2))
            .await
            .expect("timeout sending goal 2")?;

        let (handle1, handle2) = server_task.await.expect("server task failed")?;

        // Spawn a task that sends the cancel for goal2 and then awaits the result.
        // We move goal_handle2 into this task so that cancel() and result() can be called
        // in sequence without a borrow/move conflict in the outer scope.
        let client_task = tokio::spawn(async move {
            let cancel_response = timeout(Duration::from_secs(5), goal_handle2.cancel_async())
                .await
                .expect("timeout awaiting cancel response")?;
            // After cancel is confirmed, fetch the final result
            let result = timeout(Duration::from_secs(5), goal_handle2.result_async())
                .await
                .expect("timeout getting result 2")?;
            Ok::<_, zenoh::Error>((cancel_response, result))
        });

        // Give the cancel request time to arrive on the server side
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Goal 1 polls first — must NOT steal the cancel intended for goal 2
        assert!(
            !handle1.try_process_cancel(),
            "handle1.try_process_cancel() should return false (cancel was for goal2)"
        );

        // Goal 2 polls second — must see the cancel routed to its channel
        assert!(
            handle2.try_process_cancel(),
            "handle2.try_process_cancel() should return true (cancel was for goal2)"
        );

        // Complete both goals so the client task can finish
        handle1.succeed(TestResult { value: 1 })?;
        handle2.canceled(TestResult { value: 2 })?;

        let (cancel_response, _) = client_task.await.expect("client task panicked")?;
        assert_eq!(cancel_response.return_code, 0);

        let _ = timeout(Duration::from_secs(5), goal_handle1.result_async())
            .await
            .expect("timeout getting result 1")?;

        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_is_cancel_requested_processes_polling_cancel() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        let server_clone = server.clone();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let server_task = tokio::spawn(async move {
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;
            let executing = requested.accept().execute();

            ready_tx
                .send(())
                .expect("client task should wait for readiness");

            let cancel_seen = timeout(Duration::from_secs(2), async {
                loop {
                    if executing.is_cancel_requested() {
                        break true;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap_or(false);

            assert!(
                cancel_seen,
                "is_cancel_requested() should observe queued polling-mode cancel"
            );

            executing.canceled(TestResult { value: -1 })?;
            Ok::<_, zenoh::Error>(())
        });

        let goal_handle = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 1 }),
        )
        .await
        .expect("timeout sending goal")?;
        ready_rx.await.expect("server task ended before readiness");

        let cancel_response = timeout(Duration::from_secs(5), goal_handle.cancel_async())
            .await
            .expect("timeout awaiting cancel response")?;
        assert_eq!(cancel_response.return_code, 0);

        server_task.await.expect("server task failed")?;

        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_is_cancel_requested_replies_to_repeated_cancel_after_flag_set() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        let server_clone = server.clone();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (first_seen_tx, first_seen_rx) = tokio::sync::oneshot::channel();
        let (burst_sent_tx, burst_sent_rx) = tokio::sync::oneshot::channel();
        let server_task = tokio::spawn(async move {
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;
            let executing = requested.accept().execute();

            ready_tx
                .send(())
                .expect("client task should wait for readiness");

            timeout(Duration::from_secs(2), async {
                loop {
                    if executing.is_cancel_requested() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("timeout waiting for first cancel");
            first_seen_tx
                .send(())
                .expect("client task should wait for first cancel");

            burst_sent_rx
                .await
                .expect("client task ended before repeated cancels were sent");
            assert!(
                executing.is_cancel_requested(),
                "repeated cancels should still leave the goal cancel-requested"
            );

            executing.canceled(TestResult { value: -1 })?;
            Ok::<_, zenoh::Error>(())
        });

        let goal_handle = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 1 }),
        )
        .await
        .expect("timeout sending goal")?;
        let goal_id = goal_handle.id();
        ready_rx.await.expect("server task ended before readiness");

        let first_cancel = timeout(Duration::from_secs(5), goal_handle.cancel_async())
            .await
            .expect("timeout awaiting first cancel response")?;
        assert_eq!(first_cancel.return_code, 0);
        first_seen_rx
            .await
            .expect("server task ended before first cancel was observed");

        let repeated_cancel_tasks: Vec<_> = (0..6)
            .map(|_| {
                let client = client.clone();
                tokio::spawn(async move { client.cancel_goal_async(goal_id).await })
            })
            .collect();
        tokio::time::sleep(Duration::from_millis(200)).await;
        burst_sent_tx
            .send(())
            .expect("server task should wait for repeated cancels");

        for cancel_task in repeated_cancel_tasks {
            let cancel_response = timeout(Duration::from_secs(5), cancel_task)
                .await
                .expect("timeout awaiting repeated cancel task")
                .expect("repeated cancel task panicked")?;
            assert_eq!(cancel_response.return_code, 0);
        }

        server_task.await.expect("server task failed")?;

        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_termination_replies_to_routed_pending_cancels() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let req1 = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal 1")?;
            let handle1 = req1.accept().execute();

            let req2 = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal 2")?;
            let handle2 = req2.accept().execute();

            Ok::<_, zenoh::Error>((handle1, handle2))
        });

        let goal_handle1 = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 1 }),
        )
        .await
        .expect("timeout sending goal 1")?;
        let goal_handle2 = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 2 }),
        )
        .await
        .expect("timeout sending goal 2")?;
        let goal2_id = goal_handle2.id();

        let (handle1, handle2) = server_task.await.expect("server task failed")?;
        let cancel_tasks: Vec<_> = (0..3)
            .map(|_| {
                let client = client.clone();
                tokio::spawn(async move { client.cancel_goal_async(goal2_id).await })
            })
            .collect();

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !handle1.try_process_cancel(),
            "goal 1 should only route cancels for goal 2"
        );

        handle1.succeed(TestResult { value: 1 })?;
        handle2.canceled(TestResult { value: 2 })?;

        for cancel_task in cancel_tasks {
            let cancel_response = timeout(Duration::from_secs(5), cancel_task)
                .await
                .expect("timeout awaiting routed pending cancel")
                .expect("cancel task panicked")?;
            assert_eq!(cancel_response.return_code, 0);
        }

        let _ = timeout(Duration::from_secs(5), goal_handle1.result_async())
            .await
            .expect("timeout awaiting goal 1 result")?;
        let result2 = timeout(Duration::from_secs(5), goal_handle2.result_async())
            .await
            .expect("timeout awaiting goal 2 result")?;
        assert_eq!(result2.value, 2);

        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_termination_replies_to_shared_queue_cancel() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let requested = timeout(Duration::from_secs(5), server_clone.receive_goal_async())
                .await
                .expect("timeout receiving goal")?;
            Ok::<_, zenoh::Error>(requested.accept().execute())
        });

        let goal_handle = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 1 }),
        )
        .await
        .expect("timeout sending goal")?;
        let executing = server_task.await.expect("server task failed")?;

        let goal_id = goal_handle.id();
        let cancel_client = client.clone();
        let cancel_task =
            tokio::spawn(async move { cancel_client.cancel_goal_async(goal_id).await });

        tokio::time::sleep(Duration::from_millis(200)).await;
        executing.canceled(TestResult { value: -1 })?;

        let cancel_response = timeout(Duration::from_secs(5), cancel_task)
            .await
            .expect("timeout awaiting shared queue cancel")
            .expect("cancel task panicked")?;
        assert_eq!(cancel_response.return_code, 1);

        let result = timeout(Duration::from_secs(5), goal_handle.result_async())
            .await
            .expect("timeout awaiting result")?;
        assert_eq!(result.value, -1);

        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_driver_cancel_after_early_result_request() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        let _server_handle = server.with_handler(|executing| async move {
            loop {
                if executing.is_cancel_requested() {
                    executing
                        .canceled(TestResult { value: -1 })
                        .expect("failed to cancel goal");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        let goal_handle = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 1 }),
        )
        .await
        .expect("timeout sending goal")?;
        let goal_id = goal_handle.id();

        let result_client = client.clone();
        let result_task =
            tokio::spawn(async move { result_client.get_result_async(goal_id).await });
        tokio::time::sleep(Duration::from_millis(200)).await;

        let cancel_response = timeout(Duration::from_secs(2), client.cancel_goal_async(goal_id))
            .await
            .expect("timeout awaiting cancel response")?;
        assert_eq!(cancel_response.return_code, 0);

        let result = timeout(Duration::from_secs(5), result_task)
            .await
            .expect("timeout awaiting result task")
            .expect("result task panicked")?;
        assert_eq!(result.value, -1);

        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_driver_services_cancel_while_result_request_waits() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        let handler_ready = std::sync::Arc::new(tokio::sync::Notify::new());
        let handler_ready_for_task = handler_ready.clone();
        let finish = std::sync::Arc::new(tokio::sync::Notify::new());
        let handler_finish = finish.clone();
        let _server_handle = server.with_handler(move |executing| {
            let finish = handler_finish.clone();
            let handler_ready = handler_ready_for_task.clone();
            async move {
                handler_ready.notify_one();
                finish.notified().await;
                executing
                    .canceled(TestResult { value: -1 })
                    .expect("failed to cancel goal");
            }
        });

        let goal_handle = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 1 }),
        )
        .await
        .expect("timeout sending goal")?;
        let goal_id = goal_handle.id();
        timeout(Duration::from_secs(5), handler_ready.notified())
            .await
            .expect("timeout waiting for handler readiness");

        let result_client = client.clone();
        let result_task =
            tokio::spawn(async move { result_client.get_result_async(goal_id).await });
        tokio::time::sleep(Duration::from_millis(200)).await;

        let cancel_response = timeout(Duration::from_secs(2), client.cancel_goal_async(goal_id))
            .await
            .expect("timeout awaiting cancel response")?;
        assert_eq!(cancel_response.return_code, 0);

        finish.notify_one();
        let result = timeout(Duration::from_secs(5), result_task)
            .await
            .expect("timeout awaiting result task")
            .expect("result task panicked")?;
        assert_eq!(result.value, -1);

        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_driver_goal_polling_does_not_steal_other_goal_cancel() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        let ready_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let both_ready = std::sync::Arc::new(tokio::sync::Notify::new());
        let finish_goal_a = std::sync::Arc::new(tokio::sync::Notify::new());
        let finish_goal_b = std::sync::Arc::new(tokio::sync::Notify::new());

        let handler_ready_count = ready_count.clone();
        let handler_both_ready = both_ready.clone();
        let handler_finish_goal_a = finish_goal_a.clone();
        let handler_finish_goal_b = finish_goal_b.clone();
        let _server_handle = server.with_handler(move |executing| {
            let ready_count = handler_ready_count.clone();
            let both_ready = handler_both_ready.clone();
            let finish_goal_a = handler_finish_goal_a.clone();
            let finish_goal_b = handler_finish_goal_b.clone();
            async move {
                if ready_count.fetch_add(1, Ordering::Relaxed) + 1 == 2 {
                    both_ready.notify_one();
                }

                if executing.goal().order == 1 {
                    loop {
                        let _ = executing.is_cancel_requested();
                        if tokio::time::timeout(Duration::from_millis(10), finish_goal_a.notified())
                            .await
                            .is_ok()
                        {
                            executing
                                .succeed(TestResult { value: 1 })
                                .expect("failed to finish goal A");
                            break;
                        }
                    }
                } else {
                    finish_goal_b.notified().await;
                    executing
                        .canceled(TestResult { value: -2 })
                        .expect("failed to cancel goal B");
                }
            }
        });

        let goal_a = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 1 }),
        )
        .await
        .expect("timeout sending goal A")?;
        let goal_b = timeout(
            Duration::from_secs(5),
            client.send_goal_async(TestGoal { order: 2 }),
        )
        .await
        .expect("timeout sending goal B")?;

        timeout(Duration::from_secs(5), both_ready.notified())
            .await
            .expect("timeout waiting for both handlers");
        let cancel_response = timeout(Duration::from_secs(2), goal_b.cancel_async())
            .await
            .expect("timeout awaiting goal B cancel response")?;
        assert_eq!(cancel_response.return_code, 0);

        finish_goal_b.notify_one();
        let result_b = timeout(Duration::from_secs(5), goal_b.result_async())
            .await
            .expect("timeout awaiting goal B result")?;
        assert_eq!(result_b.value, -2);

        finish_goal_a.notify_one();
        let result_a = timeout(Duration::from_secs(5), goal_a.result_async())
            .await
            .expect("timeout awaiting goal A result")?;
        assert_eq!(result_a.value, 1);

        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }

    /// Tests handling multiple concurrent goals
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_multiple_goals_comm() -> Result<()> {
        let (_ctx, _node, client, server) = setup_test().await?;

        // Spawn server processing
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            // Server processes all goals
            for i in 0..3 {
                let requested = timeout(Duration::from_secs(2), server_clone.receive_goal_async())
                    .await
                    .expect("timeout receiving goal")?;
                assert_eq!(requested.goal.order, i * 10);

                let accepted = requested.accept();
                let executing = accepted.execute();
                executing.succeed(TestResult { value: i * 100 })?;
            }

            Ok::<_, zenoh::Error>(())
        });

        // Send multiple goals
        let mut goal_handles = vec![];
        for i in 0..3 {
            let goal = TestGoal { order: i * 10 };
            let handle = timeout(Duration::from_secs(2), client.send_goal_async(goal))
                .await
                .expect("timeout sending goal")?;
            goal_handles.push(handle);
        }

        // Wait for server to process all goals
        server_task.await.expect("server task failed")?;

        // Verify all results
        for (i, handle) in goal_handles.into_iter().enumerate() {
            let result = timeout(Duration::from_secs(2), handle.result_async())
                .await
                .expect("timeout getting result")?;
            assert_eq!(result.value, i as i32 * 100);
        }

        // Clean shutdown
        drop(server);
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(())
    }
}

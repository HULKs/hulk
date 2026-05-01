//! Unified driver loop for action server event handling.
//!
//! This module provides a single event loop that handles all server-side
//! action protocol events (goal requests, cancel requests, result requests)
//! in a sequential, race-condition-free manner.

use std::{
    future::Future,
    marker::PhantomData,
    sync::{Arc, Weak},
    time::Duration,
};

use tokio::{task::JoinSet, time};
use tokio_util::sync::CancellationToken;

use super::{
    Action, GoalInfo,
    messages::*,
    server::{
        ActionServer, Executing, GoalHandle, InnerServer, Requested, build_cancel_response,
        decode_query_message, malformed_cancel_response, query_attachment, reply_to_cancel_query,
        reply_with_attachment,
    },
    state::ServerGoalState,
};

/// Runs the unified driver loop for an action server with automatic goal handling.
///
/// This function consolidates all protocol logic into a single event loop,
/// eliminating race conditions and reducing task overhead.
///
/// # Arguments
///
/// * `weak_inner` - Weak reference to the inner server state
/// * `shutdown` - Cancellation token to stop the driver loop
/// * `handler` - Callback to execute goals automatically
pub(crate) async fn run_driver_loop<A, F, Fut>(
    weak_inner: Weak<InnerServer<A>>,
    shutdown: CancellationToken,
    handler: F,
) where
    A: Action,
    F: Fn(GoalHandle<A, Executing>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tracing::debug!("Action Server Driver Loop Started");

    // Try to upgrade the weak reference once at the start
    let Some(inner) = weak_inner.upgrade() else {
        tracing::debug!("Server already dropped, not starting driver loop");
        return;
    };

    let handler = Arc::new(handler);

    // Create a timer for periodic expiration checking (every 1 second)
    let mut expiration_timer = time::interval(Duration::from_secs(1));
    expiration_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    // STRUCTURED CONCURRENCY: Track all spawned goal tasks here
    let mut goal_tasks = JoinSet::new();

    loop {
        tokio::select! {
            // 1. Priority: Shutdown
            _ = shutdown.cancelled() => {
                tracing::debug!("Shutdown signal received. Aborting all goal tasks.");
                // This sends a cancellation signal to all running futures in the set
                goal_tasks.abort_all();
                break;
            }

            // 2. Reap Finished Tasks (Zombie Prevention)
            // This line is crucial. It removes finished tasks from memory.
            Some(res) = goal_tasks.join_next() => {
                if let Err(e) = res {
                    if e.is_cancelled() {
                        tracing::debug!("Goal task was cancelled");
                    } else if e.is_panic() {
                        tracing::error!("Goal task panicked!");
                    }
                }
            }

            // 3. Goal Expiration Timer
            _ = expiration_timer.tick() => {
                // Check for expired goals and clean them up
                let server = ActionServer::from_inner(Arc::clone(&inner));
                let expired_goals = server.expire_goals();
                if !expired_goals.is_empty() {
                    tracing::debug!("Expired {} goals: {:?}", expired_goals.len(), expired_goals);
                }
            }

            // 4. New Goal Requests
            query = inner.goal_server.queue().recv_async() => {
                let inner = inner.clone();
                let handler = handler.clone();

                // Spawn into the SET, not globally detached
                goal_tasks.spawn(async move {
                    // This is now safe. If it hangs, abort_all() kills it.
                    if let Err(error) = handle_goal_request(inner, query, handler).await {
                        tracing::warn!("Failed to handle action goal request: {}", error);
                    }
                });
            }

            // 5. Cancel Requests
            query = inner.cancel_server.queue().recv_async() => {
                handle_cancel_request(&inner, query).await;
            }

            // 6. Result Requests
            query = inner.result_server.queue().recv_async() => {
                let inner = inner.clone();
                goal_tasks.spawn(async move {
                    handle_result_request(&inner, query).await;
                });
            }
        }
    }

    // Ensure everything is dead before we exit
    while goal_tasks.join_next().await.is_some() {}
    tracing::debug!("Action Server Driver Loop Stopped");
}

/// Handles incoming goal requests.
async fn handle_goal_request<A, F, Fut>(
    inner: Arc<InnerServer<A>>,
    query: zenoh::query::Query,
    handler: Arc<F>,
) -> zenoh::Result<()>
where
    A: Action,
    F: Fn(GoalHandle<A, Executing>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tracing::debug!("Received goal request");
    let request = match decode_query_message::<SendGoalRequest<A>>(&query) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to deserialize goal request: {}", e);
            return Ok(());
        }
    };
    let reply_attachment = match query_attachment(&query) {
        Ok(attachment) => attachment,
        Err(error) => {
            tracing::warn!("Failed to decode goal request attachment: {}", error);
            return Ok(());
        }
    };

    // Create a temporary ActionServer handle for the goal handle
    // This is safe because we're just passing it to the goal handler
    let server = ActionServer::from_inner(Arc::clone(&inner));

    let requested = GoalHandle {
        goal: request.goal,
        info: GoalInfo::new(request.goal_id),
        server,
        query: Some(query),
        reply_attachment,
        cancel_flag: None,
        cancel_rx: None,
        _state: PhantomData::<Requested>,
    };

    let accepted = requested.try_accept()?;
    let executing = accepted.execute_driver();

    // Execute the user's handler
    // No tokio::select! needed anymore. If the driver loop aborts this task,
    // this await simply acts as a cancellation point.
    handler(executing).await;
    Ok(())
}

/// Handles incoming cancel requests.
async fn handle_cancel_request<A: Action>(inner: &Arc<InnerServer<A>>, query: zenoh::query::Query) {
    tracing::debug!("Received cancel request");
    let request = match decode_query_message::<CancelGoalServiceRequest>(&query) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to deserialize cancel request: {}", e);
            let response = malformed_cancel_response();
            reply_to_cancel_query(query, &response, "Failed to send malformed cancel response");
            return;
        }
    };

    let goal_id = request.goal_info.goal_id;
    let cancelled = inner.goal_manager.read(|manager| {
        if let Some(ServerGoalState::Executing { cancel_flag, .. }) = manager.goals.get(&goal_id) {
            cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            true
        } else {
            false
        }
    });

    let response = build_cancel_response(cancelled, request.goal_info);

    if !reply_to_cancel_query(query, &response, "Failed to send cancel response") {
        return;
    }

    tracing::debug!("Sent cancel response");
}

/// Handles incoming result requests.
async fn handle_result_request<A: Action>(inner: &Arc<InnerServer<A>>, query: zenoh::query::Query) {
    tracing::debug!("Received result request");
    let request = match decode_query_message::<GetResultRequest>(&query) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to deserialize result request: {}", e);
            return;
        }
    };

    // Check if goal is already terminated, or register a waiter
    let (tx, rx) = tokio::sync::oneshot::channel();
    enum ResultState {
        Terminated,
        Waiting,
        NotFound,
    }

    let (result_state, result_data) = inner.goal_manager.modify(|manager| {
        if let Some(ServerGoalState::Terminated { result, status, .. }) =
            manager.goals.get(&request.goal_id)
        {
            // Goal is already terminated - return result immediately
            (ResultState::Terminated, Some((result.clone(), *status)))
        } else if manager.goals.contains_key(&request.goal_id) {
            // Goal exists but not terminated yet - register waiter
            manager
                .result_futures
                .entry(request.goal_id)
                .or_default()
                .push(tx);
            (ResultState::Waiting, None)
        } else {
            // Goal doesn't exist
            (ResultState::NotFound, None)
        }
    }); // Lock released here

    let (result, status) = match result_state {
        ResultState::Terminated => {
            let (r, s) = result_data.unwrap();
            tracing::debug!(
                "Goal {:?} is already terminated with status {:?}",
                request.goal_id,
                s
            );
            (r, s)
        }
        ResultState::Waiting => {
            // Wait for goal to complete
            tracing::debug!(
                "Goal {:?} not terminated yet, waiting for result...",
                request.goal_id
            );
            match rx.await {
                Ok((r, s)) => {
                    tracing::debug!("Goal {:?} completed with status {:?}", request.goal_id, s);
                    (r, s)
                }
                Err(_) => {
                    tracing::warn!("Result future cancelled for goal {:?}", request.goal_id);
                    return; // Don't send response
                }
            }
        }
        ResultState::NotFound => {
            tracing::warn!("Goal {:?} not found", request.goal_id);
            return; // Don't send response
        }
    };

    // Send result response
    let response = GetResultResponse::<A> {
        status: status as i8,
        result,
    };
    let attachment = match query_attachment(&query) {
        Ok(attachment) => attachment,
        Err(error) => {
            tracing::warn!("Failed to decode result request attachment: {}", error);
            return;
        }
    };
    let key_expr = query.key_expr().clone();
    if let Err(error) = reply_with_attachment(query, key_expr, attachment, &response) {
        tracing::warn!("Failed to send result response: {}", error);
    }
    tracing::debug!("Sent result response");
}

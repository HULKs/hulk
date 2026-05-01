//! Safe goal state management with deadlock prevention.
//!
//! This module provides a `SafeGoalManager` that enforces synchronous-only
//! access to goal state, preventing accidental lock-across-await bugs.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use super::{Action, GoalId, GoalStatus};

/// Thread-safe goal state manager with compile-time deadlock prevention.
///
/// The `SafeGoalManager` wraps the internal goal state in a way that
/// prevents holding locks across async operations. All access must go
/// through the `modify` method, which only accepts synchronous closures.
pub struct SafeGoalManager<A: Action> {
    inner: Mutex<GoalManagerInternal<A>>,
}

impl<A: Action> SafeGoalManager<A> {
    pub fn new(result_timeout: Duration, goal_timeout: Option<Duration>) -> Self {
        Self {
            inner: Mutex::new(GoalManagerInternal {
                goals: HashMap::new(),
                result_timeout,
                goal_timeout,
                result_futures: HashMap::new(),
            }),
        }
    }

    /// The ONLY way to access goal state.
    ///
    /// This method enforces that all state access happens in a synchronous closure,
    /// preventing accidental lock-across-await bugs. The lock is automatically
    /// released when the closure returns.
    ///
    /// # Arguments
    ///
    /// * `f` - A synchronous closure that can read/modify the goal state
    ///
    /// # Returns
    ///
    /// The value returned by the closure
    pub fn modify<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut GoalManagerInternal<A>) -> R,
    {
        let mut guard = self.lock_recovering();
        f(&mut guard)
    }

    /// Read-only access to goal state.
    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&GoalManagerInternal<A>) -> R,
    {
        let guard = self.lock_recovering();
        f(&guard)
    }

    fn lock_recovering(&self) -> MutexGuard<'_, GoalManagerInternal<A>> {
        self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::error!("goal manager mutex poisoned; recovering inner state");
            poisoned.into_inner()
        })
    }
}

/// Type alias for result future senders to reduce complexity.
type ResultSenders<A> = Vec<tokio::sync::oneshot::Sender<(<A as Action>::Result, GoalStatus)>>;

/// Internal goal state storage.
///
/// This is kept private and only accessible through `SafeGoalManager::modify`.
pub struct GoalManagerInternal<A: Action> {
    pub goals: HashMap<GoalId, ServerGoalState<A>>,
    pub result_timeout: Duration,
    pub goal_timeout: Option<Duration>,
    pub result_futures: HashMap<GoalId, ResultSenders<A>>,
}

/// Server-side state for an action goal.
pub enum ServerGoalState<A: Action> {
    Accepted {
        goal: A::Goal,
        timestamp: Instant,
        expires_at: Option<Instant>,
    },
    Executing {
        goal: A::Goal,
        cancel_flag: Arc<AtomicBool>,
        expires_at: Option<Instant>,
    },
    Canceling {
        goal: A::Goal,
    },
    Terminated {
        result: A::Result,
        status: GoalStatus,
        timestamp: Instant,
        expires_at: Option<Instant>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct TestAction;

    impl Action for TestAction {
        type Goal = u8;
        type Result = u8;
        type Feedback = u8;

        fn name() -> &'static str {
            "test::Poison"
        }
    }

    #[test]
    fn goal_manager_recovers_from_poisoned_mutex() {
        let manager = SafeGoalManager::<TestAction>::new(Duration::from_secs(1), None);
        let _ = catch_unwind(AssertUnwindSafe(|| manager.modify(|_| panic!("poison"))));
        assert_eq!(manager.read(|state| state.goals.len()), 0);
    }
}

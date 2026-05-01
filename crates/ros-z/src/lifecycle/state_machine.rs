/// Native lifecycle state IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    // Primary states
    Unconfigured = 1,
    Inactive = 2,
    Active = 3,
    Finalized = 4,
    // Transition states (node is "busy")
    Configuring = 10,
    CleaningUp = 11,
    ShuttingDown = 12,
    Activating = 13,
    Deactivating = 14,
    ErrorProcessing = 15,
}

impl State {
    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn label(self) -> &'static str {
        match self {
            State::Unconfigured => "unconfigured",
            State::Inactive => "inactive",
            State::Active => "active",
            State::Finalized => "finalized",
            State::Configuring => "configuring",
            State::CleaningUp => "cleaningup",
            State::ShuttingDown => "shuttingdown",
            State::Activating => "activating",
            State::Deactivating => "deactivating",
            State::ErrorProcessing => "errorprocessing",
        }
    }

    pub fn is_primary(self) -> bool {
        matches!(
            self,
            State::Unconfigured | State::Inactive | State::Active | State::Finalized
        )
    }
}

/// User callback return codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackReturn {
    Success,
    Failure,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionResult {
    Complete(State),
    Invalid {
        transition: TransitionId,
        from: State,
    },
}

/// Native lifecycle transition IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransitionId {
    Configure = 1,
    Cleanup = 2,
    Activate = 3,
    Deactivate = 4,
    /// shutdown from Unconfigured
    UnconfiguredShutdown = 5,
    /// shutdown from Inactive
    InactiveShutdown = 6,
    /// shutdown from Active
    ActiveShutdown = 7,
}

impl TransitionId {
    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn label(self) -> &'static str {
        match self {
            TransitionId::Configure => "configure",
            TransitionId::Cleanup => "cleanup",
            TransitionId::Activate => "activate",
            TransitionId::Deactivate => "deactivate",
            TransitionId::UnconfiguredShutdown => "shutdown",
            TransitionId::InactiveShutdown => "shutdown",
            TransitionId::ActiveShutdown => "shutdown",
        }
    }

    /// Return the transition for a generic "shutdown" from the given primary state.
    pub fn shutdown_for(state: State) -> Option<TransitionId> {
        match state {
            State::Unconfigured => Some(TransitionId::UnconfiguredShutdown),
            State::Inactive => Some(TransitionId::InactiveShutdown),
            State::Active => Some(TransitionId::ActiveShutdown),
            _ => None,
        }
    }

    /// Attempt to map a raw transition id (as sent over the change_state service) to
    /// a TransitionId given the current primary state. Returns None if the transition
    /// is not valid from the current state.
    pub fn from_id_and_state(id: u8, current: State) -> Option<TransitionId> {
        match id {
            1 if current == State::Unconfigured => Some(TransitionId::Configure),
            2 if current == State::Inactive => Some(TransitionId::Cleanup),
            3 if current == State::Inactive => Some(TransitionId::Activate),
            4 if current == State::Active => Some(TransitionId::Deactivate),
            5 if current == State::Unconfigured => Some(TransitionId::UnconfiguredShutdown),
            6 if current == State::Inactive => Some(TransitionId::InactiveShutdown),
            7 if current == State::Active => Some(TransitionId::ActiveShutdown),
            _ => None,
        }
    }

    /// Map a transition label to an id for the current state.
    pub fn from_label_and_state(label: &str, current: State) -> Option<TransitionId> {
        match (label, current) {
            ("configure", State::Unconfigured) => Some(TransitionId::Configure),
            ("cleanup", State::Inactive) => Some(TransitionId::Cleanup),
            ("activate", State::Inactive) => Some(TransitionId::Activate),
            ("deactivate", State::Active) => Some(TransitionId::Deactivate),
            ("shutdown", State::Unconfigured) => Some(TransitionId::UnconfiguredShutdown),
            ("shutdown", State::Inactive) => Some(TransitionId::InactiveShutdown),
            ("shutdown", State::Active) => Some(TransitionId::ActiveShutdown),
            _ => None,
        }
    }
}

/// The full ros-z lifecycle state machine.
///
/// Holds the current state and implements all valid transition paths, including
/// the error processing path.
pub struct StateMachine {
    current: State,
    pending_transition: Option<(TransitionId, State)>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        StateMachine {
            current: State::Unconfigured,
            pending_transition: None,
        }
    }

    pub fn current_state(&self) -> State {
        self.current
    }

    /// Attempt to apply the given transition from the current primary state.
    ///
    /// The `callback` closure receives the current *primary* state before the
    /// transition begins and should return a [`CallbackReturn`].
    ///
    /// Returns the result after this transition callback completes. If the
    /// callback returns [`CallbackReturn::Error`], callers should invoke
    /// [`Self::trigger_error_processing`] to complete error handling.
    pub fn trigger<F>(&mut self, transition: TransitionId, callback: F) -> TransitionResult
    where
        F: FnOnce(State) -> CallbackReturn,
    {
        let Some(start_state) = self.begin_transition(transition) else {
            return TransitionResult::Invalid {
                transition,
                from: self.current,
            };
        };
        let cb_result = callback(start_state);
        TransitionResult::Complete(self.complete_transition(transition, start_state, cb_result))
    }

    /// Validate and enter the intermediate transition state.
    ///
    /// Returns the previous primary state when the transition is valid, or
    /// `None` when the transition is invalid and the state is unchanged.
    pub fn begin_transition(&mut self, transition: TransitionId) -> Option<State> {
        let start_state = self.current;
        if !Self::is_valid_transition(transition, start_state) {
            return None;
        }

        self.current = Self::intermediate_state(transition);
        self.pending_transition = Some((transition, start_state));
        Some(start_state)
    }

    /// Complete a transition that was started with [`Self::begin_transition`].
    pub fn complete_transition(
        &mut self,
        transition: TransitionId,
        start_state: State,
        cb_result: CallbackReturn,
    ) -> State {
        if self.current != Self::intermediate_state(transition)
            || self.pending_transition != Some((transition, start_state))
        {
            return self.current;
        }
        self.pending_transition = None;

        match cb_result {
            CallbackReturn::Success => {
                // Happy path: move to goal state
                self.current = Self::goal_state(transition);
            }
            CallbackReturn::Failure => {
                // Transition failed: revert to start state
                self.current = start_state;
            }
            CallbackReturn::Error => {
                // Error: enter ErrorProcessing. The node calls
                // trigger_error_processing() after invoking on_error.
                self.current = State::ErrorProcessing;
            }
        }

        self.current
    }

    /// Called after a `CallbackReturn::Error` to run the error-processing
    /// transition. Returns the state after error handling.
    pub fn trigger_error_processing<F>(&mut self, on_error: F) -> State
    where
        F: FnOnce(State) -> CallbackReturn,
    {
        debug_assert_eq!(self.current, State::ErrorProcessing);
        let cb = on_error(State::ErrorProcessing);
        self.current = match cb {
            CallbackReturn::Success => State::Unconfigured,
            // FAILURE or ERROR both lead to Finalized
            _ => State::Finalized,
        };
        self.current
    }

    /// The intermediate (transition) state for a given transition id.
    fn intermediate_state(t: TransitionId) -> State {
        match t {
            TransitionId::Configure => State::Configuring,
            TransitionId::Cleanup => State::CleaningUp,
            TransitionId::Activate => State::Activating,
            TransitionId::Deactivate => State::Deactivating,
            TransitionId::UnconfiguredShutdown
            | TransitionId::InactiveShutdown
            | TransitionId::ActiveShutdown => State::ShuttingDown,
        }
    }

    fn is_valid_transition(transition: TransitionId, start_state: State) -> bool {
        matches!(
            (transition, start_state),
            (TransitionId::Configure, State::Unconfigured)
                | (TransitionId::Cleanup, State::Inactive)
                | (TransitionId::Activate, State::Inactive)
                | (TransitionId::Deactivate, State::Active)
                | (TransitionId::UnconfiguredShutdown, State::Unconfigured)
                | (TransitionId::InactiveShutdown, State::Inactive)
                | (TransitionId::ActiveShutdown, State::Active)
        )
    }

    /// The goal (primary) state if the transition callback succeeds.
    fn goal_state(t: TransitionId) -> State {
        match t {
            TransitionId::Configure => State::Inactive,
            TransitionId::Cleanup => State::Unconfigured,
            TransitionId::Activate => State::Active,
            TransitionId::Deactivate => State::Inactive,
            TransitionId::UnconfiguredShutdown
            | TransitionId::InactiveShutdown
            | TransitionId::ActiveShutdown => State::Finalized,
        }
    }

    /// All states in the lifecycle graph (for `get_available_states` service).
    pub fn all_states() -> &'static [(u8, &'static str)] {
        &[
            (State::Unconfigured as u8, "unconfigured"),
            (State::Inactive as u8, "inactive"),
            (State::Active as u8, "active"),
            (State::Finalized as u8, "finalized"),
            (State::Configuring as u8, "configuring"),
            (State::CleaningUp as u8, "cleaningup"),
            (State::ShuttingDown as u8, "shuttingdown"),
            (State::Activating as u8, "activating"),
            (State::Deactivating as u8, "deactivating"),
            (State::ErrorProcessing as u8, "errorprocessing"),
        ]
    }

    /// Transitions available from the current primary state.
    pub fn available_transitions(&self) -> Vec<(TransitionId, State, State)> {
        match self.current {
            State::Unconfigured => vec![
                (
                    TransitionId::Configure,
                    State::Unconfigured,
                    State::Inactive,
                ),
                (
                    TransitionId::UnconfiguredShutdown,
                    State::Unconfigured,
                    State::Finalized,
                ),
            ],
            State::Inactive => vec![
                (TransitionId::Activate, State::Inactive, State::Active),
                (TransitionId::Cleanup, State::Inactive, State::Unconfigured),
                (
                    TransitionId::InactiveShutdown,
                    State::Inactive,
                    State::Finalized,
                ),
            ],
            State::Active => vec![
                (TransitionId::Deactivate, State::Active, State::Inactive),
                (
                    TransitionId::ActiveShutdown,
                    State::Active,
                    State::Finalized,
                ),
            ],
            _ => vec![],
        }
    }

    /// All transitions in the transition graph (for `get_transition_graph` service).
    pub fn all_transitions() -> Vec<(TransitionId, State, State)> {
        vec![
            (
                TransitionId::Configure,
                State::Unconfigured,
                State::Inactive,
            ),
            (TransitionId::Activate, State::Inactive, State::Active),
            (TransitionId::Deactivate, State::Active, State::Inactive),
            (TransitionId::Cleanup, State::Inactive, State::Unconfigured),
            (
                TransitionId::UnconfiguredShutdown,
                State::Unconfigured,
                State::Finalized,
            ),
            (
                TransitionId::InactiveShutdown,
                State::Inactive,
                State::Finalized,
            ),
            (
                TransitionId::ActiveShutdown,
                State::Active,
                State::Finalized,
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sm() -> StateMachine {
        StateMachine::new()
    }

    fn ok(_: State) -> CallbackReturn {
        CallbackReturn::Success
    }
    fn fail(_: State) -> CallbackReturn {
        CallbackReturn::Failure
    }
    fn err(_: State) -> CallbackReturn {
        CallbackReturn::Error
    }

    #[test]
    fn test_initial_state() {
        assert_eq!(sm().current_state(), State::Unconfigured);
    }

    #[test]
    fn test_configure_success() {
        let mut m = sm();
        let s = m.trigger(TransitionId::Configure, ok);
        assert_eq!(s, TransitionResult::Complete(State::Inactive));
        assert_eq!(m.current_state(), State::Inactive);
    }

    #[test]
    fn test_configure_failure() {
        let mut m = sm();
        let s = m.trigger(TransitionId::Configure, fail);
        // failure reverts to start state
        assert_eq!(s, TransitionResult::Complete(State::Unconfigured));
    }

    #[test]
    fn test_configure_error_then_error_processing_success() {
        let mut m = sm();
        m.trigger(TransitionId::Configure, err);
        assert_eq!(m.current_state(), State::ErrorProcessing);
        let s = m.trigger_error_processing(ok);
        assert_eq!(s, State::Unconfigured);
    }

    #[test]
    fn test_configure_error_then_error_processing_failure() {
        let mut m = sm();
        m.trigger(TransitionId::Configure, err);
        assert_eq!(m.current_state(), State::ErrorProcessing);
        let s = m.trigger_error_processing(fail);
        assert_eq!(s, State::Finalized);
    }

    #[test]
    fn test_full_lifecycle() {
        let mut m = sm();
        assert_eq!(
            m.trigger(TransitionId::Configure, ok),
            TransitionResult::Complete(State::Inactive)
        );
        assert_eq!(
            m.trigger(TransitionId::Activate, ok),
            TransitionResult::Complete(State::Active)
        );
        assert_eq!(
            m.trigger(TransitionId::Deactivate, ok),
            TransitionResult::Complete(State::Inactive)
        );
        assert_eq!(
            m.trigger(TransitionId::Cleanup, ok),
            TransitionResult::Complete(State::Unconfigured)
        );
        assert_eq!(
            m.trigger(TransitionId::UnconfiguredShutdown, ok),
            TransitionResult::Complete(State::Finalized)
        );
    }

    #[test]
    fn test_shutdown_from_unconfigured() {
        let mut m = sm();
        let s = m.trigger(TransitionId::UnconfiguredShutdown, ok);
        assert_eq!(s, TransitionResult::Complete(State::Finalized));
    }

    #[test]
    fn test_shutdown_from_inactive() {
        let mut m = sm();
        m.trigger(TransitionId::Configure, ok);
        let s = m.trigger(TransitionId::InactiveShutdown, ok);
        assert_eq!(s, TransitionResult::Complete(State::Finalized));
    }

    #[test]
    fn test_shutdown_from_active() {
        let mut m = sm();
        m.trigger(TransitionId::Configure, ok);
        m.trigger(TransitionId::Activate, ok);
        let s = m.trigger(TransitionId::ActiveShutdown, ok);
        assert_eq!(s, TransitionResult::Complete(State::Finalized));
    }

    #[test]
    fn test_invalid_transition_ignored() {
        let mut m = sm();
        // configure requires Unconfigured state; activate requires Inactive
        let s = m.trigger(TransitionId::Activate, ok);
        // still Unconfigured — invalid transitions are no-ops
        assert_eq!(
            s,
            TransitionResult::Invalid {
                transition: TransitionId::Activate,
                from: State::Unconfigured,
            }
        );
    }

    #[test]
    fn invalid_transition_returns_explicit_invalid_result() {
        let mut machine = StateMachine::new();
        let result = machine.trigger(TransitionId::Activate, |_| CallbackReturn::Success);
        assert_eq!(
            result,
            TransitionResult::Invalid {
                transition: TransitionId::Activate,
                from: State::Unconfigured,
            }
        );
        assert_eq!(machine.current_state(), State::Unconfigured);
    }

    #[test]
    fn complete_transition_ignores_unstarted_transition() {
        let mut machine = StateMachine::new();

        let state = machine.complete_transition(
            TransitionId::Activate,
            State::Inactive,
            CallbackReturn::Success,
        );

        assert_eq!(state, State::Unconfigured);
        assert_eq!(machine.current_state(), State::Unconfigured);
    }

    #[test]
    fn complete_transition_rejects_mismatched_shutdown_transition() {
        let mut machine = StateMachine::new();
        assert_eq!(
            machine.begin_transition(TransitionId::UnconfiguredShutdown),
            Some(State::Unconfigured)
        );

        let state = machine.complete_transition(
            TransitionId::ActiveShutdown,
            State::Active,
            CallbackReturn::Failure,
        );

        assert_eq!(state, State::ShuttingDown);
        assert_eq!(machine.current_state(), State::ShuttingDown);
    }

    #[test]
    fn test_finalized_is_terminal() {
        let mut m = sm();
        m.trigger(TransitionId::UnconfiguredShutdown, ok);
        assert_eq!(m.current_state(), State::Finalized);
        // No further transition valid from Finalized
        let s = m.trigger(TransitionId::Configure, ok);
        assert_eq!(
            s,
            TransitionResult::Invalid {
                transition: TransitionId::Configure,
                from: State::Finalized,
            }
        );
    }

    #[test]
    fn test_intermediate_state_during_configure() {
        let mut m = sm();
        // Inspect the intermediate state seen by the callback
        let mut seen = State::Unconfigured;
        m.trigger(TransitionId::Configure, |prev| {
            // The state machine has already moved to Configuring before calling us.
            // We can only observe via prev (the start state).
            seen = prev;
            CallbackReturn::Success
        });
        // Callback received start state, not transition state.
        assert_eq!(seen, State::Unconfigured);
    }

    #[test]
    fn test_available_transitions_unconfigured() {
        let m = sm();
        let t = m.available_transitions();
        assert!(t.iter().any(|(id, _, _)| *id == TransitionId::Configure));
        assert!(
            t.iter()
                .any(|(id, _, _)| *id == TransitionId::UnconfiguredShutdown)
        );
    }

    #[test]
    fn test_transition_id_from_id_and_state() {
        assert_eq!(
            TransitionId::from_id_and_state(1, State::Unconfigured),
            Some(TransitionId::Configure)
        );
        assert_eq!(
            TransitionId::from_id_and_state(3, State::Inactive),
            Some(TransitionId::Activate)
        );
        // invalid: configure while Active
        assert_eq!(TransitionId::from_id_and_state(1, State::Active), None);
    }

    #[test]
    fn test_all_states_count() {
        assert_eq!(StateMachine::all_states().len(), 10);
    }
}

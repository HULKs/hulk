/// Macro to define a simple native ros-z action type.
///
/// This macro implements the basic `Action` trait methods for custom action
/// payload types that use ros-z's serde CDR codec path.
///
/// # Syntax
///
/// ```ignore
/// define_action! {
///     ActionStruct,
///     action_name: "action_name",
///     Goal: GoalType,
///     Result: ResultType,
///     Feedback: FeedbackType,
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// use ros_z::define_action;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct NavigateToPoseGoal {
///     pub target_x: f64,
///     pub target_y: f64,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct NavigateToPoseResult {
///     pub success: bool,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct NavigateToPoseFeedback {
///     pub current_x: f64,
///     pub current_y: f64,
/// }
///
/// pub struct NavigateToPose;
///
/// define_action! {
///     NavigateToPose,
///     action_name: "navigate_to_pose",
///     Goal: NavigateToPoseGoal,
///     Result: NavigateToPoseResult,
///     Feedback: NavigateToPoseFeedback,
/// }
/// ```
#[macro_export]
macro_rules! define_action {
    (
        $action_struct:ident,
        action_name: $action_name:expr,
        Goal: $goal_type:ty,
        Result: $result_type:ty,
        Feedback: $feedback_type:ty $(,)?
    ) => {
        impl $crate::action::Action for $action_struct {
            type Goal = $goal_type;
            type Result = $result_type;
            type Feedback = $feedback_type;

            fn name() -> &'static str {
                $action_name
            }
        }

        // Provide WireMessage impls via the serde CDR codec path for types that
        // do not implement the CDR traits. Types that do implement CdrEncode +
        // CdrDecode + CdrEncodedSize get WireMessage automatically from the
        // blanket impl in ros_z::msg and should NOT use define_action!.
        impl $crate::msg::WireMessage for $goal_type
        where
            $goal_type: Send + Sync + 'static,
        {
            type Codec = $crate::msg::SerdeCdrCodec<$goal_type>;
        }
        impl $crate::msg::WireMessage for $result_type
        where
            $result_type: Send + Sync + 'static,
        {
            type Codec = $crate::msg::SerdeCdrCodec<$result_type>;
        }
        impl $crate::msg::WireMessage for $feedback_type
        where
            $feedback_type: Send + Sync + 'static,
        {
            type Codec = $crate::msg::SerdeCdrCodec<$feedback_type>;
        }
    };
}

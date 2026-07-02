use crate::{
    action,
    behavior::{
        behavior_tree::Node,
        condition::{is_ball_interception_candidate, is_close_to_ball},
        head::look_at_ball_subtree,
        kick::{intercept, kick_subtree},
        node::Blackboard,
        substates::{is_in_sub_state, sub_state_subtree},
        walk::walk_to_ball_subtree,
    },
    condition, negation, selection, sequence, subtree,
};

pub fn striker_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(condition!(is_in_sub_state), subtree!(sub_state_subtree),),
            sequence!(
                negation!(condition!(is_close_to_ball)),
                subtree!(walk_to_ball_subtree)
            ),
            sequence!(
                condition!(is_ball_interception_candidate),
                subtree!(kick_subtree),
                action!(intercept),
            ),
            subtree!(kick_subtree)
        )
    )
}

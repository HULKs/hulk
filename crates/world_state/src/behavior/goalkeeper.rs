use hsl_network_messages::SubState;
use types::motion_type::MotionType;

use crate::{
    action,
    behavior::{
        behavior_tree::Node,
        condition::{
            hulks_is_kicking_team, is_ball_near_own_goal, is_goalkeeper_clear_candidate,
            is_goalkeeper_interception_candidate, is_goalkeeper_visual_kick_hold_active,
            is_last_man_standing,
        },
        head::look_at_ball_subtree,
        kick::{goalkeeper_clear_kick_subtree, goalkeeper_intercept_kick_subtree},
        node::Blackboard,
        striker::striker_subtree,
        substates::{is_in_sub_state, is_sub_state},
        switch_motion_type::switch_motion_type,
        walk::{
            set_goalkeeper_active_defense_position, walk_alternatives_subtree,
            walk_to_block_position, walk_to_goalkeeper_default_position,
            walk_to_goalkeeper_penalty_position,
        },
    },
    condition, negation, selection, sequence, subtree,
};

pub fn goalkeeper_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(
                condition!(is_in_sub_state),
                subtree!(goalkeeper_sub_state_subtree)
            ),
            sequence!(condition!(is_last_man_standing), subtree!(striker_subtree)),
            sequence!(
                condition!(is_goalkeeper_interception_candidate),
                subtree!(goalkeeper_intercept_kick_subtree)
            ),
            sequence!(
                condition!(is_goalkeeper_visual_kick_hold_active),
                subtree!(goalkeeper_clear_kick_subtree)
            ),
            sequence!(
                condition!(is_goalkeeper_clear_candidate),
                subtree!(goalkeeper_clear_kick_subtree)
            ),
            sequence!(
                condition!(is_ball_near_own_goal),
                subtree!(goalkeeper_active_defense_position_subtree)
            ),
            subtree!(goalkeeper_default_position_subtree),
        )
    )
}

fn goalkeeper_sub_state_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_sub_state, SubState::PenaltyKick),
            negation!(condition!(hulks_is_kicking_team)),
            subtree!(goalkeeper_penalty_position_subtree)
        ),
        sequence!(
            condition!(hulks_is_kicking_team),
            condition!(is_sub_state, SubState::GoalKick),
            subtree!(goalkeeper_clear_kick_subtree)
        ),
        sequence!(
            negation!(condition!(hulks_is_kicking_team)),
            condition!(is_sub_state, SubState::CornerKick),
            subtree!(goalkeeper_active_defense_position_subtree)
        ),
        sequence!(
            negation!(condition!(hulks_is_kicking_team)),
            selection!(
                condition!(is_sub_state, SubState::ThrowIn),
                condition!(is_sub_state, SubState::IndirectFreeKick),
                condition!(is_sub_state, SubState::DirectFreeKick),
            ),
            condition!(is_ball_near_own_goal),
            subtree!(goalkeeper_active_defense_position_subtree)
        ),
        sequence!(
            condition!(hulks_is_kicking_team),
            selection!(
                condition!(is_sub_state, SubState::ThrowIn),
                condition!(is_sub_state, SubState::IndirectFreeKick),
                condition!(is_sub_state, SubState::DirectFreeKick),
            ),
            condition!(is_ball_near_own_goal),
            subtree!(goalkeeper_clear_kick_subtree)
        ),
        subtree!(goalkeeper_default_position_subtree),
    )
}

fn goalkeeper_active_defense_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        sequence!(
            action!(set_goalkeeper_active_defense_position),
            action!(walk_to_block_position)
        ),
        subtree!(walk_alternatives_subtree),
    )
}

fn goalkeeper_penalty_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(walk_to_goalkeeper_penalty_position),
        subtree!(walk_alternatives_subtree),
    )
}

fn goalkeeper_default_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(walk_to_goalkeeper_default_position),
        subtree!(walk_alternatives_subtree),
    )
}

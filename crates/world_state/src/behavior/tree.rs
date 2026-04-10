use types::{motion_command::KickPower, primary_state::PrimaryState};

use crate::{
    action,
    behavior::{
        action::{injected_motion_command, leuchtturm, prepare, stand, stand_up}, behavior_tree::Node, condition::{
            has_ball_position, is_close_to_ball, is_close_to_goal, is_closest_to_ball, is_fallen, is_goalkeeper, is_primary_state
        }, kick_actions::kicking, node::Blackboard, walk_actions::walk_to_ball
    },
    condition, negation, selection, sequence,
};

pub fn create_tree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_primary_state, PrimaryState::Safe),
            action!(prepare)
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Stop),
            action!(stand)
        ),
        action!(injected_motion_command),
        sequence!(
            selection!(
                condition!(is_primary_state, PrimaryState::Initial),
                condition!(is_primary_state, PrimaryState::Penalized)
            ),
            action!(stand)
        ),
        sequence!(condition!(is_fallen), action!(stand_up)),
        sequence!(
            condition!(is_primary_state, PrimaryState::Set),
            action!(stand)
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Ready),
            ready_subtree()
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Playing),
            playing_subtree()
        ),
        Node::Failure
    )
}

fn ready_subtree() -> Node<Blackboard> {
    action!(stand)
}

fn playing_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(condition!(is_goalkeeper), goalkeeper_subtree()),
        sequence!(negation!(condition!(has_ball_position)), search_subtree()),
        sequence!(condition!(is_closest_to_ball), striker_subtree()),
        supporter_subtree(),
    )
}

fn goalkeeper_subtree() -> Node<Blackboard> {
    action!(stand)
}

fn search_subtree() -> Node<Blackboard> {
    action!(leuchtturm)
}

fn striker_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            negation!(condition!(is_close_to_ball)),
            action!(walk_to_ball)
        ),
        sequence!(
            condition!(is_close_to_goal),
            action!(kicking, KickPower::Rumpelstilzchen)
        ),
        action!(kicking, KickPower::Schlong)
    )
}

fn supporter_subtree() -> Node<Blackboard> {
    action!(stand)
}

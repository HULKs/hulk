use types::primary_state::PrimaryState;

use crate::{
    action,
    behavior::{
        action::{injected_motion_command, prepare, stand, stand_up, walk_to_ball},
        behavior_tree::Node,
        condition::{has_ball_position, is_fallen, is_primary_state},
        node::CaptainBlackboard,
    },
    condition, selection, sequence,
};

pub fn create_tree() -> Node<CaptainBlackboard> {
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

fn ready_subtree() -> Node<CaptainBlackboard> {
    action!(stand)
}

fn playing_subtree() -> Node<CaptainBlackboard> {
    selection!(sequence!(
        condition!(has_ball_position),
        action!(walk_to_ball)
    ),)
}

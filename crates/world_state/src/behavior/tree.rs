use types::primary_state::PrimaryState;

use crate::{action, behavior::{action::{stand, walk_to_ball}, condition::{has_ball_position, is_primary_state}, new_behavior::behavior_tree::Node, node::CaptainBlackboard}, condition, selection, sequence};


pub fn create_tree() -> Node<CaptainBlackboard> {
    selection!(
        sequence!(
            condition!(is_primary_state, PrimaryState::Playing),
            condition!(has_ball_position),
            action!(walk_to_ball)
        ),
        action!(stand)
    )
}
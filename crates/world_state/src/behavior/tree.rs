use types::{motion_type::MotionType, primary_state::PrimaryState};

use crate::{
    action,
    behavior::{
        action::{injected_motion_command, prepare, stand, stand_up},
        behavior_tree::Node,
        condition::{
            has_ball_position, is_ball_interception_candidate, is_close_to_ball,
            is_closest_to_ball, is_fallen, is_goalkeeper, is_primary_state,
        },
        head::{look_at_ball_subtree, look_straight_ahead, search_for_lost_ball},
        kick::{intercept, kick_subtree},
        node::Blackboard,
        search::{has_suggested_search_position, leuchtturm, walk_to_search_position},
        switch_motion_type::switch_motion_type,
        walk::{walk_alternatives_subtree, walk_to_ball},
    },
    condition, negation, selection, sequence, subtree,
};

pub fn create_tree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_primary_state, PrimaryState::Safe),
            switch_motion_type(
                MotionType::Prepare,
                action!(prepare),
                sequence!(action!(look_straight_ahead), action!(stand))
            )
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
            sequence!(action!(look_straight_ahead), action!(stand))
        ),
        sequence!(condition!(is_fallen), action!(stand_up)),
        sequence!(
            condition!(is_primary_state, PrimaryState::Set),
            sequence!(subtree!(look_at_ball_subtree), action!(stand))
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Ready),
            subtree!(ready_subtree)
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Playing),
            subtree!(playing_subtree)
        ),
        Node::Failure
    )
}

fn ready_subtree() -> Node<Blackboard> {
    sequence!(subtree!(look_at_ball_subtree), action!(stand))
}

fn playing_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(condition!(is_goalkeeper), subtree!(goalkeeper_subtree)),
        sequence!(
            negation!(condition!(has_ball_position)),
            subtree!(search_subtree)
        ),
        sequence!(condition!(is_closest_to_ball), subtree!(striker_subtree)),
        subtree!(supporter_subtree),
    )
}

fn goalkeeper_subtree() -> Node<Blackboard> {
    sequence!(subtree!(look_at_ball_subtree), action!(stand))
}

fn search_subtree() -> Node<Blackboard> {
    sequence!(
        action!(search_for_lost_ball),
        switch_motion_type(
            MotionType::Walk,
            selection!(
                sequence!(
                    condition!(has_suggested_search_position),
                    action!(walk_to_search_position)
                ),
                action!(leuchtturm)
            ),
            subtree!(walk_alternatives_subtree),
        )
    )
}

fn striker_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(
                negation!(condition!(is_close_to_ball)),
                switch_motion_type(
                    MotionType::Walk,
                    action!(walk_to_ball),
                    subtree!(walk_alternatives_subtree),
                )
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

fn supporter_subtree() -> Node<Blackboard> {
    sequence!(subtree!(look_at_ball_subtree), action!(stand))
}

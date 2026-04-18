use types::{motion_command::KickPower, motion_type::MotionType, primary_state::PrimaryState};

use crate::{
    action,
    behavior::{
        action::{injected_motion_command, leuchtturm, prepare, stand, stand_up},
        behavior_tree::Node,
        condition::{
            has_ball_position, is_ball_interception_candidate, is_close_to_ball,
            is_closest_to_ball, is_fallen, is_goalkeeper, is_primary_state,
        },
        kick_actions::{intercept, kick, kick_instead_of_walking},
        kick_selector::{
            allow_schlong, is_close_to_target, select_kick_target, use_kick_power,
            use_last_kick_power,
        },
        node::Blackboard,
        switch_motion_type::{is_allowed_to_switch, is_last_motion_type},
        walk_actions::{walk_instead_of_kicking, walk_to_ball},
    },
    condition, negation, selection, sequence, subtree,
};

pub fn create_tree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_primary_state, PrimaryState::Safe),
            switch_motion_type(MotionType::Prepare, action!(prepare), action!(stand))
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
    action!(stand)
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
    action!(stand)
}

fn search_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(leuchtturm),
        subtree!(walk_alternatives_subtree),
    )
}

fn striker_subtree() -> Node<Blackboard> {
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
            switch_motion_type(
                MotionType::Kick,
                sequence!(
                    action!(select_kick_target),
                    subtree!(kick_power_subtree),
                    action!(intercept),
                ),
                subtree!(kick_alternatives_subtree),
            )
        ),
        switch_motion_type(
            MotionType::Kick,
            sequence!(
                action!(select_kick_target),
                subtree!(kick_power_subtree),
                action!(kick),
            ),
            subtree!(kick_alternatives_subtree)
        )
    )
}

fn supporter_subtree() -> Node<Blackboard> {
    action!(stand)
}

fn switch_motion_type(
    motion_type: MotionType,
    action: Node<Blackboard>,
    alternatives: Node<Blackboard>,
) -> Node<Blackboard> {
    let is_last_motion_type = match motion_type {
        MotionType::Kick => condition!(is_last_motion_type, MotionType::Kick),
        MotionType::Prepare => condition!(is_last_motion_type, MotionType::Prepare),
        MotionType::Stand => condition!(is_last_motion_type, MotionType::Stand),
        MotionType::StandUp => condition!(is_last_motion_type, MotionType::StandUp),
        MotionType::Walk => condition!(is_last_motion_type, MotionType::Walk),
    };

    selection!(
        sequence!(
            selection!(is_last_motion_type, condition!(is_allowed_to_switch)),
            action
        ),
        alternatives
    )
}

fn walk_alternatives_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            sequence!(
                action!(select_kick_target),
                action!(use_last_kick_power),
                action!(kick_instead_of_walking),
            )
        ),
        action!(stand)
    )
}

fn kick_alternatives_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Walk),
            action!(walk_instead_of_kicking)
        ),
        action!(stand)
    )
}

fn kick_power_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            action!(use_last_kick_power)
        ),
        sequence!(
            negation!(condition!(is_close_to_target)),
            condition!(allow_schlong),
            action!(use_kick_power, KickPower::Schlong)
        ),
        action!(use_kick_power, KickPower::Rumpelstilzchen)
    )
}

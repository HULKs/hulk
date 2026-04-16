use types::{behavior_tree::MotionType, motion_command::KickPower, primary_state::PrimaryState};

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
        node::{Blackboard},
        switch::{is_allowed_to_switch, is_last_motion_type},
        walk_actions::{walk_instead_of_kicking, walk_to_ball},
    },
    condition, negation, selection, sequence,
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
    switch_motion_type(
        MotionType::Walk,
        action!(leuchtturm),
        walk_alternatives_subtree(),
    )
}

fn striker_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            negation!(condition!(is_close_to_ball)),
            switch_motion_type(
                MotionType::Walk,
                action!(walk_to_ball),
                walk_alternatives_subtree(),
            )
        ),
        sequence!(
            condition!(is_ball_interception_candidate),
            switch_motion_type(
                MotionType::Kick,
                action!(intercept),
                kick_alternatives_subtree(),
            )
        ),
        // sequence!(
        //     condition!(is_close_to_goal),
        //     action!(kick, KickPower::Rumpelstilzchen)
        // ),
        // action!(kick, KickPower::Schlong)
        switch_motion_type(
            MotionType::Kick,
            action!(kick, KickPower::Rumpelstilzchen),
            kick_alternatives_subtree()
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
            action!(kick_instead_of_walking)
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

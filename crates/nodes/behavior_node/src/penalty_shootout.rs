use hsl_network_messages::GamePhase;
use linear_algebra::point;
use types::{behavior_tree::Status, motion_command::KickPower, motion_type::MotionType};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    conditions::hulks_is_kicking_team,
    kick::{allow_schlong, apply_visual_kick_target, kick, use_kick_power},
    node::Blackboard,
    selection, sequence, subtree,
};

pub fn is_penalty_shootout(blackboard: &mut Blackboard) -> bool {
    blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|state| matches!(state.game_phase, GamePhase::PenaltyShootout { .. }))
}

pub fn penalty_shootout_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(hulks_is_kicking_team),
            subtree!(penalty_kick_striker_subtree),
        ),
        action!(stand),
    )
}

pub fn penalty_kick_striker_subtree() -> Node<Blackboard> {
    sequence!(
        action!(kick),
        action!(set_penalty_kick_target),
        selection!(
            sequence!(
                condition!(allow_schlong),
                action!(use_kick_power, KickPower::Schlong),
            ),
            action!(use_kick_power, KickPower::Rumpelstilzchen),
        )
    )
}

pub fn set_penalty_kick_target(blackboard: &mut Blackboard) -> Status {
    let field_dimensions = blackboard.field_dimensions;
    let penalty_kick_target_y_offset = blackboard.parameters.substates.penalty_kick_target_y_offset;

    let target = match (blackboard.last_motion_type, blackboard.last_kick_target) {
        (Some(MotionType::Kick), Some(kick_target)) => kick_target,
        _ => {
            let target_y = if rand::random() {
                penalty_kick_target_y_offset
            } else {
                -penalty_kick_target_y_offset
            };

            let target = point!(field_dimensions.length / 2.0, target_y);

            blackboard.last_kick_target = Some(target);
            target
        }
    };

    apply_visual_kick_target(
        blackboard,
        target,
        blackboard.parameters.kicking.kick_target_offset_angle,
    )
}

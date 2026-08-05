use hsl_network_messages::GamePhase;
use linear_algebra::point;
use rand::{Rng, SeedableRng, rngs::StdRng};
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
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let field_dimensions = blackboard.field_dimensions;
        let penalty_kick_target_y_scale =
            blackboard.parameters.substates.penalty_kick_target_y_scale;
        let target_goal_area_half =
            field_dimensions.goal_inner_width / 2.0 * penalty_kick_target_y_scale;

        let target = match (blackboard.last_motion_type, blackboard.last_kick_target) {
            (Some(MotionType::Kick), Some(kick_target)) => kick_target,
            _ => {
                let mut rng = StdRng::seed_from_u64(blackboard.world_state.now.as_nanos() as u64);
                let target_y = if rng.random() {
                    target_goal_area_half
                } else {
                    -target_goal_area_half
                };

                let target =
                    ground_to_field * point!(field_dimensions.penalty_area_length, target_y);

                blackboard.last_kick_target = Some(target);
                target
            }
        };

        return apply_visual_kick_target(
            blackboard,
            target,
            blackboard.parameters.kicking.kick_target_offset_angle,
        );
    }
    Status::Failure
}

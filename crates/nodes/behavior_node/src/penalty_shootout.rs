use hsl_network_messages::GamePhase;
use linear_algebra::point;
use rand::{Rng, SeedableRng, rngs::StdRng};
use types::{
    behavior_tree::Status, filtered_game_state::FilteredGameState, motion_type::MotionType,
};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    conditions::{has_ball_position, hulks_is_kicking_team, is_close_to_ball},
    kick::{apply_kick_target, kick, kick_alternatives_subtree, kick_strength_subtree},
    node::Blackboard,
    selection, sequence, subtree,
    switch_motion_type::switch_motion_type,
    walk::walk_to_ball_subtree,
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
            condition!(is_shootout_playing),
            condition!(hulks_is_kicking_team),
            subtree!(penalty_kick_striker_subtree),
        ),
        action!(stand),
    )
}

fn is_shootout_playing(blackboard: &mut Blackboard) -> bool {
    blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|state| {
            matches!(state.game_state, FilteredGameState::Playing { .. })
                && state.penalties[blackboard.player_number].is_none()
        })
}

pub fn penalty_kick_striker_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_close_to_ball),
            switch_motion_type(
                MotionType::Kick,
                sequence!(
                    action!(kick),
                    action!(set_penalty_kick_target),
                    subtree!(kick_strength_subtree),
                ),
                subtree!(kick_alternatives_subtree),
            )
        ),
        sequence!(
            condition!(has_ball_position),
            subtree!(walk_to_ball_subtree)
        ),
        action!(stand),
    )
}

pub fn set_penalty_kick_target(blackboard: &mut Blackboard) -> Status {
    if blackboard.world_state.robot.ground_to_field.is_some() {
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

                let target = point!(field_dimensions.length / 2.0, target_y);

                blackboard.last_kick_target = Some(target);
                target
            }
        };

        return apply_kick_target(blackboard, target);
    }
    Status::Failure
}

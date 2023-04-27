use nalgebra::{point, Point2};
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    FieldDimensions, GameControllerState, HeadMotion, MotionCommand, PrimaryState, Role, WorldState,
};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
            is_energy_saving: true,
        }),
        PrimaryState::Set => {
            let robot_to_field = world_state.robot.robot_to_field?;
            let fallback_target = match world_state.game_controller_state {
                Some(GameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    kicking_team,
                    ..
                }) => {
                    let side_factor = match kicking_team {
                        Team::Opponent => -1.0,
                        _ => 1.0,
                    };
                    let penalty_spot_x =
                        field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance;
                    let penalty_spot_location = point![side_factor * penalty_spot_x, 0.0];
                    robot_to_field.inverse() * penalty_spot_location
                }
                _ => robot_to_field.inverse() * Point2::origin(),
            };
            let target = world_state
                .ball
                .map(|state| state.ball_in_ground)
                .unwrap_or(fallback_target);
            Some(MotionCommand::Stand {
                head: HeadMotion::LookAt { target },
                is_energy_saving: true,
            })
        }
        PrimaryState::Playing => {
            match (
                world_state.game_controller_state,
                world_state.robot.role,
                world_state.ball,
            ) {
                (
                    Some(GameControllerState {
                        game_phase: GamePhase::PenaltyShootout { .. },
                        ..
                    }),
                    Role::Striker,
                    None,
                ) => Some(MotionCommand::Stand {
                    head: HeadMotion::Center,
                    is_energy_saving: true,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

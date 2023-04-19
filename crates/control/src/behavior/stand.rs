use nalgebra::Point2;
use spl_network_messages::{GamePhase, SubState};
use types::{GameControllerState, HeadMotion, MotionCommand, PrimaryState, Role, WorldState};

pub fn execute(
    world_state: &WorldState,
    absolute_last_known_ball_position: Point2<f32>,
) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
            is_energy_saving: true,
        }),
        PrimaryState::Set => {
            let robot_to_field = world_state.robot.robot_to_field?;
            match world_state.game_controller_state {
                Some(GameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    ..
                }) => Some(MotionCommand::Stand {
                    head: HeadMotion::LookAt {
                        target: robot_to_field.inverse() * absolute_last_known_ball_position,
                    },
                    is_energy_saving: true,
                }),
                _ => Some(MotionCommand::Stand {
                    head: HeadMotion::LookAt {
                        target: robot_to_field.inverse() * Point2::origin(),
                    },
                    is_energy_saving: true,
                }),
            }
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

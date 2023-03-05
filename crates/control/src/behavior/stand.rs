use nalgebra::Point2;
use spl_network_messages::GamePhase;
use types::{GameControllerState, HeadMotion, MotionCommand, PrimaryState, Role, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        }),
        PrimaryState::Set => {
            let robot_to_field = world_state.robot.robot_to_field?;
            Some(MotionCommand::Stand {
                head: HeadMotion::LookAt {
                    target: robot_to_field.inverse() * Point2::origin(),
                },
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
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

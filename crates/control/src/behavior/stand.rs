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
            if (matches!(
                world_state.game_controller_state,
                Some(GameControllerState {
                    game_phase: GamePhase::PenaltyShootout { .. },
                    ..
                })
            ) && world_state.robot.role == Role::Striker
                && world_state.ball.is_none())
            {
                let robot_to_field = world_state.robot.robot_to_field?;
                Some(MotionCommand::Stand {
                    head: HeadMotion::LookAt {
                        target: robot_to_field.inverse() * Point2::origin(),
                    },
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

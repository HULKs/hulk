use spl_network_messages::GamePhase;
use types::{GameControllerState, MotionCommand, PrimaryState, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.game_controller_state {
        Some(GameControllerState {
            game_phase: GamePhase::PenaltyShootout { .. },
            ..
        }) => None,
        _ => match world_state.robot.primary_state {
            PrimaryState::Ready | PrimaryState::Playing => Some(MotionCommand::Stand {
                head: types::HeadMotion::LookAround,
                is_energy_saving: false,
            }),
            _ => None,
        },
    }
}

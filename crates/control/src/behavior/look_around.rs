use spl_network_messages::GamePhase;
use types::{
    game_controller_state::GameControllerState, motion_command::MotionCommand,
    primary_state::PrimaryState, world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        world_state.game_controller_state,
        world_state.robot.primary_state,
    ) {
        (
            Some(GameControllerState {
                game_phase: GamePhase::PenaltyShootout { .. },
                ..
            }),
            _,
        ) => None,
        (_, PrimaryState::Ready | PrimaryState::Playing) => Some(MotionCommand::Stand {
            head: types::motion_command::HeadMotion::LookAround,
        }),
        _ => None,
    }
}

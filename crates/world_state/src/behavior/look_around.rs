use hsl_network_messages::GamePhase;
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{HeadMotion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        &world_state.filtered_game_controller_state,
        world_state.robot.primary_state,
    ) {
        (
            Some(FilteredGameControllerState {
                game_phase: GamePhase::PenaltyShootout { .. },
                ..
            }),
            _,
        ) => None,
        (_, PrimaryState::Ready | PrimaryState::Set | PrimaryState::Playing) => {
            Some(MotionCommand::Stand {
                head: HeadMotion::LookAround,
            })
        }
        _ => None,
    }
}

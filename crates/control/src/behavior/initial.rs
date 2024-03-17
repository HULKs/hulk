use nalgebra::Point2;
use spl_network_messages::{GameState, PlayerNumber};
use types::{
    camera_position::CameraPosition,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, MotionCommand},
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    expected_referee_position: Point2<f32>,
    referee_pixel_offset: Point2<f32>,
) -> Option<MotionCommand> {
    let filtered_game_controller_state = world_state.filtered_game_controller_state?;

    if filtered_game_controller_state.game_state != FilteredGameState::Initial {
        return None;
    };
    let head = match filtered_game_controller_state.own_team_is_home_after_coin_toss {
        true => match world_state.robot.player_number {
            PlayerNumber::Seven | PlayerNumber::Four | PlayerNumber::Five => HeadMotion::LookAt {
                target: expected_referee_position,
                pixel_target: referee_pixel_offset,
                camera: Some(CameraPosition::Top),
            },
            _ => HeadMotion::ZeroAngles,
        },
        false => match world_state.robot.player_number {
            PlayerNumber::One | PlayerNumber::Two | PlayerNumber::Six => HeadMotion::LookAt {
                target: expected_referee_position,
                pixel_target: referee_pixel_offset,
                camera: Some(CameraPosition::Top),
            },
            _ => HeadMotion::ZeroAngles,
        },
    };
    Some(MotionCommand::Initial { head })
}

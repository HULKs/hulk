use coordinate_systems::Ground;
use linear_algebra::Point2;
use spl_network_messages::PlayerNumber;
use types::{
    camera_position::CameraPosition,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, MotionCommand, PixelTarget},
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    expected_referee_position: Point2<Ground>,
    pixel_target: PixelTarget,
) -> Option<MotionCommand> {
    let filtered_game_controller_state = world_state.filtered_game_controller_state?;

    if filtered_game_controller_state.game_state != FilteredGameState::Initial {
        return None;
    };
    let head = match filtered_game_controller_state.own_team_is_home_after_coin_toss {
        true => match world_state.robot.player_number {
            PlayerNumber::Seven | PlayerNumber::Four | PlayerNumber::Five | PlayerNumber::Three => {
                HeadMotion::LookAt {
                    target: expected_referee_position,
                    pixel_target,
                    camera: Some(CameraPosition::Top),
                }
            }
            _ => HeadMotion::ZeroAngles,
        },
        false => match world_state.robot.player_number {
            PlayerNumber::One | PlayerNumber::Two | PlayerNumber::Six => HeadMotion::LookAt {
                target: expected_referee_position,
                pixel_target,
                camera: Some(CameraPosition::Top),
            },
            _ => HeadMotion::ZeroAngles,
        },
    };
    Some(MotionCommand::Initial { head })
}

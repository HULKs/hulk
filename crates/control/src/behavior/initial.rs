use coordinate_systems::Ground;
use linear_algebra::Point2;
use spl_network_messages::PlayerNumber;
use types::{
    camera_position::CameraPosition,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, ImageRegionTarget, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    expected_referee_position: Option<&Point2<Ground>>,
    image_region_target: ImageRegionTarget,
) -> Option<MotionCommand> {
    let (Some(filtered_game_controller_state), Some(expected_referee_position)) = (
        world_state.filtered_game_controller_state,
        expected_referee_position,
    ) else {
        match world_state.robot.primary_state {
            PrimaryState::Initial => {
                return Some(MotionCommand::Initial {
                    head: HeadMotion::Center,
                })
            }
            _ => return None,
        };
    };

    if filtered_game_controller_state.game_state == FilteredGameState::Initial
        && world_state.robot.primary_state == PrimaryState::Initial
    {
        let head = match filtered_game_controller_state.own_team_is_home_after_coin_toss {
            true => match world_state.robot.player_number {
                PlayerNumber::Seven
                | PlayerNumber::Four
                | PlayerNumber::Five
                | PlayerNumber::Three => HeadMotion::LookAt {
                    target: *expected_referee_position,
                    image_region_target,
                    camera: Some(CameraPosition::Top),
                },
                _ => HeadMotion::ZeroAngles,
            },
            false => match world_state.robot.player_number {
                PlayerNumber::One | PlayerNumber::Two | PlayerNumber::Six => HeadMotion::LookAt {
                    target: *expected_referee_position,
                    image_region_target,
                    camera: Some(CameraPosition::Top),
                },
                _ => HeadMotion::ZeroAngles,
            },
        };
        Some(MotionCommand::Initial { head })
    } else {
        None
    }
}

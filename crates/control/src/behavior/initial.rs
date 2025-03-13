use spl_network_messages::PlayerNumber;
use types::{
    camera_position::CameraPosition,
    field_dimensions::GlobalFieldSide,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState, enable_pose_detection: bool) -> Option<MotionCommand> {
    if world_state.robot.primary_state != PrimaryState::Initial
        && world_state.robot.primary_state != PrimaryState::Standby
    {
        return None;
    }

    if world_state.robot.primary_state == PrimaryState::Initial {
        return Some(MotionCommand::Initial {
            head: HeadMotion::Center,
        });
    }

    let filtered_game_controller_state = world_state.filtered_game_controller_state.clone()?;

    let should_pose_detection_be_active = world_state.robot.primary_state == PrimaryState::Standby
        && filtered_game_controller_state.game_state == FilteredGameState::Standby
        && enable_pose_detection;

    let player_number_should_look_for_referee = matches!(
        (
            world_state.robot.player_number,
            filtered_game_controller_state.global_field_side,
        ),
        (
            PlayerNumber::Four | PlayerNumber::Seven,
            GlobalFieldSide::Home
        ) | (PlayerNumber::Two | PlayerNumber::Six, GlobalFieldSide::Away)
    );

    Some(MotionCommand::Initial {
        head: match (
            should_pose_detection_be_active,
            player_number_should_look_for_referee,
        ) {
            (true, true) => HeadMotion::LookAtReferee {
                image_region_target: ImageRegion::Bottom,
                camera: Some(CameraPosition::Top),
            },
            _ => HeadMotion::Center,
        },
    })
}

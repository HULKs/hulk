use spl_network_messages::PlayerNumber;
use types::{
    camera_position::CameraPosition,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState, enable_pose_detection: bool) -> Option<MotionCommand> {
    if world_state.robot.primary_state == PrimaryState::Initial {
        return Some(MotionCommand::Initial {
            head: HeadMotion::Center,
        });
    }
    if world_state.robot.primary_state == PrimaryState::Standby {
        return Some(
            look_at_referee(world_state.clone(), enable_pose_detection).unwrap_or(
                MotionCommand::Initial {
                    head: HeadMotion::Center,
                },
            ),
        );
    }
    None
}

fn look_at_referee(world_state: WorldState, enable_pose_detection: bool) -> Option<MotionCommand> {
    let filtered_game_controller_state = world_state.filtered_game_controller_state.as_ref()?;
    if !enable_pose_detection
        || filtered_game_controller_state.game_state != FilteredGameState::Standby
    {
        return None;
    }

    match (
        world_state.robot.player_number,
        filtered_game_controller_state.own_team_is_home_after_coin_toss,
    ) {
        (PlayerNumber::Four | PlayerNumber::Seven, true) => {}
        (PlayerNumber::Two | PlayerNumber::Six, false) => {}
        _ => return None,
    }

    Some(MotionCommand::Initial {
        head: HeadMotion::LookAtReferee {
            image_region_target: ImageRegion::Bottom,
            camera: Some(CameraPosition::Top),
        },
    })
}

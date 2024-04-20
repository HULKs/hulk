use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use spl_network_messages::PlayerNumber;
use types::{
    camera_position::CameraPosition,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    expected_referee_position: Option<Point2<Field>>,
) -> Option<MotionCommand> {
    if world_state.robot.primary_state != PrimaryState::Initial {
        return None;
    }

    Some(
        look_at_referee(
            world_state.robot.ground_to_field,
            world_state.filtered_game_controller_state,
            expected_referee_position,
            world_state.clone(),
        )
        .unwrap_or(MotionCommand::Initial {
            head: HeadMotion::Center,
            should_look_for_referee: false,
        }),
    )
}

fn look_at_referee(
    ground_to_field: Option<Isometry2<Ground, Field>>,
    filtered_game_controller_state: Option<FilteredGameControllerState>,
    expected_referee_position: Option<Point2<Field>>,
    world_state: WorldState,
) -> Option<MotionCommand> {
    let ground_to_field = ground_to_field?;
    let expected_referee_position = expected_referee_position?;
    if filtered_game_controller_state?.game_state != FilteredGameState::Initial {
        return None;
    }

    let position = ground_to_field.as_pose().position();

    if position.y().signum() == expected_referee_position.y().signum() {
        return None;
    };

    let own_team_is_home_after_coin_toss = world_state
        .filtered_game_controller_state?
        .own_team_is_home_after_coin_toss;

    match (
        world_state.robot.player_number,
        own_team_is_home_after_coin_toss,
    ) {
        (PlayerNumber::Four | PlayerNumber::Seven, true) => {}
        (PlayerNumber::Two | PlayerNumber::Six, false) => {}
        _ => return None,
    }

    Some(MotionCommand::Initial {
        head: HeadMotion::LookAt {
            target: ground_to_field.inverse() * expected_referee_position,
            image_region_target: ImageRegion::Bottom,
            camera: Some(CameraPosition::Top),
        },
        should_look_for_referee: true,
    })
}

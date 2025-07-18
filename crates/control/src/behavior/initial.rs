use types::{
    camera_position::CameraPosition,
    field_dimensions::GlobalFieldSide,
    filtered_game_state::FilteredGameState,
    initial_pose::InitialPose,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    players::Players,
    primary_state::PrimaryState,
    support_foot::Side,
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    enable_pose_detection: bool,
    initial_poses: &Players<InitialPose>,
) -> Option<MotionCommand> {
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

    let initial_pose_should_look_for_referee = match (
        initial_poses[world_state.robot.player_number].side,
        filtered_game_controller_state.global_field_side,
    ) {
        (Side::Left, GlobalFieldSide::Home) => false,
        (Side::Left, GlobalFieldSide::Away) => true,
        (Side::Right, GlobalFieldSide::Home) => true,
        (Side::Right, GlobalFieldSide::Away) => false,
    };

    Some(MotionCommand::Initial {
        head: match (
            should_pose_detection_be_active,
            initial_pose_should_look_for_referee,
        ) {
            (true, true) => HeadMotion::LookAtReferee {
                image_region_target: ImageRegion::Bottom,
                camera: Some(CameraPosition::Top),
            },
            _ => HeadMotion::Center,
        },
    })
}

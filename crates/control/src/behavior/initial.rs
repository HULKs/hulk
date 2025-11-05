use coordinate_systems::Field;
use linear_algebra::{point, Point2};
use types::{
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
    expected_referee_position: Option<&Point2<Field>>,
    enable_pose_detection: bool,
    initial_poses: &Players<InitialPose>,
) -> Option<MotionCommand> {
    if world_state.robot.primary_state != PrimaryState::Initial
        && world_state.robot.primary_state != PrimaryState::Standby
    {
        return None;
    }

    if world_state.robot.primary_state == PrimaryState::Initial
        && world_state.filtered_game_controller_state.is_none()
    {
        return Some(MotionCommand::Initial {
            head: HeadMotion::Center,
        });
    }

    let filtered_game_controller_state = world_state.filtered_game_controller_state.clone()?;

    let initial_pose_should_look_for_referee = match (
        initial_poses[world_state.robot.player_number].side,
        filtered_game_controller_state.global_field_side,
    ) {
        (Side::Left, GlobalFieldSide::Home) => false,
        (Side::Left, GlobalFieldSide::Away) => true,
        (Side::Right, GlobalFieldSide::Home) => true,
        (Side::Right, GlobalFieldSide::Away) => false,
    };

    let expected_referee_position = world_state.robot.ground_to_field?.inverse()
        * expected_referee_position.unwrap_or(&point!(0.0, 0.0));

    let should_only_look_at_referee = world_state.robot.primary_state == PrimaryState::Initial
        && filtered_game_controller_state.game_state == FilteredGameState::Initial;

    let should_pose_detection_be_active = world_state.robot.primary_state == PrimaryState::Standby
        && filtered_game_controller_state.game_state == FilteredGameState::Standby
        && enable_pose_detection;

    Some(MotionCommand::Initial {
        head: match (
            should_pose_detection_be_active,
            initial_pose_should_look_for_referee,
            should_only_look_at_referee,
        ) {
            (true, true, false) => HeadMotion::LookAtReferee {
                image_region_target: ImageRegion::Bottom,
            },
            (false, true, true) => HeadMotion::LookAt {
                target: expected_referee_position,
                image_region_target: ImageRegion::Bottom,
            },
            _ => HeadMotion::Center,
        },
    })
}

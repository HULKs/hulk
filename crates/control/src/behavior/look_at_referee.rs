use coordinate_systems::Field;
use linear_algebra::Point2;
use types::{
    camera_position::CameraPosition,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    expected_referee_position: Option<Point2<Field>>,
    enable_pose_detection: bool,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let expected_referee_position = expected_referee_position?;

    let position = ground_to_field.as_pose().position();

    if position.y().signum() == expected_referee_position.y().signum() {
        return None;
    };

    Some(MotionCommand::Stand {
        head: HeadMotion::LookAt {
            target: ground_to_field.inverse() * expected_referee_position,
            image_region_target: ImageRegion::Bottom,
            camera: Some(CameraPosition::Top),
        },
        should_look_for_referee: enable_pose_detection,
    })
}

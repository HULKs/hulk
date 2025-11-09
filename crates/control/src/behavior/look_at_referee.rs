use coordinate_systems::{Field, Ground};
use framework::AdditionalOutput;
use linear_algebra::{Point2, Pose2, Rotation2};
use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand, OrientationMode, WalkSpeed},
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::walk_to_pose::WalkAndStand;

pub fn execute(
    enable_pose_detection: bool,
    walk_and_stand: &WalkAndStand,
    expected_referee_position: Option<&Point2<Field>>,
    world_state: &WorldState,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    let pose_looking_at_referee = if let (Some(expected_referee_position), Some(ground_to_field)) =
        (expected_referee_position, world_state.robot.ground_to_field)
    {
        Rotation2::from_vector((ground_to_field.inverse() * expected_referee_position).coords())
            * Pose2::<Ground>::default()
    } else {
        Pose2::<Ground>::default()
    };

    let head_motion = if enable_pose_detection {
        HeadMotion::LookAtReferee {
            image_region_target: ImageRegion::Bottom,
        }
    } else {
        HeadMotion::Center
    };

    walk_and_stand.execute(
        pose_looking_at_referee,
        head_motion,
        path_obstacles_output,
        walk_speed,
        OrientationMode::AlignWithPath,
        distance_to_be_aligned,
        walk_and_stand.parameters.hysteresis,
    )
}

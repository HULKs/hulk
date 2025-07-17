use framework::AdditionalOutput;
use linear_algebra::{point, Pose2};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    field_dimensions: &FieldDimensions,
    walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let kick_off_pose = Pose2::from(point![
        field_dimensions.length / 2.0
            - field_dimensions.penalty_marker_distance
            - field_dimensions.penalty_marker_size * 2.0,
        0.0
    ]);
    walk_and_stand.execute(
        ground_to_field.inverse() * kick_off_pose,
        look_action.execute(),
        path_obstacles_output,
        walk_speed,
        OrientationMode::AlignWithPath,
        distance_to_be_aligned,
        walk_and_stand.parameters.hysteresis,
    )
}

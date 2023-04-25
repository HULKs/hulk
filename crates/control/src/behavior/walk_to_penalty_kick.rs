use framework::AdditionalOutput;
use nalgebra::Isometry2;
use types::{FieldDimensions, MotionCommand, PathObstacle, WorldState};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    field_dimensions: &FieldDimensions,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let kick_off_pose = Isometry2::translation(
        field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance - 0.1,
        0.0,
    );
    walk_and_stand.execute(
        robot_to_field.inverse() * kick_off_pose,
        look_action.execute(),
        path_obstacles_output,
    )
}

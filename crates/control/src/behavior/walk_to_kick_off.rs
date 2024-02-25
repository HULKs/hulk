use coordinate_systems::{Point2, Pose};
use framework::AdditionalOutput;
use types::{
    coordinate_systems::Field, motion_command::MotionCommand, path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    striker_set_position: Point2<Field>,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    walk_and_stand.execute(
        ground_to_field.inverse() * Pose::from_position(striker_set_position),
        look_action.execute(),
        path_obstacles_output,
    )
}

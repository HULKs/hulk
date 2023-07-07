use framework::AdditionalOutput;
use nalgebra::{Translation2, Vector2};
use types::{MotionCommand, PathObstacle, WorldState};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    striker_set_position: Vector2<f32>,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    walk_and_stand.execute(
        robot_to_field.inverse() * Translation2::from(striker_set_position),
        look_action.execute(),
        path_obstacles_output,
    )
}

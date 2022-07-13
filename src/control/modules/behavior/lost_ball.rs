use nalgebra::{vector, Point2};
use types::{rotate_towards, HeadMotion, MotionCommand, OrientationMode, PathObstacle, WorldState};

use crate::framework::AdditionalOutput;

use super::walk_to_pose::WalkPathPlanner;

pub fn execute(
    world_state: &WorldState,
    absolute_last_known_ball_position: Point2<f32>,
    walk_path_planner: &WalkPathPlanner,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let walk_target =
        robot_to_field.inverse() * (absolute_last_known_ball_position - vector![1.0, 0.0]);
    let relative_last_known_ball_position =
        robot_to_field.inverse() * absolute_last_known_ball_position;
    let orientation = rotate_towards(Point2::origin(), relative_last_known_ball_position);
    let path = walk_path_planner.plan(
        walk_target,
        robot_to_field,
        None,
        &world_state.obstacles,
        path_obstacles_output,
    );
    Some(MotionCommand::Walk {
        head: HeadMotion::SearchForLostBall,
        orientation_mode: OrientationMode::Override(orientation),
        path,
    })
}

use coordinate_systems::{IntoFramed, IntoTransform};
use framework::AdditionalOutput;
use geometry::look_at::LookAt;
use nalgebra::Point2;
use types::{
    motion_command::HeadMotion,
    motion_command::{MotionCommand, OrientationMode},
    parameters::LostBallParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::walk_to_pose::WalkPathPlanner;

pub fn execute(
    world_state: &WorldState,
    absolute_last_known_ball_position: Point2<f32>,
    walk_path_planner: &WalkPathPlanner,
    lost_ball_parameters: &LostBallParameters,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    struct Robot;
    struct Field;

    let robot_to_field = world_state
        .robot
        .robot_to_field?
        .framed_transform::<Robot, Field>();
    let absolute_last_known_ball_position = absolute_last_known_ball_position.framed();
    let offset_to_last_ball_location = lost_ball_parameters.offset_to_last_ball_location.framed();
    let walk_target = robot_to_field.inverse()
        * (absolute_last_known_ball_position - offset_to_last_ball_location);
    let relative_last_known_ball_position =
        robot_to_field.inverse() * absolute_last_known_ball_position;

    let orientation = Point2::origin()
        .framed()
        .look_at(&relative_last_known_ball_position);
    let path = walk_path_planner.plan(
        walk_target.inner,
        robot_to_field.inner,
        None,
        1.0,
        &world_state.obstacles,
        &world_state.rule_obstacles,
        path_obstacles_output,
    );
    Some(walk_path_planner.walk_with_obstacle_avoiding_arms(
        HeadMotion::SearchForLostBall,
        OrientationMode::Override(orientation.inner),
        path,
    ))
}

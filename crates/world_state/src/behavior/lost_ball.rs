use coordinate_systems::Field;
use framework::AdditionalOutput;
use linear_algebra::{Orientation2, Point2};
use types::{
    motion_command::{HeadMotion, MotionCommand, OrientationMode, WalkSpeed},
    parameters::LostBallParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::walk_to_pose::WalkPathPlanner;

pub fn execute(
    world_state: &WorldState,
    absolute_last_known_ball_position: Point2<Field>,
    walk_path_planner: &WalkPathPlanner,
    lost_ball_parameters: &LostBallParameters,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let walk_target = ground_to_field.inverse()
        * (absolute_last_known_ball_position - lost_ball_parameters.offset_to_last_ball_location);
    let relative_last_known_ball_position =
        ground_to_field.inverse() * absolute_last_known_ball_position;

    let path = walk_path_planner.plan(
        walk_target,
        ground_to_field,
        None,
        1.0,
        &world_state.obstacles,
        &world_state.rule_obstacles,
        path_obstacles_output,
    );
    let best_hypothetical_ball_position = world_state
        .hypothetical_ball_positions
        .iter()
        .max_by(|a, b| a.validity.total_cmp(&b.validity));

    let head = match best_hypothetical_ball_position {
        Some(hypothesis) => HeadMotion::LookAt {
            target: hypothesis.position,
            image_region_target: Default::default(),
        },
        None => HeadMotion::SearchForLostBall,
    };
    Some(walk_path_planner.walk_with_obstacle_avoiding_arms(
        head,
        OrientationMode::LookAt {
            target: relative_last_known_ball_position,
            tolerance: 0.0,
        },
        Orientation2::identity(),
        distance_to_be_aligned,
        path,
        walk_speed,
    ))
}

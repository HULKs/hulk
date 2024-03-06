use framework::AdditionalOutput;
use spl_network_messages::Team;
use std::f32::consts::PI;
use types::{
    filtered_game_controller_state::FilteredGameControllerState, parameters::DribblingParameters,
    path_obstacles::PathObstacle, planned_path::PathSegment, world_state::WorldState,
};

use crate::behavior::walk_to_pose::WalkPathPlanner;

pub fn plan(
    walk_path_planner: &WalkPathPlanner,
    world_state: &WorldState,
    dribbling_parameters: &DribblingParameters,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<Vec<PathSegment>> {
    let kick_decisions = world_state.kick_decisions.as_ref()?;
    let best_kick_decision = kick_decisions.first()?;
    let ball = world_state.ball?;
    let ground_to_field = world_state.robot.ground_to_field?;

    let ball_position_in_ground = ball.ball_in_ground;
    let ball_position_in_field = ball.ball_in_field;
    let best_pose = best_kick_decision.kick_pose;
    let robot_to_ball = ball_position_in_ground.coords();
    let dribble_pose_to_ball = ball_position_in_ground - best_pose.position();

    let angle = robot_to_ball.angle(dribble_pose_to_ball);
    let should_avoid_ball = angle > dribbling_parameters.angle_to_approach_ball_from_threshold;
    let ball_obstacle = should_avoid_ball.then_some(ball_position_in_ground);

    let ball_is_between_robot_and_own_goal =
        ball_position_in_field.coords().x() - ground_to_field.translation().x() < 0.0;
    let ball_obstacle_radius_factor = if ball_is_between_robot_and_own_goal {
        1.0f32
    } else {
        (angle - dribbling_parameters.angle_to_approach_ball_from_threshold)
            / (PI - dribbling_parameters.angle_to_approach_ball_from_threshold)
    };

    let is_near_ball = matches!(
        world_state.ball,
        Some(ball) if ball.ball_in_ground.coords().norm() < dribbling_parameters.ignore_robot_when_near_ball_radius,
    );
    let obstacles = if is_near_ball {
        &[]
    } else {
        world_state.obstacles.as_slice()
    };

    let rule_obstacles = if matches!(
        world_state.filtered_game_controller_state,
        Some(FilteredGameControllerState {
            kicking_team: Team::Hulks,
            ..
        })
    ) {
        &[]
    } else {
        world_state.rule_obstacles.as_slice()
    };

    Some(walk_path_planner.plan(
        best_pose.position(),
        ground_to_field,
        ball_obstacle,
        ball_obstacle_radius_factor,
        obstacles,
        rule_obstacles,
        path_obstacles_output,
    ))
}

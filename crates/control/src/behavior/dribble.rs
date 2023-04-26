use framework::AdditionalOutput;
use nalgebra::{Isometry2, Point2};
use types::{
    configuration::{Dribbling, InWalkKickInfo, InWalkKicks},
    rotate_towards, HeadMotion, MotionCommand,
    OrientationMode::{self, AlignWithPath},
    PathObstacle, WorldState,
};

use super::walk_to_pose::{hybrid_alignment, WalkPathPlanner};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    world_state: &WorldState,
    walk_path_planner: &WalkPathPlanner,
    in_walk_kicks: &InWalkKicks,
    parameters: &Dribbling,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    let ball_position = world_state.ball?.position;
    let robot_to_field = world_state.robot.robot_to_field?;
    let head = HeadMotion::LookLeftAndRightOf {
        target: ball_position,
    };
    let kick_decisions = world_state.kick_decisions.as_ref()?;

    let available_kick = kick_decisions.iter().find(|decision| {
        is_kick_pose_reached(decision.kick_pose, &in_walk_kicks[decision.variant])
    });
    if let Some(kick) = available_kick {
        let command = MotionCommand::InWalkKick {
            head,
            kick: kick.variant,
            kicking_side: kick.kicking_side,
        };
        return Some(command);
    }

    let best_kick_decision = match kick_decisions.first() {
        Some(decision) => decision,
        None => {
            return Some(MotionCommand::Stand {
                head,
                is_energy_saving: false,
            })
        }
    };

    let best_pose = best_kick_decision.kick_pose;

    let hybrid_orientation_mode = hybrid_alignment(
        best_pose,
        parameters.hybrid_align_distance,
        parameters.distance_to_be_aligned,
    );
    let orientation_mode = match hybrid_orientation_mode {
        AlignWithPath if ball_position.coords.norm() > 0.0 => {
            OrientationMode::Override(rotate_towards(Point2::origin(), ball_position))
        }
        orientation_mode => orientation_mode,
    };

    let robot_to_ball = ball_position.coords;
    let dribble_pose_to_ball = ball_position.coords - best_pose.translation.vector;
    let angle = robot_to_ball.angle(&dribble_pose_to_ball);
    let should_avoid_ball = angle > parameters.angle_to_approach_ball_from_threshold;
    let ball_obstacle = should_avoid_ball.then_some(ball_position);

    let is_near_ball = matches!(
        world_state.ball,
        Some(ball) if ball.position.coords.norm() < parameters.ignore_robot_when_near_ball_radius,
    );
    let obstacles = if is_near_ball {
        &[]
    } else {
        world_state.obstacles.as_slice()
    };
    let path = walk_path_planner.plan(
        best_pose * Point2::origin(),
        robot_to_field,
        ball_obstacle,
        obstacles,
        path_obstacles_output,
    );
    Some(walk_path_planner.walk_with_obstacle_avoiding_arms(head, orientation_mode, path))
}

fn is_kick_pose_reached(kick_pose_to_robot: Isometry2<f32>, kick_info: &InWalkKickInfo) -> bool {
    let is_x_reached = kick_pose_to_robot.translation.x.abs() < kick_info.reached_thresholds.x;
    let is_y_reached = kick_pose_to_robot.translation.y.abs() < kick_info.reached_thresholds.y;
    let is_orientation_reached =
        kick_pose_to_robot.rotation.angle().abs() < kick_info.reached_thresholds.z;
    is_x_reached && is_y_reached && is_orientation_reached
}

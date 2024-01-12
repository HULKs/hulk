use geometry::look_at::LookAt;
use nalgebra::{Isometry2, Point2};

use types::{
    motion_command::{HeadMotion, MotionCommand, OrientationMode},
    parameters::{DribblingParameters, InWalkKickInfoParameters, InWalkKicksParameters},
    planned_path::PathSegment,
    world_state::WorldState,
};

use super::walk_to_pose::{hybrid_alignment, WalkPathPlanner};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    world_state: &WorldState,
    walk_path_planner: &WalkPathPlanner,
    in_walk_kicks: &InWalkKicksParameters,
    parameters: &DribblingParameters,
    dribble_path: Option<Vec<PathSegment>>,
) -> Option<MotionCommand> {
    let ball_position = world_state.ball?.ball_in_ground;
    let head = HeadMotion::LookLeftAndRightOf {
        target: ball_position,
    };
    let kick_decisions = world_state.kick_decisions.as_ref()?;
    let instant_kick_decisions = world_state.instant_kick_decisions.as_ref()?;

    let available_kick = kick_decisions
        .iter()
        .chain(instant_kick_decisions.iter())
        .find(|decision| {
            is_kick_pose_reached(decision.kick_pose, &in_walk_kicks[decision.variant])
        });
    if let Some(kick) = available_kick {
        let command = MotionCommand::InWalkKick {
            head,
            kick: kick.variant,
            kicking_side: kick.kicking_side,
            strength: kick.strength,
        };
        return Some(command);
    }

    let best_kick_decision = match kick_decisions.first() {
        Some(decision) => decision,
        None => return Some(MotionCommand::Stand { head }),
    };

    let best_pose = best_kick_decision.kick_pose;

    let hybrid_orientation_mode = hybrid_alignment(
        best_pose,
        parameters.hybrid_align_distance,
        parameters.distance_to_be_aligned,
    );
    let orientation_mode = match hybrid_orientation_mode {
        types::motion_command::OrientationMode::AlignWithPath
            if ball_position.coords.norm() > 0.0 =>
        {
            OrientationMode::Override(Point2::origin().look_at(&ball_position))
        }
        orientation_mode => orientation_mode,
    };
    match dribble_path {
        Some(path) => {
            Some(walk_path_planner.walk_with_obstacle_avoiding_arms(head, orientation_mode, path))
        }
        None => Some(MotionCommand::Stand { head }),
    }
}

fn is_kick_pose_reached(
    kick_pose_to_robot: Isometry2<f32>,
    kick_info: &InWalkKickInfoParameters,
) -> bool {
    let is_x_reached = kick_pose_to_robot.translation.x.abs() < kick_info.reached_thresholds.x;
    let is_y_reached = kick_pose_to_robot.translation.y.abs() < kick_info.reached_thresholds.y;
    let is_orientation_reached =
        kick_pose_to_robot.rotation.angle().abs() < kick_info.reached_thresholds.z;
    is_x_reached && is_y_reached && is_orientation_reached
}

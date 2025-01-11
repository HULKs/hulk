use coordinate_systems::{Ground, UpcomingSupport};
use geometry::look_at::LookAt;
use linear_algebra::{Isometry2, Point, Pose2};
use spl_network_messages::GamePhase;
use types::{
    camera_position::CameraPosition,
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{
        ArmMotion, HeadMotion, ImageRegion, MotionCommand, OrientationMode, WalkSpeed,
    },
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
    mut walk_speed: WalkSpeed,
) -> Option<MotionCommand> {
    let ball_position = world_state.ball?.ball_in_ground;
    let distance_to_ball = ball_position.coords().norm();
    let head = if distance_to_ball < parameters.distance_to_look_directly_at_the_ball {
        HeadMotion::LookAt {
            target: ball_position,
            image_region_target: ImageRegion::Center,
            camera: Some(CameraPosition::Bottom),
        }
    } else {
        HeadMotion::LookLeftAndRightOf {
            target: ball_position,
        }
    };
    let kick_decisions = world_state.kick_decisions.as_ref()?;
    let instant_kick_decisions = world_state.instant_kick_decisions.as_ref()?;

    let available_kick = kick_decisions
        .iter()
        .chain(instant_kick_decisions.iter())
        .find(|decision| {
            is_kick_pose_reached(
                decision.kick_pose,
                &in_walk_kicks[decision.variant],
                world_state.robot.ground_to_upcoming_support,
            )
        });
    if let Some(kick) = available_kick {
        let command = MotionCommand::InWalkKick {
            head,
            kick: kick.variant,
            kicking_side: kick.kicking_side,
            strength: kick.strength,
            left_arm: ArmMotion::Swing,
            right_arm: ArmMotion::Swing,
        };
        return Some(command);
    }

    let best_kick_decision = match kick_decisions.first() {
        Some(decision) => decision,
        None => {
            return Some(MotionCommand::Stand {
                head,
                should_look_for_referee: false,
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
        types::motion_command::OrientationMode::AlignWithPath
            if ball_position.coords().norm() > 0.0 =>
        {
            OrientationMode::Override(Point::origin().look_at(&ball_position))
        }
        orientation_mode => orientation_mode,
    };

    if let Some(FilteredGameControllerState {
        game_phase: GamePhase::PenaltyShootout { .. },
        ..
    }) = world_state.filtered_game_controller_state
    {
        walk_speed = WalkSpeed::Slow;
    }

    match dribble_path {
        Some(path) => Some(walk_path_planner.walk_with_obstacle_avoiding_arms(
            head,
            orientation_mode,
            path,
            walk_speed,
        )),
        None => Some(MotionCommand::Stand {
            head,
            should_look_for_referee: false,
        }),
    }
}

fn is_kick_pose_reached(
    kick_pose: Pose2<Ground>,
    kick_info: &InWalkKickInfoParameters,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
) -> bool {
    let upcoming_kick_pose = ground_to_upcoming_support * kick_pose;
    let is_x_reached = upcoming_kick_pose.position().x().abs() < kick_info.reached_thresholds.x;
    let is_y_reached = upcoming_kick_pose.position().y().abs() < kick_info.reached_thresholds.y;
    let is_orientation_reached =
        upcoming_kick_pose.orientation().angle().abs() < kick_info.reached_thresholds.z;
    is_x_reached && is_y_reached && is_orientation_reached
}

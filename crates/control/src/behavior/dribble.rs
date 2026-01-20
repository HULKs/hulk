use coordinate_systems::{Ground, UpcomingSupport};
use hsl_network_messages::GamePhase;
use linear_algebra::{Isometry2, Pose2};
use types::{
    dribble_path_plan::DribblePathPlan,
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{ArmMotion, HeadMotion, ImageRegion, MotionCommand, WalkSpeed},
    parameters::{DribblingParameters, InWalkKickInfoParameters, InWalkKicksParameters},
    world_state::WorldState,
};

use super::walk_to_pose::WalkPathPlanner;

pub fn execute(
    world_state: &WorldState,
    walk_path_planner: &WalkPathPlanner,
    in_walk_kicks: &InWalkKicksParameters,
    parameters: &DribblingParameters,
    dribble_path_plan: Option<DribblePathPlan>,
    mut walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    let ball_position = world_state.ball?.ball_in_ground;
    let distance_to_ball = ball_position.coords().norm();
    let head = if distance_to_ball < parameters.distance_to_look_directly_at_the_ball {
        HeadMotion::LookAt {
            target: ball_position,
            image_region_target: ImageRegion::Center,
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

    if let Some(FilteredGameControllerState {
        game_phase: GamePhase::PenaltyShootout { .. },
        ..
    }) = world_state.filtered_game_controller_state
    {
        walk_speed = WalkSpeed::Slow;
    }

    match dribble_path_plan {
        Some(DribblePathPlan {
            orientation_mode,
            target_orientation,
            path,
        }) => Some(walk_path_planner.walk_with_obstacle_avoiding_arms(
            head,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            path,
            walk_speed,
        )),
        None => Some(MotionCommand::Stand { head }),
    }
}

fn is_kick_pose_reached(
    kick_pose: Pose2<Ground>,
    kick_info: &InWalkKickInfoParameters,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
) -> bool {
    let upcoming_kick_pose = ground_to_upcoming_support * kick_pose;
    let is_x_reached = kick_info
        .reached_x
        .contains(&upcoming_kick_pose.position().x());
    let is_y_reached = kick_info
        .reached_y
        .contains(&upcoming_kick_pose.position().y());
    let is_orientation_reached = kick_info
        .reached_turn
        .contains(&upcoming_kick_pose.orientation().angle());
    is_x_reached && is_y_reached && is_orientation_reached
}

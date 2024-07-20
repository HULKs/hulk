use coordinate_systems::{Ground, UpcomingSupport};
use geometry::look_at::LookAt;
use linear_algebra::{Isometry2, Point, Pose2};
use spl_network_messages::{GamePhase, Team};
use std::time::{Duration, SystemTime};
use types::{
    camera_position::CameraPosition,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    motion_command::{
        ArmMotion, HeadMotion, ImageRegion, MotionCommand, OrientationMode, WalkSpeed,
    },
    parameters::{DribblingParameters, InWalkKicksParameters},
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
    game_controller_state: Option<FilteredGameControllerState>,
    precision_kick_timeout: u8,
    bigger_threshold_start_time: Option<SystemTime>,
    cycle_start_time: SystemTime,
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
    let break_precision_kick = break_precision_kick(
        game_controller_state,
        precision_kick_timeout,
        bigger_threshold_start_time,
        cycle_start_time,
    );

    let available_kick = kick_decisions
        .iter()
        .chain(instant_kick_decisions.iter())
        .find(|decision| {
            is_kick_pose_reached(
                decision.kick_pose,
                if break_precision_kick {
                    in_walk_kicks[decision.variant].reached_thresholds
                } else {
                    in_walk_kicks[decision.variant].precision_kick_reached_thresholds
                },
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
        None => Some(MotionCommand::Stand { head }),
    }
}

pub fn is_kick_pose_reached(
    kick_pose: Pose2<Ground>,
    thresholds: nalgebra::Vector3<f32>,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
) -> bool {
    let upcoming_kick_pose = ground_to_upcoming_support * kick_pose;

    let is_x_reached = upcoming_kick_pose.position().x().abs() < thresholds.x;
    let is_y_reached = upcoming_kick_pose.position().y().abs() < thresholds.y;
    let is_orientation_reached = upcoming_kick_pose.orientation().angle().abs() < thresholds.z;

    is_x_reached && is_y_reached && is_orientation_reached
}

pub fn break_precision_kick(
    game_controller_state: Option<FilteredGameControllerState>,
    precision_kick_timeout: u8,
    bigger_threshold_start_time: Option<SystemTime>,
    cycle_start_time: SystemTime,
) -> bool {
    let game_controller_state = game_controller_state.unwrap_or_default();
    let mut time_difference: Duration = Duration::default();

    if bigger_threshold_start_time.is_some() {
        time_difference = cycle_start_time
            .duration_since(bigger_threshold_start_time.unwrap())
            .expect("Time ran back");
    };

    let precision_kick = matches!(
        game_controller_state.game_phase,
        GamePhase::PenaltyShootout { .. }
    ) || game_controller_state.sub_state.is_some();

    let own_kick_off = matches!(
        game_controller_state.game_state,
        FilteredGameState::Playing {
            kick_off: true,
            ball_is_free: true
        }
    );
    // dbg!(time_difference);
    // dbg!(Duration::from_secs(precision_kick_timeout.into()));
    // dbg!(time_difference > Duration::from_secs(precision_kick_timeout.into()));
    let sub_state = game_controller_state.sub_state.is_some();
    let kicking = matches!(game_controller_state.kicking_team, Team::Hulks);
    (precision_kick || own_kick_off || sub_state && kicking)
        && time_difference > Duration::from_secs(precision_kick_timeout.into())
}

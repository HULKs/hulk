use std::f32::consts::PI;

use coordinate_systems::{Field, Ground};
use geometry::look_at::LookAt;
use linear_algebra::{Isometry2, Point, Pose2};
use serde::{Deserialize, Serialize};

use color_eyre::{eyre::Ok, Result};
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use spl_network_messages::Team;
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    kick_decision::KickDecision,
    motion_command::{MotionCommand, OrientationMode},
    obstacles::Obstacle,
    parameters::{DribblingParameters, PathPlanningParameters},
    path_obstacles::PathObstacle,
    planned_path::PathSegment,
    rule_obstacles::RuleObstacle,
    world_state::BallState,
};

use crate::behavior::walk_to_pose::{hybrid_alignment, WalkPathPlanner};

#[derive(Deserialize, Serialize)]
pub struct DribblePathPlanner {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball: RequiredInput<Option<BallState>, "ball_state?">,
    kick_decisions: RequiredInput<Option<Vec<KickDecision>>, "kick_decisions?">,
    ground_to_field: RequiredInput<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    rule_obstacles: Input<Vec<RuleObstacle>, "rule_obstacles">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,

    dribbling_parameters: Parameter<DribblingParameters, "behavior.dribbling">,
    path_planning_parameters: Parameter<PathPlanningParameters, "behavior.path_planning">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    dribble_path_obstacles_output: AdditionalOutput<Vec<PathObstacle>, "dribble_path_obstacles">,

    last_motion_command: CyclerState<MotionCommand, "last_motion_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub dribble_path_plan: MainOutput<Option<(OrientationMode, Vec<PathSegment>)>>,
}

impl DribblePathPlanner {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let walk_path_planner = WalkPathPlanner::new(
            context.field_dimensions,
            context.obstacles,
            context.path_planning_parameters,
            context.last_motion_command,
        );

        let best_kick_decision = match context.kick_decisions.first() {
            Some(decision) => decision,
            None => {
                return Ok(MainOutputs {
                    dribble_path_plan: None.into(),
                })
            }
        };
        let best_kick_pose = best_kick_decision.kick_pose;

        let mut dribble_path_obstacles = None;
        let mut dribble_path_obstacles_output = AdditionalOutput::new(
            context.dribble_path_obstacles_output.is_subscribed()
                || context.dribble_path_obstacles_output.is_subscribed(),
            &mut dribble_path_obstacles,
        );

        let Some(dribble_path) = plan(
            &walk_path_planner,
            *context.ball,
            best_kick_pose,
            *context.ground_to_field,
            context.obstacles,
            context.rule_obstacles,
            context.filtered_game_controller_state,
            context.dribbling_parameters,
            &mut dribble_path_obstacles_output,
        ) else {
            return Ok(MainOutputs {
                dribble_path_plan: None.into(),
            });
        };
        context
            .dribble_path_obstacles_output
            .fill_if_subscribed(|| dribble_path_obstacles.clone().unwrap_or_default());

        let hybrid_orientation_mode = hybrid_alignment(
            best_kick_pose,
            context.dribbling_parameters.hybrid_align_distance,
            context.dribbling_parameters.distance_to_be_aligned,
        );
        let ball_position = &context.ball.ball_in_ground;
        let orientation_mode = match hybrid_orientation_mode {
            types::motion_command::OrientationMode::AlignWithPath
                if ball_position.coords().norm() > 0.0 =>
            {
                OrientationMode::Override(Point::origin().look_at(ball_position))
            }
            orientation_mode => orientation_mode,
        };

        Ok(MainOutputs {
            dribble_path_plan: Some((orientation_mode, dribble_path)).into(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn plan(
    walk_path_planner: &WalkPathPlanner,
    ball: BallState,
    best_pose: Pose2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
    obstacles: &[Obstacle],
    rule_obstacles: &[RuleObstacle],
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    dribbling_parameters: &DribblingParameters,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<Vec<PathSegment>> {
    let robot_to_ball = ball.ball_in_ground.coords();
    let dribble_pose_to_ball = ball.ball_in_ground - best_pose.position();
    let angle = robot_to_ball.angle(&dribble_pose_to_ball);
    let should_avoid_ball = angle > dribbling_parameters.angle_to_approach_ball_from_threshold;
    let ball_obstacle = should_avoid_ball.then_some(ball.ball_in_ground);

    let ball_is_between_robot_and_own_goal =
        ball.ball_in_field.coords().x() - ground_to_field.translation().x() < 0.0;
    let ball_obstacle_radius_factor = if ball_is_between_robot_and_own_goal {
        1.0f32
    } else {
        (angle - dribbling_parameters.angle_to_approach_ball_from_threshold)
            / (PI - dribbling_parameters.angle_to_approach_ball_from_threshold)
    };

    let ball_is_near = ball.ball_in_ground.coords().norm()
        < dribbling_parameters.ignore_robot_when_near_ball_radius;
    let hulks_is_kicking_team =
        filtered_game_controller_state.is_some_and(|filtered_game_controller_state| {
            filtered_game_controller_state.kicking_team == Team::Hulks
        });

    let obstacles = if ball_is_near { &[] } else { obstacles };
    let rule_obstacles = if hulks_is_kicking_team {
        &[]
    } else {
        rule_obstacles
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

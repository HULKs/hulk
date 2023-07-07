use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::Point2;
use spl_network_messages::Team;
use std::f32::consts::PI;
use types::{
    parameters::Behavior, FieldDimensions, GameControllerState, PathObstacle, PathSegment,
    WorldState,
};

use crate::behavior::walk_to_pose::WalkPathPlanner;

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub parameters: Parameter<Behavior, "behavior">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub path_obstacles: AdditionalOutput<Vec<PathObstacle>, "time_to_reach_obstacles">,
    pub world_state: Input<WorldState, "world_state">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub dribble_path: MainOutput<Option<Vec<PathSegment>>>,
}

pub struct DribblePath {}
impl DribblePath {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let path_planning_parameters = &context.parameters.path_planning;
        let field_dimensions = context.field_dimensions;
        let dribbling_parameters = &context.parameters.dribbling;
        let world_state = context.world_state;

        let path_obstacles_output = &mut context.path_obstacles;

        let walk_path_planner = WalkPathPlanner::new(
            field_dimensions,
            &world_state.obstacles,
            path_planning_parameters,
        );

        let Some(kick_decisions) = world_state.kick_decisions.as_ref() else { return Ok(MainOutputs::default()) };
        let Some(best_kick_decision) = kick_decisions.first() else { return Ok(MainOutputs::default()) };
        let (ball_position_in_ground, ball_position_in_field) = match world_state.ball {
            Some(ball_position) => (ball_position.ball_in_ground, ball_position.ball_in_field),
            None => return Ok(MainOutputs::default()),
        };
        let best_pose = best_kick_decision.kick_pose;
        let Some(robot_to_field) = world_state.robot.robot_to_field else { return Ok(MainOutputs::default()) };
        let robot_to_ball = ball_position_in_ground.coords;
        let dribble_pose_to_ball = ball_position_in_ground.coords - best_pose.translation.vector;

        let angle = robot_to_ball.angle(&dribble_pose_to_ball);
        let should_avoid_ball = angle > dribbling_parameters.angle_to_approach_ball_from_threshold;
        let ball_obstacle = should_avoid_ball.then_some(ball_position_in_ground);

        let ball_is_between_robot_and_own_goal =
            ball_position_in_field.coords.x - robot_to_field.translation.x < 0.0f32;
        let ball_obstacle_radius_factor = if ball_is_between_robot_and_own_goal {
            1.0f32
        } else {
            (angle - dribbling_parameters.angle_to_approach_ball_from_threshold)
                / (PI - dribbling_parameters.angle_to_approach_ball_from_threshold)
        };

        let is_near_ball = matches!(
            world_state.ball,
            Some(ball) if ball.ball_in_ground.coords.norm() < dribbling_parameters.ignore_robot_when_near_ball_radius,
        );
        let obstacles = if is_near_ball {
            &[]
        } else {
            world_state.obstacles.as_slice()
        };

        let rule_obstacles = if matches!(
            world_state.game_controller_state,
            Some(GameControllerState {
                kicking_team: Team::Hulks,
                ..
            })
        ) {
            &[]
        } else {
            world_state.rule_obstacles.as_slice()
        };

        let path = Some(walk_path_planner.plan(
            best_pose * Point2::origin(),
            robot_to_field,
            ball_obstacle,
            ball_obstacle_radius_factor,
            obstacles,
            rule_obstacles,
            path_obstacles_output,
        ));
        Ok(MainOutputs {
            dribble_path: path.into(),
        })
    }
}

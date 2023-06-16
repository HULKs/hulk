use color_eyre::Result;
use nalgebra::Point2;
use spl_network_messages::Team;
use std::{f32::consts::PI, time::Duration};

use crate::behavior::walk_to_pose::WalkPathPlanner;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{
    configuration::{Behavior, Dribbling, PathPlanning},
    FieldDimensions, GameControllerState, PathObstacle, PathSegment, WorldState,
};
#[context]
pub struct CycleContext {
    pub world_state: Input<WorldState, "world_state">,
    pub time_to_reach_kick_position: PersistentState<Duration, "time_to_reach_kick_position">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub configuration: Parameter<Behavior, "behavior">,
    pub parameters: Parameter<Behavior, "behavior">,
    pub path_obstacles: AdditionalOutput<Vec<PathObstacle>, "time_to_reach_obstacles">,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct MainOutputs {
    pub time_to_reach_kick_position: MainOutput<Option<Duration>>,
}

pub struct TimeToReachKickPosition {}

impl TimeToReachKickPosition {
    pub fn new(_: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let path = self.plan_path(
            context.world_state,
            &context.configuration.path_planning,
            &context.configuration.dribbling,
            &mut context.path_obstacles,
            context.field_dimensions,
        );

        let time_to_reach_kick_position = path
            .as_ref()
            .map(|path| {
                path.iter()
                    .map(|segment: &PathSegment| segment.length())
                    .sum()
            })
            .map(Duration::from_secs_f32);

        *context.time_to_reach_kick_position =
            time_to_reach_kick_position.unwrap_or(Duration::from_secs(1900));
        Ok(MainOutputs {
            time_to_reach_kick_position: time_to_reach_kick_position.into(),
        })
    }
    pub fn plan_path(
        &mut self,
        world_state: &WorldState,
        configuration: &PathPlanning,
        parameters: &Dribbling,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        field_dimensions: &FieldDimensions,
    ) -> Option<Vec<PathSegment>> {
        let walk_path_planner =
            WalkPathPlanner::new(field_dimensions, &world_state.obstacles, configuration);
        let kick_decisions = world_state.kick_decisions.as_ref()?;
        let best_kick_decision = match kick_decisions.first() {
            Some(decision) => decision,
            None => return None,
        };
        let ball_position = world_state.ball?.ball_in_ground;
        let best_pose = best_kick_decision.kick_pose;
        let robot_to_field = world_state.robot.robot_to_field?;
        let robot_to_ball = ball_position.coords;
        let dribble_pose_to_ball = ball_position.coords - best_pose.translation.vector;
        let angle = robot_to_ball.angle(&dribble_pose_to_ball);
        let should_avoid_ball = angle > parameters.angle_to_approach_ball_from_threshold;
        let ball_obstacle = should_avoid_ball.then_some(ball_position);
        let ball_obstacle_radius_factor = (angle
            - parameters.angle_to_approach_ball_from_threshold)
            / (PI - parameters.angle_to_approach_ball_from_threshold);

        let is_near_ball = matches!(
            world_state.ball,
            Some(ball) if ball.ball_in_ground.coords.norm() < parameters.ignore_robot_when_near_ball_radius,
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
        let path = walk_path_planner.plan(
            best_pose * Point2::origin(),
            robot_to_field,
            ball_obstacle,
            ball_obstacle_radius_factor,
            obstacles,
            rule_obstacles,
            path_obstacles_output,
        );
        Some(path)
    }
}

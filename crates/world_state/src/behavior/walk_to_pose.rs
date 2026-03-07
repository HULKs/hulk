use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_relative_hysteresis;
use framework::AdditionalOutput;
use linear_algebra::{Isometry2, Point, Point2, Pose2, Vector2, point};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::{HeadMotion, MotionCommand},
    obstacles::Obstacle,
    parameters::{PathPlanningParameters, WalkAndStandParameters, WalkToPoseParameters},
    path_obstacles::PathObstacle,
    planned_path::{Path, direct_path},
    rule_obstacles::RuleObstacle,
    world_state::WorldState,
};

use crate::path_planner::PathPlanner;

pub struct WalkPathPlanner<'cycle> {
    field_dimensions: &'cycle FieldDimensions,
    parameters: &'cycle PathPlanningParameters,
    last_motion_command: &'cycle MotionCommand,
}

impl<'cycle> WalkPathPlanner<'cycle> {
    pub fn new(
        field_dimensions: &'cycle FieldDimensions,
        parameters: &'cycle PathPlanningParameters,
        last_motion_command: &'cycle MotionCommand,
    ) -> Self {
        Self {
            field_dimensions,
            parameters,
            last_motion_command,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn plan(
        &self,
        target_in_ground: Point2<Ground>,
        ground_to_field: Isometry2<Ground, Field>,
        ball_obstacle: Option<Point2<Ground>>,
        ball_obstacle_radius_factor: f32,
        obstacles: &[Obstacle],
        rule_obstacles: &[RuleObstacle],
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Path {
        let mut planner = PathPlanner::default();
        planner.with_last_motion(
            self.last_motion_command,
            self.parameters.rotation_penalty_factor,
        );
        planner.with_obstacles(obstacles, self.parameters.robot_radius_at_hip_height);
        planner.with_rule_obstacles(
            ground_to_field.inverse(),
            rule_obstacles,
            self.parameters.robot_radius_at_hip_height,
        );
        planner.with_field_borders(
            ground_to_field,
            self.field_dimensions.length,
            self.field_dimensions.width,
            self.field_dimensions.border_strip_width,
            self.parameters.field_border_weight,
        );
        planner.with_goal_support_structures(ground_to_field.inverse(), self.field_dimensions);
        if let Some(ball_position) = ball_obstacle {
            let foot_proportion = self.parameters.minimum_robot_radius_at_foot_height
                / self.parameters.robot_radius_at_foot_height;
            let calculated_robot_radius_at_foot_height =
                self.parameters.robot_radius_at_foot_height
                    * ((ball_obstacle_radius_factor * (1.0 - foot_proportion)) + foot_proportion);
            planner.with_ball(
                ball_position,
                self.parameters.ball_obstacle_radius,
                calculated_robot_radius_at_foot_height,
            );
        }

        let target_in_field = ground_to_field * target_in_ground;
        let x_max = self.field_dimensions.length / 2.0 + self.field_dimensions.border_strip_width;
        let y_max = self.field_dimensions.width / 2.0 + self.field_dimensions.border_strip_width;
        let clamped_target_in_robot = ground_to_field.inverse()
            * point![
                target_in_field.x().clamp(-x_max, x_max),
                target_in_field.y().clamp(-y_max, y_max)
            ];

        let path = planner
            .plan(Point::origin(), clamped_target_in_robot)
            .unwrap();
        path_obstacles_output.fill_if_subscribed(|| planner.obstacles.clone());
        path.unwrap_or_else(|| direct_path(Point::origin(), target_in_ground))
    }
}

pub struct WalkAndStand<'cycle> {
    world_state: &'cycle WorldState,
    pub walk_and_stand_parameters: &'cycle WalkAndStandParameters,
    pub walk_to_pose_parameters: &'cycle WalkToPoseParameters,
    walk_path_planner: &'cycle WalkPathPlanner<'cycle>,
    last_motion_command: &'cycle MotionCommand,
}

impl<'cycle> WalkAndStand<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        walk_and_stand_parameters: &'cycle WalkAndStandParameters,
        walk_to_pose_parameters: &'cycle WalkToPoseParameters,
        walk_path_planner: &'cycle WalkPathPlanner<'cycle>,
        last_motion_command: &'cycle MotionCommand,
    ) -> Self {
        Self {
            world_state,
            walk_and_stand_parameters,
            walk_to_pose_parameters,
            walk_path_planner,
            last_motion_command,
        }
    }

    pub fn execute(
        &self,
        target_pose: Pose2<Ground>,
        head: HeadMotion,
        walk_to_pose_state: &mut WalkToPoseState,
        cycle_time: f32,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        hysteresis: nalgebra::Vector2<f32>,
    ) -> Option<MotionCommand> {
        let distance_to_walk = target_pose.position().coords().norm();
        let angle_to_walk = target_pose.orientation().angle();
        let was_standing_last_cycle =
            matches!(self.last_motion_command, MotionCommand::Stand { .. });

        let is_reached = less_than_with_relative_hysteresis(
            was_standing_last_cycle,
            distance_to_walk,
            self.walk_and_stand_parameters.target_reached_thresholds.x,
            0.0..=hysteresis.x,
        ) && less_than_with_relative_hysteresis(
            was_standing_last_cycle,
            angle_to_walk.abs(),
            self.walk_and_stand_parameters.target_reached_thresholds.y,
            0.0..=hysteresis.y,
        );

        if is_reached {
            walk_to_pose_state.reset();
            Some(MotionCommand::Stand { head })
        } else {
            walk_to_pose_state.walk_to(
                target_pose,
                cycle_time,
                head,
                self.walk_to_pose_parameters,
                self.walk_path_planner,
                &self.world_state.obstacles,
                &self.world_state.rule_obstacles,
                path_obstacles_output,
                self.world_state,
            )
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct WalkToPoseState {
    previous_position_error: Vector2<Ground>,
    previous_angle_error: f32,
}

impl Default for WalkToPoseState {
    fn default() -> Self {
        Self {
            previous_position_error: Vector2::zeros(),
            previous_angle_error: 0.0,
        }
    }
}

impl WalkToPoseState {
    #[expect(clippy::too_many_arguments)]
    pub fn walk_to(
        &mut self,
        target_pose: Pose2<Ground>,
        cycle_time: f32,
        head: HeadMotion,
        parameters: &WalkToPoseParameters,
        walk_path_planner: &WalkPathPlanner,
        obstacles: &[Obstacle],
        rule_obstacles: &[RuleObstacle],
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        world_state: &WorldState,
    ) -> Option<MotionCommand> {
        let ground_to_field = world_state.robot.ground_to_field?;

        let path = walk_path_planner.plan(
            target_pose.position(),
            ground_to_field,
            world_state.ball.map(|ball| ball.ball_in_ground),
            1.0,
            obstacles,
            rule_obstacles,
            path_obstacles_output,
        );

        let walk_direction = path.direction();
        let distance_to_target = target_pose.position().coords().norm();
        let position_error = walk_direction * distance_to_target;

        let angle_error = target_pose.orientation().angle();

        let d_position = if cycle_time > 0.0 {
            (position_error - self.previous_position_error) / cycle_time
        } else {
            Vector2::zeros()
        };
        let d_angle = if cycle_time > 0.0 {
            (angle_error - self.previous_angle_error) / cycle_time
        } else {
            0.0
        };

        self.previous_position_error = position_error;
        self.previous_angle_error = angle_error;

        let velocity =
            position_error * parameters.translation_p + d_position * parameters.translation_d;

        let speed = velocity.norm();
        let velocity = if speed > parameters.max_speed {
            velocity * (parameters.max_speed / speed)
        } else {
            velocity
        };

        let angular_velocity = (angle_error * parameters.rotation_p
            + d_angle * parameters.rotation_d)
            .clamp(-parameters.max_turn, parameters.max_turn);

        Some(MotionCommand::WalkWithVelocity {
            head,
            velocity,
            angular_velocity,
        })
    }

    pub fn reset(&mut self) {
        self.previous_position_error = Vector2::zeros();
        self.previous_angle_error = 0.0;
    }
}

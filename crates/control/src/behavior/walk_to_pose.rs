use std::f32::consts::PI;

use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_hysteresis;
use framework::AdditionalOutput;
use linear_algebra::{point, Isometry2, Orientation2, Point, Point2, Pose};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::ArmMotion,
    motion_command::MotionCommand,
    motion_command::{HeadMotion, OrientationMode},
    obstacles::Obstacle,
    parameters::{PathPlanningParameters, WalkAndStandParameters},
    path_obstacles::PathObstacle,
    planned_path::{direct_path, PathSegment},
    rule_obstacles::RuleObstacle,
    support_foot::Side,
    world_state::WorldState,
};

use crate::path_planner::PathPlanner;

pub struct WalkPathPlanner<'cycle> {
    field_dimensions: &'cycle FieldDimensions,
    obstacles: &'cycle [Obstacle],
    parameters: &'cycle PathPlanningParameters,
    last_motion_command: &'cycle MotionCommand,
}

impl<'cycle> WalkPathPlanner<'cycle> {
    pub fn new(
        field_dimensions: &'cycle FieldDimensions,
        obstacles: &'cycle [Obstacle],
        parameters: &'cycle PathPlanningParameters,
        last_motion_command: &'cycle MotionCommand,
    ) -> Self {
        Self {
            field_dimensions,
            obstacles,
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
    ) -> Vec<PathSegment> {
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
        path.unwrap_or_else(|| direct_path(Point::origin(), Point::origin()))
    }

    pub fn walk_with_obstacle_avoiding_arms(
        &self,
        head: HeadMotion,
        orientation_mode: OrientationMode,
        path: Vec<PathSegment>,
    ) -> MotionCommand {
        MotionCommand::Walk {
            head,
            orientation_mode,
            path,
            left_arm: self.arm_motion_with_obstacles(Side::Left),
            right_arm: self.arm_motion_with_obstacles(Side::Right),
        }
    }

    fn arm_motion_with_obstacles(&self, side: Side) -> ArmMotion {
        if self.obstacles.iter().any(|obstacle| {
            let is_on_relevant_side = match side {
                Side::Left => obstacle.position.y().is_sign_positive(),
                Side::Right => obstacle.position.y().is_sign_negative(),
            };
            is_on_relevant_side
                && obstacle.position.x().abs() < 0.5
                && obstacle.position.y().abs() < 0.5
        }) {
            ArmMotion::PullTight
        } else {
            ArmMotion::Swing
        }
    }
}

pub struct WalkAndStand<'cycle> {
    world_state: &'cycle WorldState,
    parameters: &'cycle WalkAndStandParameters,
    walk_path_planner: &'cycle WalkPathPlanner<'cycle>,
    last_motion_command: &'cycle MotionCommand,
}

impl<'cycle> WalkAndStand<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        parameters: &'cycle WalkAndStandParameters,
        walk_path_planner: &'cycle WalkPathPlanner,
        last_motion_command: &'cycle MotionCommand,
    ) -> Self {
        Self {
            world_state,
            parameters,
            walk_path_planner,
            last_motion_command,
        }
    }

    pub fn execute(
        &self,
        target_pose: Pose<Ground>,
        head: HeadMotion,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let ground_to_field = self.world_state.robot.ground_to_field?;
        let distance_to_walk = target_pose.position().coords().norm();
        let angle_to_walk = target_pose.orientation().angle();
        let was_standing_last_cycle =
            matches!(self.last_motion_command, MotionCommand::Stand { .. });
        let is_reached = less_than_with_hysteresis(
            was_standing_last_cycle,
            distance_to_walk,
            self.parameters.target_reached_thresholds.x + self.parameters.hysteresis.x,
            self.parameters.hysteresis.x,
        ) && less_than_with_hysteresis(
            was_standing_last_cycle,
            angle_to_walk.abs(),
            self.parameters.target_reached_thresholds.y + self.parameters.hysteresis.y,
            self.parameters.hysteresis.y,
        );
        let orientation_mode = hybrid_alignment(
            target_pose,
            self.parameters.hybrid_align_distance,
            self.parameters.distance_to_be_aligned,
        );

        if is_reached {
            Some(MotionCommand::Stand { head })
        } else {
            let path = self.walk_path_planner.plan(
                target_pose.position(),
                ground_to_field,
                self.world_state.ball.map(|ball| ball.ball_in_ground),
                1.0,
                &self.world_state.obstacles,
                &self.world_state.rule_obstacles,
                path_obstacles_output,
            );
            Some(self.walk_path_planner.walk_with_obstacle_avoiding_arms(
                head,
                orientation_mode,
                path,
            ))
        }
    }
}

pub fn hybrid_alignment(
    target_pose: Pose<Ground>,
    hybrid_align_distance: f32,
    distance_to_be_aligned: f32,
) -> OrientationMode {
    assert!(hybrid_align_distance > distance_to_be_aligned);
    let distance_to_target = target_pose.position().coords().norm();
    if distance_to_target >= hybrid_align_distance {
        return OrientationMode::AlignWithPath;
    }

    let angle_limit = ((distance_to_target - distance_to_be_aligned)
        / (hybrid_align_distance - distance_to_be_aligned))
        .clamp(0.0, 1.0)
        * PI;

    let orientation = clamp_around(
        Orientation2::identity(),
        target_pose.orientation(),
        angle_limit,
    );

    OrientationMode::Override(orientation)
}

pub fn clamp_around(
    input: Orientation2<Ground>,
    center: Orientation2<Ground>,
    angle_limit: f32,
) -> Orientation2<Ground> {
    let center_to_input = center.rotation_to(input);
    let clamped = center_to_input.clamp_angle::<Ground>(-angle_limit, angle_limit);

    clamped * center
}

#[cfg(test)]
mod test {
    use super::*;

    use std::f32::consts::{FRAC_PI_2, PI};

    use approx::assert_relative_eq;

    #[test]
    fn clamp_noop_when_less_than_limit_around_center() {
        let testcases = [
            (0.0, 0.0),
            (0.0, PI),
            (1.0, FRAC_PI_2),
            (-1.0, FRAC_PI_2),
            (FRAC_PI_2, FRAC_PI_2),
            (-FRAC_PI_2, FRAC_PI_2),
        ];

        for (input, angle_limit) in testcases {
            let input = Orientation2::new(input);
            let center = Orientation2::new(0.0);
            assert_relative_eq!(clamp_around(input, center, angle_limit), input);
        }
    }

    #[test]
    fn clamp_clamps_to_limit_around_center() {
        let testcases = [
            (0.0, 0.0),
            (PI, PI),
            (2.0, FRAC_PI_2),
            (-2.0, FRAC_PI_2),
            (FRAC_PI_2, FRAC_PI_2),
            (-FRAC_PI_2, FRAC_PI_2),
            (PI, FRAC_PI_2),
            (-PI, FRAC_PI_2),
        ];

        for (input, angle_limit) in testcases {
            let input = Orientation2::new(input);
            let center = Orientation2::new(0.0);
            assert_relative_eq!(
                clamp_around(input, center, angle_limit).angle().abs(),
                angle_limit
            );
        }
    }

    #[test]
    fn clamped_always_closer_than_limit() {
        let angles = [0.0, PI, -PI, FRAC_PI_2, -FRAC_PI_2, 1.0, -1.0, 2.0, -2.0];

        for input in angles {
            for center in angles {
                for angle_limit in angles {
                    let angle_limit = angle_limit.abs();
                    let input = Orientation2::new(input);
                    let center = Orientation2::new(center);
                    let output = clamp_around(input, center, angle_limit);

                    assert!(center.rotation_to(output).angle().abs() <= angle_limit);
                }
            }
        }
    }
}

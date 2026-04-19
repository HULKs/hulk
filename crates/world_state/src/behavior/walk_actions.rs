use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_relative_hysteresis;
use linear_algebra::{Isometry2, Orientation2, Point, Point2, Pose2, Vector2, point, vector};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
    path::{Path, direct_path},
};

use crate::{behavior::node::Blackboard, path_planner::PathPlanner};

#[allow(clippy::too_many_arguments)]
pub fn plan(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Path {
    let parameters: &types::parameters::PathPlanningParameters =
        &blackboard.parameters.path_planning;
    let field_dimensions = blackboard.field_dimensions;

    let mut planner = PathPlanner::default();
    planner.with_last_motion(
        &blackboard.last_motion_command,
        parameters.rotation_penalty_factor,
    );
    planner.with_obstacles(
        &blackboard.world_state.obstacles,
        parameters.robot_radius_at_hip_height,
    );
    planner.with_rule_obstacles(
        ground_to_field.inverse(),
        &blackboard.world_state.rule_obstacles,
        parameters.robot_radius_at_hip_height,
    );
    planner.with_field_borders(
        ground_to_field,
        field_dimensions.length,
        field_dimensions.width,
        field_dimensions.border_strip_width,
        parameters.field_border_weight,
    );
    planner.with_goal_support_structures(ground_to_field.inverse(), &field_dimensions);
    let ball_obstacle = blackboard.world_state.ball.map(|ball| ball.ball_in_ground);

    if let Some(ball_position) = ball_obstacle {
        let foot_proportion =
            parameters.minimum_robot_radius_at_foot_height / parameters.robot_radius_at_foot_height;
        let calculated_robot_radius_at_foot_height = parameters.robot_radius_at_foot_height
            * ((parameters.ball_obstacle_radius_factor * (1.0 - foot_proportion))
                + foot_proportion);
        planner.with_ball(
            ball_position,
            parameters.ball_obstacle_radius,
            calculated_robot_radius_at_foot_height,
        );
    }

    let target_in_field = ground_to_field * target_in_ground;
    let x_max = field_dimensions.length / 2.0 + field_dimensions.border_strip_width;
    let y_max = field_dimensions.width / 2.0 + field_dimensions.border_strip_width;
    let clamped_target_in_robot = ground_to_field.inverse()
        * point![
            target_in_field.x().clamp(-x_max, x_max),
            target_in_field.y().clamp(-y_max, y_max)
        ];

    let path = planner
        .plan(Point::origin(), clamped_target_in_robot)
        .unwrap();
    blackboard.path_obstacles_output = planner.obstacles;
    path.unwrap_or_else(|| direct_path(Point::origin(), target_in_ground))
}

pub fn walk_to(
    blackboard: &mut Blackboard,
    target_pose: Pose2<Ground>,
    walk_speed: f32,
    orientation_mode: OrientationMode,
    distance_to_be_aligned: f32,
    hysteresis: nalgebra::Vector2<f32>,
) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let parameters = &blackboard.parameters.walk_and_stand;
        let distance_to_walk = target_pose.position().coords().norm();
        let angle_to_walk = target_pose.orientation().angle();
        let was_standing_last_cycle =
            matches!(blackboard.last_motion_command, MotionCommand::Stand { .. });
        let is_reached = less_than_with_relative_hysteresis(
            was_standing_last_cycle,
            distance_to_walk,
            parameters.target_reached_thresholds.x,
            0.0..=hysteresis.x,
        ) && less_than_with_relative_hysteresis(
            was_standing_last_cycle,
            angle_to_walk.abs(),
            parameters.target_reached_thresholds.y,
            0.0..=hysteresis.y,
        );

        if is_reached {
            blackboard.body_motion = Some(BodyMotion::Stand);
            Status::Success
        } else {
            let path = plan(blackboard, target_pose.position(), ground_to_field);
            blackboard.body_motion = Some(BodyMotion::Walk {
                path,
                orientation_mode,
                target_orientation: target_pose.orientation(),
                distance_to_be_aligned,
                speed: walk_speed,
            });
            Status::Success
        }
    } else {
        Status::Failure
    }
}

pub fn walk_to_ball(blackboard: &mut Blackboard) -> Status {
    if let Some(ball) = &blackboard.world_state.ball {
        walk_to(
            blackboard,
            Pose2::from(ball.ball_in_ground),
            blackboard.parameters.walk_speed.kicking,
            OrientationMode::AlignWithPath,
            blackboard
                .parameters
                .walk_and_stand
                .normal_distance_to_be_aligned,
            blackboard.parameters.walk_and_stand.hysteresis,
        )
    } else {
        Status::Failure
    }
}

pub fn walk_instead_of_kicking(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.last_ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let field_to_ground = ground_to_field.inverse();
        let ball_in_ground = field_to_ground * ball.position;

        let goal_position: Vector2<Field> = vector!(blackboard.field_dimensions.length / 2.0, 0.0);

        let kick_direction =
            Orientation2::from_vector(field_to_ground * goal_position - ball_in_ground.coords());

        walk_to(
            blackboard,
            Pose2::from(ball_in_ground),
            blackboard.parameters.walk_speed.kicking,
            OrientationMode::LookTowards {
                direction: kick_direction,
                tolerance: blackboard
                    .parameters
                    .walk_and_stand
                    .normal_distance_to_be_aligned,
            },
            blackboard
                .parameters
                .walk_and_stand
                .normal_distance_to_be_aligned,
            blackboard.parameters.walk_and_stand.hysteresis,
        )
    } else {
        Status::Failure
    }
}

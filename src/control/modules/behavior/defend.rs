use std::ops::Range;

use nalgebra::{point, Isometry2, Point2, UnitComplex, Vector2};

use crate::{
    framework::configuration::RolePositions,
    types::{BallState, FieldDimensions, Line, Side, WorldState},
};

pub fn defend_left_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
        })
        .unwrap_or_default();

    let position_to_defend = point![
        -field_dimensions.length / 2.0,
        role_positions.defender_y_offset
    ];
    let distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let defend_pose = block_on_circle(ball.position, position_to_defend, distance_to_target);
    Some(robot_to_field.inverse() * defend_pose)
}

pub fn defend_right_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
        })
        .unwrap_or_default();

    let position_to_defend = point![
        -field_dimensions.length / 2.0,
        -role_positions.defender_y_offset
    ];
    let distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let defend_pose = block_on_circle(ball.position, position_to_defend, distance_to_target);
    Some(robot_to_field.inverse() * defend_pose)
}

pub fn defend_goal_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
        })
        .unwrap_or_default();

    let position_to_defend = point![-field_dimensions.length / 2.0 - 1.0, 0.0];
    let defend_pose = block_on_line(
        ball.position,
        position_to_defend,
        -field_dimensions.length / 2.0 + role_positions.keeper_x_offset,
        -0.7..0.7,
    );
    Some(robot_to_field.inverse() * defend_pose)
}

fn face_towards(origin: Point2<f32>, target: Point2<f32>) -> UnitComplex<f32> {
    let origin_to_target = target - origin;
    UnitComplex::rotation_between(&Vector2::x(), &origin_to_target)
}

fn block_on_circle(
    ball_position: Point2<f32>,
    target: Point2<f32>,
    distance_to_target: f32,
) -> Isometry2<f32> {
    let target_to_ball = ball_position - target;
    let block_position = target + target_to_ball.normalize() * distance_to_target;
    Isometry2::new(
        block_position.coords,
        face_towards(block_position, ball_position).angle(),
    )
}

fn block_on_line(
    ball_position: Point2<f32>,
    target: Point2<f32>,
    defense_line_x: f32,
    defense_line_y_range: Range<f32>,
) -> Isometry2<f32> {
    let is_ball_in_front_of_defense_line = defense_line_x < ball_position.x;
    if is_ball_in_front_of_defense_line {
        let defense_line = Line(
            point![defense_line_x, defense_line_y_range.start],
            point![defense_line_x, defense_line_y_range.end],
        );
        let ball_target_line = Line(ball_position, target);
        let intersection_point = defense_line.intersection(&ball_target_line);
        let defense_position = point![
            intersection_point.x,
            intersection_point
                .y
                .clamp(defense_line_y_range.start, defense_line_y_range.end)
        ];
        Isometry2::new(
            defense_position.coords,
            face_towards(defense_position, ball_position).angle(),
        )
    } else {
        let defense_position = point![
            defense_line_x,
            (defense_line_y_range.start + defense_line_y_range.end) / 2.0
        ];
        Isometry2::new(
            defense_position.coords,
            face_towards(defense_position, ball_position).angle(),
        )
    }
}

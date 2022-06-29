use std::f32::consts::FRAC_PI_4;

use nalgebra::{point, Isometry2, Point2, UnitComplex, Vector2};

use crate::{
    framework::configuration::RolePositions,
    types::{BallState, FieldDimensions, Side, WorldState},
};

pub fn support_striker_pose(
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
    let side = ball.field_side.opposite();
    let offset_vector = UnitComplex::new(match side {
        Side::Left => -FRAC_PI_4,
        Side::Right => FRAC_PI_4,
    }) * -Vector2::x();
    let supporting_position = ball.position + offset_vector;
    let clamped_position = point![
        supporting_position.x.clamp(
            role_positions.striker_supporter_minimum_x,
            field_dimensions.length / 2.0
        ),
        supporting_position.y
    ];

    let support_pose = Isometry2::new(
        clamped_position.coords,
        face_towards(clamped_position, ball.position).angle(),
    );
    Some(robot_to_field.inverse() * support_pose)
}

fn face_towards(origin: Point2<f32>, target: Point2<f32>) -> UnitComplex<f32> {
    let origin_to_target = target - origin;
    UnitComplex::rotation_between(&Vector2::x(), &origin_to_target)
}

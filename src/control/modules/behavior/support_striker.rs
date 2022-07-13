use std::f32::consts::FRAC_PI_4;

use nalgebra::{point, Isometry2, UnitComplex, Vector2};
use types::{
    rotate_towards, BallState, FieldDimensions, FilteredGameState, MotionCommand, PathObstacle,
    Side, WorldState,
};

use crate::framework::{configuration::RolePositions, AdditionalOutput};

use super::{head::look_for_ball, walk_to_pose::WalkAndStand};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
    walk_and_stand: &WalkAndStand,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    let pose = support_striker_pose(world_state, field_dimensions, role_positions)?;
    walk_and_stand.execute(pose, look_for_ball(world_state.ball), path_obstacles_output)
}

fn support_striker_pose(
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
    }) * -(Vector2::x() * role_positions.striker_supporter_distance_to_ball);
    let supporting_position = ball.position + offset_vector;
    let clamped_x = match world_state.filtered_game_state {
        Some(FilteredGameState::Ready { .. })
        | Some(FilteredGameState::Playing {
            ball_is_free: false,
        }) => supporting_position.x.clamp(
            role_positions
                .striker_supporter_minimum_x
                .min(role_positions.striker_supporter_maximum_x_in_ready_and_when_ball_is_not_free),
            role_positions
                .striker_supporter_minimum_x
                .max(role_positions.striker_supporter_maximum_x_in_ready_and_when_ball_is_not_free),
        ),
        _ => supporting_position.x.clamp(
            role_positions.striker_supporter_minimum_x,
            field_dimensions.length / 2.0,
        ),
    };
    let clamped_position = point![clamped_x, supporting_position.y];
    let support_pose = Isometry2::new(
        clamped_position.coords,
        rotate_towards(clamped_position, ball.position).angle(),
    );
    Some(robot_to_field.inverse() * support_pose)
}

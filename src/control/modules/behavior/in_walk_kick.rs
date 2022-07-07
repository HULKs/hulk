use std::f32::consts::FRAC_2_PI;

use nalgebra::{point, vector, Rotation2, UnitComplex};
use types::{FieldDimensions, HeadMotion, KickVariant, MotionCommand, Side, WorldState};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state.ball?;
    let relative_ball = ball.position;
    let absolute_ball = robot_to_field * relative_ball;
    let opponent_goal = point![field_dimensions.length / 2.0 + 0.2, 0.0];
    let ball_to_goal = opponent_goal - absolute_ball;

    let left_forward_offset = vector![-0.25, 0.035];
    let right_forward_offset = vector![-0.25, -0.035];
    let left_turn_offset = vector![-0.2, -0.04];
    let right_turn_offset = vector![-0.2, 0.04];

    let forward_kick_direction = vector![1.0, 0.0];
    let left_turn_kick_direction = Rotation2::new(FRAC_2_PI) * vector![1.0, 0.0];
    let right_turn_kick_direction = Rotation2::new(-FRAC_2_PI) * vector![1.0, 0.0];

    let absolute_forward_kick_direction = robot_to_field * forward_kick_direction;
    let absolute_left_turn_kick_direction = robot_to_field * left_turn_kick_direction;
    let absolute_right_turn_kick_direction = robot_to_field * right_turn_kick_direction;

    let forward_kick_direction_angle_deviation =
        UnitComplex::rotation_between(&ball_to_goal, &absolute_forward_kick_direction)
            .angle()
            .abs();
    let left_turn_kick_direction_angle_deviation =
        UnitComplex::rotation_between(&ball_to_goal, &absolute_left_turn_kick_direction)
            .angle()
            .abs();
    let right_turn_kick_direction_angle_deviation =
        UnitComplex::rotation_between(&ball_to_goal, &absolute_right_turn_kick_direction)
            .angle()
            .abs();

    let to_left_forward = (relative_ball + left_forward_offset).coords;
    let to_right_forward = (relative_ball + right_forward_offset).coords;
    let to_left_turn = (relative_ball + left_turn_offset).coords;
    let to_right_turn = (relative_ball + right_turn_offset).coords;

    let (kick, kicking_side) = if to_left_forward.x.abs() < 0.1
        && to_left_forward.y.abs() < 0.05
        && forward_kick_direction_angle_deviation < 0.2
    {
        (KickVariant::Forward, Side::Left)
    } else if to_right_forward.x.abs() < 0.1
        && to_right_forward.y.abs() < 0.05
        && forward_kick_direction_angle_deviation < 0.2
    {
        (KickVariant::Forward, Side::Right)
    } else if to_left_turn.x.abs() < 0.1
        && to_left_turn.y.abs() < 0.05
        && left_turn_kick_direction_angle_deviation < 0.3
    {
        (KickVariant::Turn, Side::Left)
    } else if to_right_turn.x.abs() < 0.1
        && to_right_turn.y.abs() < 0.05
        && right_turn_kick_direction_angle_deviation < 0.3
    {
        (KickVariant::Turn, Side::Right)
    } else {
        return None;
    };

    Some(MotionCommand::InWalkKick {
        head: HeadMotion::LookAt {
            target: relative_ball,
        },
        kick,
        kicking_side,
    })
}

use nalgebra::{point, Point2, UnitComplex};

use crate::types::{FieldDimensions, HeadMotion, KickVariant, MotionCommand, Side, WorldState};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state.ball?;
    let relative_ball = ball.position;
    let absolute_ball = robot_to_field * relative_ball;
    let opponent_goal = point![field_dimensions.length / 2.0 + 0.2, 0.0];
    let goal_to_ball = absolute_ball - opponent_goal;
    let ball_to_robot = robot_to_field * (Point2::origin() - relative_ball);
    let is_robot_behind_ball = UnitComplex::rotation_between(&goal_to_ball, &ball_to_robot)
        .angle()
        .abs()
        < 0.5
        && (0.05..0.2).contains(&relative_ball.coords.x);

    if !is_robot_behind_ball {
        return None;
    }

    let kick_side = match relative_ball.coords.y {
        y if (0.01..=0.15).contains(&y) => Side::Left,
        y if (-0.15..=-0.01).contains(&y) => Side::Right,
        _ => return None,
    };

    Some(MotionCommand::InWalkKick {
        head: HeadMotion::LookAt {
            target: relative_ball,
        },
        kick: KickVariant::Forward,
        kicking_side: kick_side,
    })
}

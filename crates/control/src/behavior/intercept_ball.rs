use nalgebra::{point, Point2, UnitComplex};
use types::{
    HeadMotion, LineSegment, MotionCommand, OrientationMode, PathSegment, PrimaryState, Role,
    WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        world_state.robot.primary_state,
        world_state.ball,
        world_state.robot.robot_to_field,
    ) {
        (PrimaryState::Playing, Some(ball), Some(robot_to_field)) => {
            // Ball has to be in front of robot
            if ball.ball_in_ground.coords.norm() > 2.0
                || ball.ball_in_ground.x < 0.0
                || ball.ball_in_ground.y.abs() > 0.5
            {
                return None;
            }
            // Ball has to be moving towards robot
            if ball.ball_in_ground_velocity.x > -0.15 {
                return None;
            }
            // Ball has to be moving towards own goal
            if (robot_to_field * ball.ball_in_ground_velocity).x > -0.05 {
                return None;
            }

            // Only keeper
            if !matches!(
                world_state.robot.role,
                Role::Keeper | Role::ReplacementKeeper | Role::DefenderLeft | Role::DefenderRight
            ) {
                return None;
            }

            let intercept_position = ball.ball_in_ground.y
                - ball.ball_in_ground.x * ball.ball_in_ground_velocity.y
                    / ball.ball_in_ground_velocity.x;
            Some(MotionCommand::Walk {
                head: HeadMotion::LookAt {
                    target: ball.ball_in_ground,
                },
                path: vec![PathSegment::LineSegment(LineSegment(
                    Point2::origin(),
                    point![0.0, intercept_position],
                ))],
                left_arm: types::ArmMotion::Swing,
                right_arm: types::ArmMotion::Swing,
                orientation_mode: OrientationMode::Override(UnitComplex::default()),
            })
        }
        _ => None,
    }
}

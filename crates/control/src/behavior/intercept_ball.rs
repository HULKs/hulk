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
            let ball_in_front_of_robot = ball.ball_in_ground.coords.norm() < 2.0
                && ball.ball_in_ground.x > 0.0
                && ball.ball_in_ground.y.abs() < 0.5;
            let ball_moving_towards_robot = ball.ball_in_ground_velocity.x < -0.15;
            let ball_moving_towards_own_half =
                (robot_to_field * ball.ball_in_ground_velocity).x < -0.05;

            if !(ball_in_front_of_robot
                && ball_moving_towards_robot
                && ball_moving_towards_own_half)
            {
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

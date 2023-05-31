use nalgebra::{point, Point2, UnitComplex};
use types::{
    configuration::InterceptBall, HeadMotion, LineSegment, MotionCommand, OrientationMode,
    PathSegment, PrimaryState, WorldState,
};

pub fn execute(world_state: &WorldState, parameters: InterceptBall) -> Option<MotionCommand> {
    match (
        world_state.robot.primary_state,
        world_state.ball,
        world_state.robot.robot_to_field,
    ) {
        (PrimaryState::Playing, Some(ball), Some(robot_to_field)) => {
            let ball_in_front_of_robot = ball.ball_in_ground.coords.norm()
                < parameters.maximum_distance_to_ball
                && ball.ball_in_ground.x > 0.0
                && ball.ball_in_ground.y.abs() < 0.5;
            let ball_moving_towards_robot =
                ball.ball_in_ground_velocity.x < -parameters.minimum_ball_towards_robot_velocity;
            let ball_moving_towards_own_half = (robot_to_field * ball.ball_in_ground_velocity).x
                < -parameters.minimum_ball_towards_own_half_velocity;

            if !(ball_in_front_of_robot
                && ball_moving_towards_robot
                && ball_moving_towards_own_half)
            {
                return None;
            }

            let time_to_intercept_point = ball.ball_in_ground.x / ball.ball_in_ground_velocity.x;
            let intercept_position =
                ball.ball_in_ground.y - ball.ball_in_ground_velocity.y * time_to_intercept_point;

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

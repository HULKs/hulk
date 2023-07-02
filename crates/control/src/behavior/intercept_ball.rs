use nalgebra::{Isometry2, Point2, UnitComplex, Vector2};
use spl_network_messages::{GamePhase, SubState};
use types::{
    parameters::InterceptBall, Arc, BallState, Circle, FilteredGameState, GameControllerState,
    HeadMotion, LineSegment, MotionCommand, Orientation, OrientationMode, PathSegment, Step,
    WorldState,
};

pub fn execute(
    world_state: &WorldState,
    parameters: InterceptBall,
    maximum_step_size: Step,
    current_step: Step,
) -> Option<MotionCommand> {
    if let Some(
        GameControllerState {
            game_phase: GamePhase::PenaltyShootout { .. },
            ..
        }
        | GameControllerState {
            sub_state: Some(SubState::PenaltyKick),
            ..
        },
    ) = world_state.game_controller_state
    {
        return None;
    }
    match (
        world_state.filtered_game_state,
        world_state.ball,
        world_state.robot.robot_to_field,
    ) {
        (
            Some(FilteredGameState::Playing { ball_is_free: true }),
            Some(ball),
            Some(robot_to_field),
        ) => {
            if !ball_is_interception_candidate(ball, robot_to_field, &parameters) {
                return None;
            }

            let Step {
                forward,
                left,
                turn: _,
            } = maximum_step_size;

            if forward == 0.0 || left == 0.0 {
                return None;
            }

            let normalized_velocity = ball.ball_in_ground_velocity.normalize();

            // Find the point with the least distance from the line traversed by the ball
            let optimal_interception_point = ball.ball_in_ground
                - ball.ball_in_ground.coords.dot(&normalized_velocity) * normalized_velocity;

            if optimal_interception_point.coords.norm() > parameters.maximum_intercept_distance {
                return None;
            }

            let walking_direction = Vector2::new(current_step.forward, current_step.left);
            let path =
                get_interception_path(optimal_interception_point, walking_direction, &parameters);

            Some(MotionCommand::Walk {
                head: HeadMotion::LookAt {
                    target: ball.ball_in_ground,
                    camera: None,
                },
                path,
                left_arm: types::ArmMotion::Swing,
                right_arm: types::ArmMotion::Swing,
                orientation_mode: OrientationMode::Override(UnitComplex::default()),
            })
        }
        _ => None,
    }
}

fn ball_is_interception_candidate(
    ball: BallState,
    robot_to_field: Isometry2<f32>,
    parameters: &InterceptBall,
) -> bool {
    let ball_in_front_of_robot = ball.ball_in_ground.coords.norm()
        < parameters.maximum_ball_distance
        && ball.ball_in_ground.x > 0.0;
    let ball_moving_towards_robot =
        ball.ball_in_ground_velocity.x < -parameters.minimum_ball_velocity_towards_robot;

    let ball_in_field_velocity = robot_to_field * ball.ball_in_ground_velocity;
    let ball_moving = ball_in_field_velocity.norm() > parameters.minimum_ball_velocity;
    let ball_moving_towards_own_half =
        ball_in_field_velocity.x < -parameters.minimum_ball_velocity_towards_own_half;

    ball_in_front_of_robot
        && ball_moving
        && ball_moving_towards_robot
        && ball_moving_towards_own_half
}

fn get_interception_path(
    optimal_interception_point: Point2<f32>,
    walking_direction: Vector2<f32>,
    parameters: &InterceptBall,
) -> Vec<PathSegment> {
    if walking_direction.norm() < parameters.minimum_arc_radius {
        vec![PathSegment::LineSegment(LineSegment(
            Point2::origin(),
            optimal_interception_point,
        ))]
    } else {
        // If we are moving, we can not change the direction instantaneously without
        // slowing down. Instead, traverse an Arc with radius dependent on the current
        // speed until the direction is changed.
        let arc_radius = walking_direction.norm();
        let (arc, arc_orientation) = calculate_arc_tangent_to(
            walking_direction,
            optimal_interception_point.coords,
            arc_radius,
        );

        let interception_point = optimal_interception_point + (arc.end - arc.circle.center);

        vec![
            PathSegment::Arc(arc, arc_orientation),
            PathSegment::LineSegment(LineSegment(arc.end, interception_point)),
        ]
    }
}

fn calculate_arc_tangent_to(
    vector1: Vector2<f32>,
    vector2: Vector2<f32>,
    radius: f32,
) -> (Arc, Orientation) {
    let normal_vector1 = Vector2::new(vector1.y, -vector1.x).normalize();
    let normal_vector2 = Vector2::new(vector2.y, -vector2.x).normalize();

    let start = Point2::origin();

    let arc_orientation =
        Orientation::triangle_orientation(start, start + vector1, start + vector1 + vector2);

    let sign = match arc_orientation {
        Orientation::Clockwise => -1.0,
        Orientation::Counterclockwise => 1.0,
        Orientation::Colinear => 1.0,
    };

    let arc_center = start - sign * radius * normal_vector1;
    let end_point = arc_center + sign * radius * normal_vector2;

    (
        Arc::new(Circle::new(arc_center, radius), Point2::origin(), end_point),
        arc_orientation,
    )
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::vector;
    use types::{Arc, Orientation};

    use super::calculate_arc_tangent_to;

    #[test]
    fn arc_is_tangent() {
        let vector1 = vector![-1.4, 3.1];
        let vector2 = vector![0.9, -2.1];

        let (arc, _) = calculate_arc_tangent_to(vector1, vector2, 3.0);
        let Arc { start, circle, end } = arc;
        let center = circle.center;

        assert_relative_eq!((start - center).dot(&vector1), 0.0, epsilon = 0.00001);
        assert_relative_eq!((end - center).dot(&vector2), 0.0, epsilon = 0.00001);
    }

    #[test]
    fn colinear_arc_has_same_start_end() {
        let vector1 = vector![-3.1, 1.9];
        let vector2 = vector![-6.2, 3.8];

        let (arc, _) = calculate_arc_tangent_to(vector1, vector2, 3.0);

        assert_relative_eq!(arc.start, arc.end);
    }

    #[test]
    fn arc_orientation() {
        let vector1 = vector![1.3, 4.8];
        let vector2 = vector![1.2, 5.7];

        let (_, orientation1) = calculate_arc_tangent_to(vector1, vector2, 3.0);
        let (_, orientation2) = calculate_arc_tangent_to(vector2, vector1, 3.0);

        assert_eq!(orientation1, Orientation::Counterclockwise);
        assert_eq!(orientation2, Orientation::Clockwise);
    }
}

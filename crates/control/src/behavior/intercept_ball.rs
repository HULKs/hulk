use coordinate_systems::{Field, Ground};
use geometry::line_segment::LineSegment;
use geometry::{line::Line, look_at::LookAt};
use hsl_network_messages::{GamePhase, SubState};
use linear_algebra::{Isometry2, Point};
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand, OrientationMode, WalkSpeed},
    parameters::InterceptBallParameters,
    planned_path::{Path, PathSegment},
    world_state::{BallState, WorldState},
};

pub fn execute(
    world_state: &WorldState,
    parameters: InterceptBallParameters,
    walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    if let Some(
        FilteredGameControllerState {
            game_phase: GamePhase::PenaltyShootout { .. },
            ..
        }
        | FilteredGameControllerState {
            sub_state: Some(SubState::PenaltyKick),
            ..
        },
    ) = world_state.filtered_game_controller_state
    {
        return None;
    }

    let filtered_game_state = world_state
        .filtered_game_controller_state
        .as_ref()
        .map(|filtered_game_controller_state| filtered_game_controller_state.game_state);
    match (
        filtered_game_state,
        world_state.ball,
        world_state.robot.ground_to_field,
    ) {
        (
            Some(FilteredGameState::Playing {
                ball_is_free: true, ..
            })
            | None,
            Some(ball),
            Some(ground_to_field),
        ) => {
            if !ball_is_interception_candidate(ball, ground_to_field, &parameters) {
                return None;
            }

            let ball_line = Line {
                point: ball.ball_in_ground,
                direction: ball.ball_in_ground_velocity,
            };
            let interception_point = ball_line.closest_point(Point::origin());

            if interception_point.coords().norm() > parameters.maximum_intercept_distance {
                return None;
            }

            let path = Path {
                segments: vec![PathSegment::LineSegment(LineSegment(
                    Point::origin(),
                    interception_point,
                ))],
            };

            let target_orientation = interception_point.look_at(&ball.ball_in_ground);
            Some(MotionCommand::Walk {
                head: HeadMotion::LookAt {
                    target: ball.ball_in_ground,
                    image_region_target: ImageRegion::Center,
                },
                path,
                left_arm: types::motion_command::ArmMotion::Swing,
                right_arm: types::motion_command::ArmMotion::Swing,
                orientation_mode: OrientationMode::LookTowards {
                    direction: target_orientation,
                    tolerance: 0.0,
                },
                speed: walk_speed,
                target_orientation,
                distance_to_be_aligned,
            })
        }
        _ => None,
    }
}

fn ball_is_interception_candidate(
    ball: BallState,
    ground_to_field: Isometry2<Ground, Field>,
    parameters: &InterceptBallParameters,
) -> bool {
    let ball_is_in_front_of_robot = ball.ball_in_ground.coords().norm()
        < parameters.maximum_ball_distance
        && ball.ball_in_ground.x() > 0.0;
    let ball_is_moving_towards_robot =
        ball.ball_in_ground_velocity.x() < -parameters.minimum_ball_velocity_towards_robot;

    let ball_in_field_velocity = ground_to_field * ball.ball_in_ground_velocity;
    let ball_is_moving = ball_in_field_velocity.norm() > parameters.minimum_ball_velocity;
    let ball_is_moving_towards_own_half =
        ball_in_field_velocity.x() < -parameters.minimum_ball_velocity_towards_own_half;

    ball_is_in_front_of_robot
        && ball_is_moving
        && ball_is_moving_towards_robot
        && ball_is_moving_towards_own_half
}

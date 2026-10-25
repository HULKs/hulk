use coordinate_systems::Field;
use hsl_network_messages::SubState;
use linear_algebra::{Orientation2, Point2, Pose2, Vector2, point};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, OrientationMode},
    motion_type::MotionType,
    path::PathSegment,
};

use crate::{
    action,
    behavior_tree::Node,
    condition,
    conditions::{hulks_is_kicking_team, is_closest_to_ball},
    head::look_at_ball_subtree,
    kick::{apply_kick_target, disable_strong_kick, intercept, kick, kick_alternatives_subtree},
    negation,
    node::Blackboard,
    selection, sequence,
    substates::{is_in_sub_state, is_sub_state},
    subtree,
    switch_motion_type::{is_allowed_to_start_kick, is_last_motion_type, switch_motion_type},
    tree::striker_subtree,
    voronoi::calculate_voronoi_grid,
    walk::{
        adaptive_blocking_velocity, walk_alternatives_subtree, walk_to, walk_to_block_position,
    },
};

pub fn goalkeeper_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(
                condition!(is_in_sub_state),
                subtree!(goalkeeper_sub_state_subtree)
            ),
            sequence!(
                condition!(is_goalkeeper_kick_away_needed),
                switch_motion_type(
                    MotionType::Kick,
                    sequence!(
                        action!(kick),
                        action!(select_goalkeeper_kick_away_target),
                        action!(disable_strong_kick),
                    ),
                    subtree!(kick_alternatives_subtree),
                )
            ),
            sequence!(
                condition!(is_goalkeeper_interception_candidate),
                selection!(
                    condition!(is_last_motion_type, MotionType::Kick),
                    condition!(is_allowed_to_start_kick),
                ),
                action!(kick),
                action!(intercept),
                action!(disable_strong_kick),
            ),
            subtree!(goalkeeper_shot_block_position_subtree),
            sequence!(
                action!(calculate_voronoi_grid),
                condition!(is_closest_to_ball),
                subtree!(striker_subtree),
            ),
            sequence!(
                condition!(is_ball_near_own_goal),
                subtree!(goalkeeper_active_defense_position_subtree),
            ),
            subtree!(goalkeeper_default_position_subtree),
        ),
    )
}

fn goalkeeper_sub_state_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_sub_state, SubState::PenaltyKick),
            negation!(condition!(hulks_is_kicking_team)),
            switch_motion_type(
                MotionType::Walk,
                action!(walk_to_goalkeeper_penalty_position),
                subtree!(walk_alternatives_subtree),
            )
        ),
        sequence!(
            negation!(condition!(hulks_is_kicking_team)),
            subtree!(goalkeeper_shot_block_position_subtree),
        ),
        sequence!(
            negation!(condition!(hulks_is_kicking_team)),
            selection!(
                condition!(is_sub_state, SubState::CornerKick),
                sequence!(
                    selection!(
                        condition!(is_sub_state, SubState::ThrowIn),
                        condition!(is_sub_state, SubState::IndirectFreeKick),
                        condition!(is_sub_state, SubState::DirectFreeKick),
                    ),
                    condition!(is_ball_near_own_goal)
                ),
            ),
            subtree!(goalkeeper_active_defense_position_subtree)
        ),
        sequence!(
            condition!(hulks_is_kicking_team),
            condition!(is_sub_state, SubState::GoalKick),
            action!(calculate_voronoi_grid),
            condition!(is_closest_to_ball),
            subtree!(striker_subtree),
        ),
        subtree!(goalkeeper_default_position_subtree),
    )
}

fn goalkeeper_active_defense_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        sequence!(
            action!(set_goalkeeper_active_defense_position),
            action!(walk_to_block_position)
        ),
        Node::Failure,
    )
}

fn goalkeeper_default_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(walk_to_goalkeeper_default_position),
        subtree!(walk_alternatives_subtree),
    )
}

fn is_goalkeeper_interception_candidate(blackboard: &mut Blackboard) -> bool {
    if goalkeeper_shot_block_position(blackboard).is_none() {
        return false;
    }

    let Some(ball_velocity) = ball_interception_velocity_in_field(blackboard) else {
        return false;
    };

    if !is_interception_velocity_towards_own_half(blackboard, ball_velocity) {
        return false;
    }

    if let Some(ball) = &blackboard.ball {
        let field_dimensions = blackboard.field_dimensions;
        let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field else {
            return false;
        };

        let own_goal_x = -field_dimensions.length / 2.0;
        let interception_line_x = own_goal_x + blackboard.parameters.goalkeeper.x_offset;
        let time_to_interception_line =
            (interception_line_x - ball.position.x()) / ball_velocity.x();

        if time_to_interception_line <= 0.0 {
            return false;
        }

        let y_at_interception_line =
            ball.position.y() + ball_velocity.y() * time_to_interception_line;
        let goal_half_width = field_dimensions.goal_inner_width / 2.0
            + field_dimensions.goal_post_diameter / 2.0
            + field_dimensions.ball_radius;

        if y_at_interception_line.abs() >= goal_half_width {
            return false;
        }

        let ball_in_ground = ground_to_field.inverse() * ball.position;
        if ball_in_ground.coords().norm()
            > blackboard
                .parameters
                .ball
                .interception
                .maximum_ball_distance
                .min(blackboard.parameters.kicking.kick_activation_distance)
        {
            return false;
        }
        let velocity = ground_to_field.inverse() * ball.velocity;
        let time_to_closest_approach =
            -ball_in_ground.coords().dot(&velocity) / velocity.norm_squared();
        if time_to_closest_approach < 0.0 {
            return false;
        }

        let interception_point = ball_in_ground + velocity * time_to_closest_approach;
        interception_point.x()
            >= blackboard
                .parameters
                .kicking
                .minimum_interception_forward_distance
            && interception_point.coords().norm()
                <= blackboard
                    .parameters
                    .ball
                    .interception
                    .maximum_intercept_distance
    } else {
        false
    }
}

fn is_goalkeeper_kick_away_needed(blackboard: &mut Blackboard) -> bool {
    if !is_ball_in_own_penalty_area(blackboard)
        || goalkeeper_shot_block_position(blackboard).is_some()
    {
        return false;
    }

    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;

        ball_in_ground.x() > 0.0
            && ball_in_ground.coords().norm()
                < blackboard
                    .parameters
                    .goalkeeper
                    .kick_away_ball_maximum_robot_distance
    } else {
        false
    }
}

fn is_ball_in_own_penalty_area(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let field_dimensions = blackboard.field_dimensions;
        let own_penalty_area_x =
            -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length;

        ball.position.x() < own_penalty_area_x
            && ball.position.y().abs() < field_dimensions.penalty_area_width / 2.0
    })
}

fn is_ball_near_own_goal(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let own_goal_x = -blackboard.field_dimensions.length / 2.0;
        let maximum_ball_x = own_goal_x + blackboard.parameters.goalkeeper.passive_distance;

        ball.position.x() < maximum_ball_x
    })
}
pub fn set_goalkeeper_active_defense_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let field_dimensions = blackboard.field_dimensions;
        let parameters = &blackboard.parameters.goalkeeper;

        let own_goal_line_x = -field_dimensions.length / 2.0;
        let own_goal_center = point!(own_goal_line_x, 0.0);
        let goal_to_ball = ball.position - own_goal_center;
        let defense_radius =
            (field_dimensions.goal_inner_width / 2.0 - parameters.distance_to_goalpost).max(0.0);
        let unclamped_defense_position_in_field = if goal_to_ball.norm() < f32::EPSILON {
            point!(own_goal_line_x + defense_radius, 0.0)
        } else {
            own_goal_center + goal_to_ball.normalize() * defense_radius
        };
        let minimum_defense_x = own_goal_line_x + parameters.distance_to_goalpost;
        let defense_position_in_field = point!(
            unclamped_defense_position_in_field
                .x()
                .max(minimum_defense_x),
            unclamped_defense_position_in_field.y()
        );
        let defense_position_in_ground = ground_to_field.inverse() * defense_position_in_field;

        if defense_position_in_ground.coords().norm()
            > parameters.active_defense_maximum_robot_distance
        {
            return Status::Failure;
        }

        blackboard.walk_position = Some(defense_position_in_ground);

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn walk_to_goalkeeper_default_position(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let field_dimensions = blackboard.field_dimensions;
        let own_goal_line_x = -field_dimensions.length / 2.0;
        let default_position_in_field = point!(
            own_goal_line_x + blackboard.parameters.goalkeeper.x_offset,
            0.0
        );
        let default_pose_in_field =
            Pose2::from_parts(default_position_in_field, Orientation2::new(0.0));
        let default_pose_in_ground = ground_to_field.inverse() * default_pose_in_field;
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let blocking_speed = blackboard.parameters.walking.speed.blocking;
        let orientation_mode = if let Some(ball) = &blackboard.world_state.ball {
            OrientationMode::LookAt {
                target: ball.ball_in_ground,
                tolerance: walk_and_stand.orientation_tolerance,
            }
        } else if blackboard
            .ball
            .as_ref()
            .is_some_and(|ball| ball.position.x() <= 0.0)
        {
            OrientationMode::LookAt {
                target: ground_to_field.inverse() * point!(own_goal_line_x, 0.0),
                tolerance: walk_and_stand.orientation_tolerance,
            }
        } else {
            OrientationMode::AlignWithPath
        };

        walk_to(
            blackboard,
            default_pose_in_ground,
            blocking_speed,
            orientation_mode,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.goalkeeper_hysteresis,
        )
    } else {
        Status::Failure
    }
}
pub fn walk_to_goalkeeper_penalty_position(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let own_goal_line_x = -blackboard.field_dimensions.length / 2.0;
        let penalty_position_in_field = point!(own_goal_line_x, 0.0);
        let penalty_pose_in_field =
            Pose2::from_parts(penalty_position_in_field, Orientation2::new(0.0));
        let penalty_pose_in_ground = ground_to_field.inverse() * penalty_pose_in_field;
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let blocking_speed = blackboard.parameters.walking.speed.blocking;

        walk_to(
            blackboard,
            penalty_pose_in_ground,
            blocking_speed,
            OrientationMode::AlignWithPath,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.goalkeeper_hysteresis,
        )
    } else {
        Status::Failure
    }
}

fn select_goalkeeper_kick_away_target(blackboard: &mut Blackboard) -> Status {
    if let Some(ball) = &blackboard.ball {
        let target_y = if ball.position.y() >= 0.0 {
            blackboard.field_dimensions.width / 4.0
        } else {
            -blackboard.field_dimensions.width / 4.0
        };

        apply_kick_target(blackboard, point!(0.0, target_y))
    } else {
        Status::Failure
    }
}

fn ball_interception_velocity_in_field(blackboard: &Blackboard) -> Option<Vector2<Field>> {
    let ball = blackboard.ball.as_ref()?;
    Some(ball.velocity)
}

fn is_interception_velocity_towards_own_half(
    blackboard: &Blackboard,
    ball_velocity: Vector2<Field>,
) -> bool {
    let parameters = &blackboard.parameters.ball.interception;

    ball_velocity.norm() > parameters.minimum_ball_velocity && ball_velocity.x() < 0.0
}

pub(crate) fn goalkeeper_shot_block_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        sequence!(
            action!(set_goalkeeper_shot_block_position),
            action!(walk_to_goalkeeper_shot_block_position)
        ),
        Node::Failure,
    )
}

fn shot_y_at_defense_line(
    ball: Point2<Field>,
    velocity: Vector2<Field>,
    goal_x: f32,
    defense_x: f32,
    goal_half_width: f32,
) -> Option<f32> {
    if ![ball.x(), ball.y(), velocity.x(), velocity.y()]
        .into_iter()
        .all(f32::is_finite)
        || velocity.x() >= 0.0
        || ball.x() <= defense_x
    {
        return None;
    }
    let goal_y = ball.y() + velocity.y() * (goal_x - ball.x()) / velocity.x();
    if goal_y.abs() >= goal_half_width {
        return None;
    }
    Some(ball.y() + velocity.y() * (defense_x - ball.x()) / velocity.x())
}

pub(crate) fn goalkeeper_shot_block_position(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let ball = blackboard.ball.as_ref()?;
    if !is_interception_velocity_towards_own_half(blackboard, ball.velocity) {
        return None;
    }
    goalkeeper_block_position_for_direction(blackboard, ball.velocity)
}

fn goalkeeper_block_position_for_direction(
    blackboard: &Blackboard,
    direction: Vector2<Field>,
) -> Option<Point2<Field>> {
    let ball = blackboard.ball.as_ref()?;
    let dimensions = blackboard.field_dimensions;
    let parameters = &blackboard.parameters.goalkeeper;
    let goal_x = -dimensions.length / 2.0;
    let defense_x = goal_x + parameters.x_offset;
    let goal_half_width = dimensions.goal_inner_width / 2.0;
    let target_y = shot_y_at_defense_line(
        ball.position,
        direction,
        goal_x,
        defense_x,
        goal_half_width + dimensions.goal_post_diameter / 2.0 + dimensions.ball_radius,
    )?;
    let maximum_y = (goal_half_width - parameters.distance_to_goalpost).max(0.0);
    Some(point!(defense_x, target_y.clamp(-maximum_y, maximum_y)))
}

fn set_goalkeeper_shot_block_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(position), Some(ground_to_field)) = (
        goalkeeper_shot_block_position(blackboard),
        blackboard.world_state.robot.ground_to_field,
    ) {
        blackboard.walk_position = Some(ground_to_field.inverse() * position);
        Status::Success
    } else {
        Status::Failure
    }
}

fn walk_to_goalkeeper_shot_block_position(blackboard: &mut Blackboard) -> Status {
    let status = walk_to_block_position(blackboard);
    if status == Status::Failure {
        return crate::actions::stand(blackboard);
    }
    if let Some(BodyMotion::Walk { speed, .. }) = &mut blackboard.body_motion {
        *speed = speed.max(1.0);
    }
    let limits = blackboard.walking_velocity_limits;
    let (
        Some(BodyMotion::Walk {
            path,
            target_orientation,
            ..
        }),
        Some(target),
        Some(ball),
        Some(transform),
    ) = (
        &blackboard.body_motion,
        blackboard.walk_position,
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    )
    else {
        return status;
    };
    // Follow planned detours normally; only accelerate along an unobstructed segment.
    let [PathSegment::LineSegment(line)] = path.segments.as_slice() else {
        return status;
    };
    if limits.validate().is_err()
        || line.0.coords().norm() > 1e-4
        || (line.1 - target).norm() > 1e-4
        || blackboard
            .path_obstacles_output
            .iter()
            .any(|obstacle| obstacle.shape.intersects_line_segment(*line))
    {
        return status;
    }
    let time_to_line = ((transform * target).x() - ball.position.x()) / ball.velocity.x();
    if !time_to_line.is_finite() || time_to_line <= 0.0 {
        return status;
    }
    let velocity = adaptive_blocking_velocity(
        blackboard,
        target.coords() / time_to_line,
        transform.inverse() * ball.position,
    );
    let angular_velocity = (target_orientation.angle() / time_to_line).clamp(
        -limits.angular_velocity_limit,
        limits.angular_velocity_limit,
    );
    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity,
        angular_velocity,
    });
    status
}

#[cfg(test)]
mod tests {
    use super::{goalkeeper_subtree, shot_y_at_defense_line};
    use hsl_network_messages::{PlayerNumber, SubState, Team};
    use linear_algebra::{Isometry2, point, vector};
    use types::{
        field_dimensions::{FieldDimensions, Side},
        filtered_game_state::FilteredGameState,
        motion_command::BodyMotion,
        world_state::BallSource,
    };

    use crate::node::{Blackboard, LastBall};

    #[test]
    fn close_crossing_shots_block_laterally_instead_of_clearing_or_chasing() {
        for restart in [
            None,
            Some(SubState::CornerKick),
            Some(SubState::DirectFreeKick),
        ] {
            let ball = LastBall {
                position: point![-3.6, 0.0],
                velocity: vector![-2.0, 1.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            };
            let mut board = Blackboard {
                field_dimensions: FieldDimensions::SPL_2025,
                player_number: PlayerNumber::One,
                ball: Some(ball.clone()),
                last_ball: Some(ball),
                last_motion_type: Some(types::motion_type::MotionType::Walk),
                ..Default::default()
            };
            board.world_state.robot.ground_to_field =
                Some(Isometry2::from_parts(vector![-4.2, 0.0], 0.0));
            board.parameters.goalkeeper.player_number = PlayerNumber::One;
            board.parameters.goalkeeper.x_offset = 0.3;
            board.parameters.goalkeeper.distance_to_goalpost = 0.15;
            board
                .parameters
                .goalkeeper
                .kick_away_ball_maximum_robot_distance = 1.5;
            board.parameters.ball.interception.minimum_ball_velocity = 0.3;
            board.parameters.ball.interception.maximum_ball_distance = 3.5;
            board
                .parameters
                .ball
                .interception
                .maximum_intercept_distance = 1.0;
            board.parameters.kicking.kick_activation_distance = 1.3;
            board
                .parameters
                .kicking
                .minimum_interception_forward_distance = 0.3;
            board.parameters.kicking.approach_ball_standoff = 0.3;
            let mut game = crate::test_support::filtered_game_controller_state();
            game.game_state = FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            };
            game.kicking_team = Some(Team::Opponent);
            game.sub_state = restart;
            board.world_state.filtered_game_controller_state = Some(game);

            let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);

            assert_eq!(status, types::behavior_tree::Status::Success);
            approx::assert_relative_eq!(board.walk_position.unwrap().y(), 0.3, epsilon = 1e-5);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Walk { .. } | BodyMotion::WalkWithVelocity { .. })
            ));
        }
    }

    #[test]
    fn keeper_starts_walking_for_shot_from_halfway() {
        let transform = Isometry2::from_parts(vector![-4.2, 0.0], 0.0);
        let ball = LastBall {
            position: point![0.0, 1.5],
            velocity: vector![-3.0, -0.8],
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        };
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball: Some(ball.clone()),
            last_ball: Some(ball),
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(transform);
        board.parameters.goalkeeper.x_offset = 0.3;
        board.parameters.goalkeeper.passive_distance = 1.0;
        board.parameters.ball.interception.minimum_ball_velocity = 1.0;
        board
            .parameters
            .ball
            .interception
            .minimum_ball_velocity_towards_own_half = 0.7;
        let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
        assert_eq!(status, types::behavior_tree::Status::Success);
        assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
        let target = transform * board.walk_position.unwrap();
        approx::assert_relative_eq!(target.x(), -4.2);
        approx::assert_relative_eq!(target.y(), 0.38, epsilon = 1e-5);
    }

    #[test]
    fn keeper_leaves_ball_goal_line_for_fast_angled_shot() {
        for side in [-1.0, 1.0] {
            let goal_to_ball = vector![0.7_f32, side];
            let ready_position = point![-4.5, 0.0] + goal_to_ball.normalize() * 0.6;
            let transform = Isometry2::from_parts(ready_position.coords(), 0.0);
            let ball = LastBall {
                position: point![-3.8, side],
                velocity: vector![-0.5, -1.2 * side],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            };
            let mut board = Blackboard {
                field_dimensions: FieldDimensions::SPL_2025,
                ball: Some(ball.clone()),
                last_ball: Some(ball),
                ..Default::default()
            };
            board.world_state.robot.ground_to_field = Some(transform);
            board.last_motion_type = Some(types::motion_type::MotionType::Walk);
            board.parameters.goalkeeper.x_offset = 0.3;
            board.parameters.goalkeeper.distance_to_goalpost = 0.15;
            board.parameters.goalkeeper.passive_distance = 4.5;
            board
                .parameters
                .goalkeeper
                .active_defense_maximum_robot_distance = 2.0;
            board.parameters.ball.interception.minimum_ball_velocity = 1.0;
            board
                .parameters
                .ball
                .interception
                .minimum_ball_velocity_towards_own_half = 0.7;
            board.parameters.ball.interception.maximum_ball_distance = 3.5;
            board
                .parameters
                .ball
                .interception
                .maximum_intercept_distance = 1.0;
            board
                .parameters
                .kicking
                .minimum_interception_forward_distance = 0.3;
            let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
            assert_eq!(status, types::behavior_tree::Status::Success);
            let target = transform * board.walk_position.unwrap();
            approx::assert_relative_eq!(target.x(), -4.2, epsilon = 1e-5);
            approx::assert_relative_eq!(target.y(), 0.04 * side, epsilon = 1e-5);
            assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
        }
    }

    #[test]
    fn keeper_commands_enough_speed_to_reach_fast_shot() {
        let transform = Isometry2::from_parts(vector![-6.7, 0.0], 0.0);
        let ball = LastBall {
            position: point![-4.5, -0.6],
            velocity: vector![-2.8, 1.6],
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        };
        let mut board = Blackboard {
            field_dimensions: FieldDimensions {
                length: 14.0,
                goal_inner_width: 2.6,
                ..FieldDimensions::SPL_2025
            },
            ball: Some(ball.clone()),
            last_ball: Some(ball),
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(transform);
        board.last_motion_type = Some(types::motion_type::MotionType::Walk);
        board.parameters.goalkeeper.x_offset = 0.3;
        board.parameters.goalkeeper.distance_to_goalpost = 0.15;
        board.parameters.ball.interception.minimum_ball_velocity = 1.0;
        board.parameters.ball.interception.maximum_ball_distance = 3.5;
        board
            .parameters
            .ball
            .interception
            .maximum_intercept_distance = 1.0;
        board
            .parameters
            .kicking
            .minimum_interception_forward_distance = 0.3;
        board.parameters.walking.speed.blocking = 0.8;
        board.parameters.walking.speed.minimum_speed = 0.2;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.walking_velocity_limits.forward_velocity_limits = [-0.5, 1.0];
        board.parameters.voronoi.grid_resolution = 0.25;
        board.parameters.voronoi.padding = 0.5;
        board.walking_velocity_limits.lateral_velocity_limit = 1.0;
        board.walking_velocity_limits.angular_velocity_limit = 2.0;
        let (status, _) =
            super::goalkeeper_shot_block_position_subtree().tick_with_trace(&mut board);
        assert_eq!(status, types::behavior_tree::Status::Success);
        let target = board.walk_position.unwrap();
        let time_to_line = 2.2 / 2.8;
        let commanded_speed = match board.body_motion.as_ref().unwrap() {
            BodyMotion::Walk { speed, .. } => *speed,
            BodyMotion::WalkWithVelocity { velocity, .. } => velocity.norm(),
            motion => panic!("expected goalkeeper walk, got {motion:?}"),
        };
        assert!(
            commanded_speed * time_to_line >= target.coords().norm() - 1e-4,
            "commanded {commanded_speed}m/s cannot cover {}m before shot in {time_to_line}s",
            target.coords().norm()
        );
        board.ball.as_mut().unwrap().velocity = vector![-3.5, 2.4];
        board.last_ball = board.ball.clone();
        super::goalkeeper_shot_block_position_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = &board.body_motion else {
            panic!("expected deadline walk for fast angled shot");
        };
        approx::assert_relative_eq!(velocity.y(), 1.0);
        board.walking_velocity_limits.lateral_velocity_limit = 1.5;
        super::goalkeeper_shot_block_position_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = &board.body_motion else {
            panic!("expected faster deadline walk when lateral limit permits");
        };
        assert!(velocity.y() > 1.0);
        approx::assert_relative_eq!(
            velocity.y() * (2.2 / 3.5),
            board.walk_position.unwrap().y(),
            epsilon = 1e-5
        );
        board.ball.as_mut().unwrap().velocity = vector![-2.8, 1.6];
        board.last_ball = board.ball.clone();
        board.walking_velocity_limits.lateral_velocity_limit = 0.25;
        super::goalkeeper_shot_block_position_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::WalkWithVelocity {
            velocity,
            angular_velocity,
        }) = &board.body_motion
        else {
            panic!("expected policy-limited deadline walk");
        };
        approx::assert_relative_eq!(velocity.y(), 0.25);
        assert!(angular_velocity.abs() <= board.walking_velocity_limits.angular_velocity_limit);

        board.parameters.walking.path_planning.robot_radius = 0.15;
        board
            .world_state
            .obstacles
            .push(types::obstacles::Obstacle::robot(
                point![0.0, 0.35],
                0.1,
                0.1,
            ));
        super::goalkeeper_shot_block_position_subtree().tick_with_trace(&mut board);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::Walk { .. })),
            "planned detours must retain ordinary path following"
        );

        board.world_state.obstacles.clear();
        board
            .world_state
            .rule_obstacles
            .push(types::rule_obstacles::RuleObstacle::Rectangle(
                geometry::rectangle::Rectangle {
                    min: point![-6.9, 0.4],
                    max: point![-6.5, 0.9],
                },
            ));
        super::goalkeeper_shot_block_position_subtree().tick_with_trace(&mut board);
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        board.world_state.rule_obstacles.clear();
    }

    #[test]
    fn blocks_distant_shots_at_the_defense_line() {
        let target = shot_y_at_defense_line(point!(0.0, 1.5), vector!(-3.0, -0.8), -4.5, -4.0, 0.8);
        approx::assert_relative_eq!(target.unwrap(), 1.5 - 0.8 * 4.0 / 3.0);
    }

    #[test]
    fn ignores_shots_wide_away_or_already_past_defense_line() {
        for (ball, velocity) in [
            (point!(0.0, 1.5), vector!(-3.0, 0.0)),
            (point!(0.0, 0.0), vector!(3.0, 0.0)),
            (point!(0.0, 0.0), vector!(0.0, 0.0)),
            (point!(0.0, 0.0), vector!(f32::NEG_INFINITY, 0.0)),
            (point!(f32::NAN, 0.0), vector!(-3.0, 0.0)),
            (point!(-4.2, 0.0), vector!(-3.0, 0.0)),
        ] {
            assert!(shot_y_at_defense_line(ball, velocity, -4.5, -4.0, 0.8).is_none());
        }
    }
}

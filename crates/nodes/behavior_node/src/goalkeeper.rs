use coordinate_systems::Field;
use hsl_network_messages::SubState;
use linear_algebra::{Orientation2, Pose2, Vector2, point};
use types::{
    behavior_tree::Status,
    motion_command::{KickPower, OrientationMode},
    motion_type::MotionType,
};

use crate::{
    action,
    behavior_tree::Node,
    condition,
    conditions::{hulks_is_kicking_team, is_closest_to_ball},
    head::look_at_ball_subtree,
    kick::{apply_visual_kick_target, intercept, kick, kick_alternatives_subtree, use_kick_power},
    negation,
    node::Blackboard,
    selection, sequence,
    substates::{is_in_sub_state, is_sub_state},
    subtree,
    switch_motion_type::switch_motion_type,
    tree::striker_subtree,
    voronoi::calculate_voronoi_grid,
    walk::{walk_alternatives_subtree, walk_to, walk_to_block_position},
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
                        action!(use_kick_power, KickPower::Rumpelstilzchen),
                    ),
                    subtree!(kick_alternatives_subtree),
                )
            ),
            sequence!(
                condition!(is_goalkeeper_interception_candidate),
                switch_motion_type(
                    MotionType::Kick,
                    sequence!(
                        action!(kick),
                        action!(intercept),
                        action!(use_kick_power, KickPower::Rumpelstilzchen),
                    ),
                    subtree!(kick_alternatives_subtree),
                )
            ),
            sequence!(
                condition!(is_ball_close_enough_to_goal_to_become_striker),
                selection!(sequence!(
                    action!(calculate_voronoi_grid),
                    condition!(is_closest_to_ball),
                    subtree!(striker_subtree),
                ),),
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
    if !is_ball_near_own_goal(blackboard) {
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
        let interception_line_x = own_goal_x + blackboard.parameters.keeper.x_offset;
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
        let velocity = ball.velocity;
        let time_to_closest_approach =
            -ball_in_ground.coords().dot(&velocity) / velocity.norm_squared();
        if time_to_closest_approach < 0.0 {
            return false;
        }

        let interception_point = ball_in_ground + velocity * time_to_closest_approach;
        interception_point.x() >= blackboard.parameters.kicking.kick_position_ball_distance
            && interception_point.coords().norm()
                <= blackboard
                    .parameters
                    .intercept_ball
                    .maximum_intercept_distance
    } else {
        false
    }
}

fn is_goalkeeper_kick_away_needed(blackboard: &mut Blackboard) -> bool {
    if !is_ball_in_own_penalty_area(blackboard) {
        return false;
    }

    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let parameters = &blackboard.parameters.keeper;
        let ball_in_ground = ground_to_field.inverse() * ball.position;

        ball_in_ground.x() > 0.0
            && ball_in_ground.coords().norm() < parameters.kick_away_ball_maximum_robot_distance
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

fn is_ball_close_enough_to_goal_to_become_striker(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let own_goal_x = blackboard.field_dimensions.length / 2.0;
        let maximum_ball_x = own_goal_x + blackboard.parameters.keeper.striker_distance;

        ball.position.x() < maximum_ball_x
    })
}

fn is_ball_near_own_goal(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let own_goal_x = blackboard.field_dimensions.length / 2.0;
        let maximum_ball_x = own_goal_x + blackboard.parameters.keeper.passive_distance;

        ball.position.x() < maximum_ball_x
    })
}
pub fn set_goalkeeper_active_defense_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let field_dimensions = blackboard.field_dimensions;
        let parameters = &blackboard.parameters.keeper;

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
        let parameters = &blackboard.parameters.keeper;
        let own_goal_line_x = -field_dimensions.length / 2.0;
        let default_position_in_field = point!(own_goal_line_x + parameters.x_offset, 0.0);
        let default_pose_in_field =
            Pose2::from_parts(default_position_in_field, Orientation2::new(0.0));
        let default_pose_in_ground = ground_to_field.inverse() * default_pose_in_field;
        let orientation_mode = if let Some(ball) = &blackboard.world_state.ball {
            OrientationMode::LookAt {
                target: ball.ball_in_ground,
                tolerance: blackboard.parameters.walk_and_stand.orientation_tolerance,
            }
        } else if blackboard
            .ball
            .as_ref()
            .is_some_and(|ball| ball.position.x() <= 0.0)
        {
            OrientationMode::LookAt {
                target: ground_to_field.inverse() * point!(own_goal_line_x, 0.0),
                tolerance: blackboard.parameters.walk_and_stand.orientation_tolerance,
            }
        } else {
            OrientationMode::AlignWithPath
        };

        walk_to(
            blackboard,
            default_pose_in_ground,
            blackboard.parameters.walk_speed.blocking,
            orientation_mode,
            blackboard
                .parameters
                .walk_and_stand
                .normal_distance_to_be_aligned,
            blackboard.parameters.walk_and_stand.goalkeeper_hysteresis,
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

        walk_to(
            blackboard,
            penalty_pose_in_ground,
            blackboard.parameters.walk_speed.blocking,
            OrientationMode::AlignWithPath,
            blackboard
                .parameters
                .walk_and_stand
                .normal_distance_to_be_aligned,
            blackboard.parameters.walk_and_stand.hysteresis,
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

        apply_visual_kick_target(blackboard, point!(0.0, target_y), 0.0)
    } else {
        Status::Failure
    }
}

fn ball_interception_velocity_in_field(blackboard: &Blackboard) -> Option<Vector2<Field>> {
    let ball = blackboard.ball.as_ref()?;
    let ground_to_field = blackboard.world_state.robot.ground_to_field?;

    Some(ground_to_field * ball.velocity)
}

fn is_interception_velocity_towards_own_half(
    blackboard: &Blackboard,
    ball_velocity: Vector2<Field>,
) -> bool {
    let parameters = &blackboard.parameters.intercept_ball;

    ball_velocity.norm() > parameters.minimum_ball_velocity
        && ball_velocity.x() < -parameters.minimum_ball_velocity_towards_own_half
}

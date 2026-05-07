use hsl_network_messages::{SubState, Team};
use linear_algebra::{Orientation2, Pose2, Rotation2, point};
use types::{
    behavior_tree::Status, field_dimensions::Side,
    filtered_game_controller_state::FilteredGameControllerState, motion_command::OrientationMode,
};

use crate::{
    action,
    behavior::{
        behavior_tree::Node,
        condition::{has_ball_position, is_close_to_ball_aligned, is_sub_state},
        kick::kick_subtree,
        node::Blackboard,
        tree::search_subtree,
        walk::{walk_to, walk_to_ball_subtree},
    },
    condition, negation, selection, sequence, subtree,
};

pub fn sub_state_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            negation!(condition!(has_ball_position)),
            subtree!(search_subtree)
        ),
        sequence!(
            condition!(hulks_is_kicking_team),
            selection!(
                sequence!(
                    negation!(condition!(is_close_to_ball_aligned)),
                    subtree!(walk_to_ball_subtree)
                ),
                subtree!(kick_subtree)
            )
        ),
        sequence!(
            selection!(
                sequence!(
                    selection!(
                        condition!(is_sub_state, SubState::ThrowIn),
                        condition!(is_sub_state, SubState::IndirectFreeKick),
                        condition!(is_sub_state, SubState::DirectFreeKick),
                        condition!(is_sub_state, SubState::GoalKick),
                    ),
                    action!(set_block_position_field),
                ),
                sequence!(
                    condition!(is_sub_state, SubState::CornerKick),
                    action!(set_block_position_corner),
                ),
                sequence!(
                    condition!(is_sub_state, SubState::PenaltyKick),
                    action!(set_block_position_penalty_kick),
                )
            ),
            action!(walk_to_block_position)
        )
    )
}

pub fn hulks_is_kicking_team(blackboard: &mut Blackboard) -> bool {
    matches!(
        blackboard.world_state.filtered_game_controller_state,
        Some(FilteredGameControllerState {
            kicking_team: Some(Team::Hulks),
            ..
        })
    )
}

pub fn walk_to_block_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(block_position), Some(ball), Some(ground_to_field)) = (
        &blackboard.walk_position,
        &blackboard.last_ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_position = ground_to_field.inverse() * ball.position;
        let orientation = Orientation2::from_vector(ball_position - *block_position);

        walk_to(
            blackboard,
            Pose2::from_parts(*block_position, orientation),
            blackboard.parameters.walk_speed.blocking,
            OrientationMode::LookAt {
                target: ball_position,
                tolerance: blackboard.parameters.walk_and_stand.orientation_tolerance,
            },
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

pub fn set_block_position_field(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let goal_position =
            ground_to_field.inverse() * point!(-blackboard.field_dimensions.length / 2.0, 0.0);
        let direction = (goal_position - ball_in_ground).normalize();

        let distance_to_ball = (blackboard.field_dimensions.center_circle_diameter / 2.0
            + blackboard.parameters.substates.blocking_distance_offset)
            .max(
                blackboard.free_kick_obstacle_radius
                    + blackboard.parameters.path_planning.robot_radius,
            );

        blackboard.walk_position = Some(ball_in_ground + (direction * distance_to_ball));

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn set_block_position_corner(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_position = ground_to_field.inverse() * ball.position;
        let goal_position =
            ground_to_field.inverse() * point!(-blackboard.field_dimensions.length / 2.0, 0.0);
        let angle_direction = match ball.field_side {
            Side::Left => 1.0,
            Side::Right => -1.0,
        };

        let parameters = &blackboard.parameters.substates;

        let direction = Rotation2::new(angle_direction * parameters.corner_kick_blocking_angle)
            * (goal_position - ball_position).normalize();

        let distance_to_ball = (blackboard.field_dimensions.center_circle_diameter / 2.0
            + blackboard.parameters.substates.blocking_distance_offset)
            .max(
                blackboard.free_kick_obstacle_radius
                    + blackboard.parameters.path_planning.robot_radius,
            );

        blackboard.walk_position = Some(ball_position + direction * distance_to_ball);

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn set_block_position_penalty_kick(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = &blackboard.world_state.robot.ground_to_field {
        let field_dimensions = blackboard.field_dimensions;
        let penalty_area_x = -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length;
        let distance_to_ball_along_x =
            field_dimensions.penalty_area_length - field_dimensions.penalty_marker_distance;
        let distance_to_ball = (blackboard.field_dimensions.center_circle_diameter / 2.0
            + blackboard.parameters.substates.blocking_distance_offset)
            .max(blackboard.free_kick_obstacle_radius);

        let line_position = (distance_to_ball.powi(2) - distance_to_ball_along_x.powi(2))
            .max(0.0)
            .sqrt();

        let side_sign = (ground_to_field * point!(0.0, 0.0)).y().signum();

        blackboard.walk_position = Some(point!(penalty_area_x, side_sign * line_position));

        Status::Success
    } else {
        Status::Failure
    }
}

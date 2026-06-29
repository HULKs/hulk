use coordinate_systems::Field;
use hsl_network_messages::SubState;
use linear_algebra::{Vector2, point};
use types::{behavior_tree::Status, motion_command::KickPower, motion_type::MotionType};

use crate::{
    action,
    behavior::{
        behavior_tree::Node,
        condition::{hulks_is_kicking_team, is_closest_to_ball},
        head::look_at_ball_subtree,
        kick::{
            apply_visual_kick_target, intercept, kick, kick_alternatives_subtree, use_kick_power,
        },
        node::Blackboard,
        substates::{is_in_sub_state, is_sub_state},
        switch_motion_type::switch_motion_type,
        walk::{
            set_goalkeeper_active_defense_position, walk_alternatives_subtree,
            walk_to_block_position, walk_to_goalkeeper_default_position,
            walk_to_goalkeeper_penalty_position,
        },
        striker::striker_subtree,
    },
    condition, negation, selection, sequence, subtree,
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
                condition!(is_ball_near_own_goal),
                subtree!(goalkeeper_active_defense_position_subtree)
            ),
            sequence!(
                condition!(is_closest_to_ball),
                selection!(
                    sequence!(
                        condition!(), // closest to ball and ball is close to goal
                        subtree!(striker_subtree)
                    ),
                    sequence!(
                        condition!(), // closest to ball but ball too far from goal
                        action!() // give striker role to someone else
                    )
                    )
                )
            subtree!(goalkeeper_default_position_subtree),
        )
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
            ),
            subtree!(goalkeeper_active_defense_position_subtree)
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

        y_at_interception_line.abs() < goal_half_width
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

        ball.velocity.norm() < parameters.kick_away_ball_maximum_velocity
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

fn is_ball_near_own_goal(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let own_goal_x = -blackboard.field_dimensions.length / 2.0;
        let maximum_ball_x = own_goal_x + blackboard.parameters.keeper.passive_distance;

        ball.position.x() < maximum_ball_x
    })
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

fn goalkeeper_default_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(walk_to_goalkeeper_default_position),
        subtree!(walk_alternatives_subtree),
    )
}

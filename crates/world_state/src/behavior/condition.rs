use filtering::hysteresis::less_than_with_hysteresis;
use hsl_network_messages::Team;
use linear_algebra::{Vector2, point, vector};
use types::{
    filtered_game_controller_state::FilteredGameControllerState, motion_type::MotionType,
    primary_state::PrimaryState,
};
use voronoi::Ownership;

use coordinate_systems::Field;

use crate::behavior::node::Blackboard;

pub fn is_ball_interception_candidate(blackboard: &mut Blackboard) -> bool {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let parameters = &blackboard.parameters.intercept_ball;

        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let ball_is_in_front_of_robot = ball_in_ground.coords().norm()
            < parameters.maximum_ball_distance
            && ball_in_ground.x() > 0.0;
        let ball_is_moving_towards_robot =
            ball.velocity.x() < -parameters.minimum_ball_velocity_towards_robot;

        let Some(ball_in_field_velocity) = ball_interception_velocity_in_field(blackboard) else {
            return false;
        };

        ball_is_in_front_of_robot
            && ball_is_moving_towards_robot
            && is_interception_velocity_towards_own_half(blackboard, ball_in_field_velocity)
    } else {
        false
    }
}

pub fn is_goalkeeper_interception_candidate(blackboard: &mut Blackboard) -> bool {
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
        let interception_line_x = own_goal_x + blackboard.parameters.role_positions.keeper_x_offset;
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

pub fn is_ball_near_own_goal(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let own_goal_x = -blackboard.field_dimensions.length / 2.0;
        let maximum_ball_x = own_goal_x
            + blackboard
                .parameters
                .role_positions
                .keeper_ball_near_own_goal_distance;

        ball.position.x() < maximum_ball_x
    })
}

pub fn is_ball_in_own_danger_area(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let field_dimensions = blackboard.field_dimensions;
        let own_penalty_area_x =
            -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length;

        ball.position.x() < own_penalty_area_x
            && ball.position.y().abs() < field_dimensions.penalty_area_width / 2.0
    })
}

pub fn is_ball_suspiciously_behind_own_goal(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let field_dimensions = blackboard.field_dimensions;
        let own_goal_x = -field_dimensions.length / 2.0;

        ball.position.x() < own_goal_x - field_dimensions.goal_depth
    })
}

pub fn is_goalkeeper_clear_candidate(blackboard: &mut Blackboard) -> bool {
    if !is_ball_in_own_danger_area(blackboard) && !is_ball_suspiciously_behind_own_goal(blackboard)
    {
        return false;
    }

    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let parameters = &blackboard.parameters.role_positions;
        let ball_in_ground = ground_to_field.inverse() * ball.position;

        ball.velocity.norm() < parameters.keeper_clear_ball_maximum_velocity
            && ball_in_ground.coords().norm() < parameters.keeper_clear_ball_maximum_robot_distance
    } else {
        false
    }
}

pub fn is_goalkeeper_visual_kick_hold_active(blackboard: &mut Blackboard) -> bool {
    if !matches!(blackboard.last_motion_type, Some(MotionType::Kick))
        || !is_ball_near_own_goal(blackboard)
    {
        return false;
    }

    let time_since_last_motion_switch = blackboard
        .world_state
        .now
        .duration_since(blackboard.last_motion_switch_time)
        .unwrap_or_default();

    time_since_last_motion_switch
        < blackboard
            .parameters
            .role_positions
            .keeper_visual_kick_hold_duration
}

pub fn is_close_to_ball(blackboard: &mut Blackboard) -> bool {
    let mut is_close = false;
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let distance_to_ball = (ground_to_field.inverse() * ball.position).coords().norm();
        let parameters = &blackboard.parameters.kicking;
        is_close = less_than_with_hysteresis(
            blackboard.last_close_enough_to_kick,
            distance_to_ball,
            parameters.distance_for_kick,
            parameters.distance_for_kick_hysteresis,
        );
        blackboard.last_close_enough_to_kick = is_close;
    }

    is_close
}

pub fn is_close_to_ball_aligned(blackboard: &mut Blackboard) -> bool {
    let mut is_close_and_aligned = false;
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let goal_position =
            ground_to_field.inverse() * point!(blackboard.field_dimensions.length / 2.0, 0.0);
        let target_kick_position = ball_in_ground
            - (goal_position - ball_in_ground).normalize()
                * blackboard.parameters.kicking.kick_position_ball_distance;

        let parameters = &blackboard.parameters.substates;
        let distance_to_ball = target_kick_position.coords().norm();
        let is_close = less_than_with_hysteresis(
            blackboard.last_close_enough_to_kick,
            distance_to_ball,
            parameters.distance_for_kick,
            parameters.distance_for_kick_hysteresis,
        );

        let direction = (goal_position - target_kick_position).normalize();
        let robot_facing_direction = vector!(1.0, 0.0);
        let is_aligned =
            direction.angle(&robot_facing_direction) < parameters.alignment_angle_threshold;

        blackboard.direction_difference = direction.angle(&robot_facing_direction);

        is_close_and_aligned = is_close && (is_aligned || blackboard.last_close_enough_to_kick);
        blackboard.last_close_enough_to_kick = is_close_and_aligned;
    }

    is_close_and_aligned
}

pub fn is_closest_to_ball(blackboard: &mut Blackboard) -> bool {
    let own_player_number = blackboard.world_state.robot.player_number;

    let raw_is_closest =
        if let (Some(ball), Some(voronoi_map)) = (&blackboard.ball, &blackboard.voronoi_map) {
            let ownership_at_ball = voronoi_map.ownership_at(ball.position);
            match ownership_at_ball {
                Some(Ownership::Robot(player_number)) if player_number == own_player_number => true,
                Some(Ownership::Blocked) => voronoi_map
                    .nearest_non_blocked_ownership(ball.position)
                    .is_some_and(|ownership| ownership == Ownership::Robot(own_player_number)),
                _ => false,
            }
        } else {
            false
        };

    let now = blackboard.world_state.now;
    if raw_is_closest {
        blackboard.closest_to_ball_entered_area_since = blackboard
            .closest_to_ball_entered_area_since
            .or(Some(now.to_wallclock()));
        blackboard.closest_to_ball_left_area_since = None;
    } else {
        blackboard.closest_to_ball_left_area_since = blackboard
            .closest_to_ball_left_area_since
            .or(Some(now.to_wallclock()));
        blackboard.closest_to_ball_entered_area_since = None;
    }

    let is_closest = if blackboard.last_closest_to_ball {
        raw_is_closest
            || blackboard
                .closest_to_ball_left_area_since
                .is_some_and(|since| {
                    now.to_wallclock()
                        .duration_since(since)
                        .expect("time ran backwards")
                        < blackboard.parameters.closest_to_ball_exit_duration
                })
    } else {
        raw_is_closest
            && blackboard
                .closest_to_ball_entered_area_since
                .is_some_and(|since| {
                    now.to_wallclock()
                        .duration_since(since)
                        .expect("time ran backwards")
                        >= blackboard.parameters.closest_to_ball_enter_duration
                })
    };

    blackboard.last_closest_to_ball = is_closest;
    is_closest
}

pub fn is_fallen(blackboard: &mut Blackboard) -> bool {
    blackboard
        .world_state
        .fall_down_state
        .is_some_and(|fall_down_state| fall_down_state.is_recovery_available)
}

pub fn is_goalkeeper(blackboard: &mut Blackboard) -> bool {
    blackboard.world_state.robot.player_number == blackboard.parameters.goal_keeper_number
}

pub fn is_last_man_standing(blackboard: &mut Blackboard) -> bool {
    blackboard.parameters.last_man_standing
        && blackboard
            .world_state
            .player_states
            .iter()
            .filter(|(player_number, _)| {
                *player_number != blackboard.world_state.robot.player_number
            })
            .all(|(_, player_state)| player_state.is_none())
}

pub fn is_primary_state(blackboard: &mut Blackboard, primary_state: PrimaryState) -> bool {
    blackboard.world_state.robot.primary_state == primary_state
}

pub fn is_remote_controlled(blackboard: &mut Blackboard) -> bool {
    blackboard.parameters.remote_control.enable
}

pub fn is_remote_kick_mode(blackboard: &mut Blackboard) -> bool {
    blackboard.parameters.remote_control.kick_mode_toggle
}

pub fn has_ball_position(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.is_some()
}

pub fn has_new_ball_position(blackboard: &mut Blackboard) -> bool {
    blackboard.world_state.ball.is_some()
}

pub fn has_hypothetical_ball_position(blackboard: &mut Blackboard) -> bool {
    !blackboard
        .world_state
        .hypothetical_ball_positions
        .is_empty()
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

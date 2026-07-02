use filtering::hysteresis::less_than_with_hysteresis;
use hsl_network_messages::Team;
use linear_algebra::{point, vector};
use types::{
    filtered_game_controller_state::FilteredGameControllerState, primary_state::PrimaryState,
};
use voronoi::Ownership;

use crate::node::Blackboard;

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

        let ball_in_field_velocity = ground_to_field * ball.velocity;
        let ball_is_moving = ball_in_field_velocity.norm() > parameters.minimum_ball_velocity;
        let ball_is_moving_towards_own_half =
            ball_in_field_velocity.x() < -parameters.minimum_ball_velocity_towards_own_half;

        ball_is_in_front_of_robot
            && ball_is_moving
            && ball_is_moving_towards_robot
            && ball_is_moving_towards_own_half
    } else {
        false
    }
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
        blackboard.closest_to_ball_entered_area_since =
            blackboard.closest_to_ball_entered_area_since.or(Some(now));
        blackboard.closest_to_ball_left_area_since = None;
    } else {
        blackboard.closest_to_ball_left_area_since =
            blackboard.closest_to_ball_left_area_since.or(Some(now));
        blackboard.closest_to_ball_entered_area_since = None;
    }

    let is_closest = if blackboard.last_closest_to_ball {
        raw_is_closest
            || blackboard
                .closest_to_ball_left_area_since
                .is_some_and(|since| {
                    now.duration_since(since) < blackboard.parameters.closest_to_ball_exit_duration
                })
    } else {
        raw_is_closest
            && blackboard
                .closest_to_ball_entered_area_since
                .is_some_and(|since| {
                    now.duration_since(since)
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

pub fn is_alone(blackboard: &mut Blackboard) -> bool {
    let own_player_number = blackboard.world_state.robot.player_number;

    blackboard
        .world_state
        .player_states
        .iter()
        .all(|(player_number, player_state)| {
            player_number == own_player_number || player_state.is_none()
        })
}

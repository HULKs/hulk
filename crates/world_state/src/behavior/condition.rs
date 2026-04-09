use coordinate_systems::Field;
use linear_algebra::{Vector2, vector};
use types::primary_state::PrimaryState;

use crate::behavior::node::Blackboard;

pub fn is_close_to_ball(blackboard: &mut Blackboard) -> bool {
    if let Some(ball) = &blackboard.world_state.ball {
        ball.ball_in_ground.coords().norm()
            < blackboard.parameters.kicking.distance_for_kick_hysteresis
    } else {
        false
    }
}

pub fn is_close_to_goal(blackboard: &mut Blackboard) -> bool {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let field_to_ground = ground_to_field.inverse();

        let goal_position: Vector2<Field> = vector!(blackboard.field_dimensions.length / 2.0, 0.0);

        let target_position = (field_to_ground * goal_position).as_point();

        target_position.coords().norm()
            < blackboard
                .parameters
                .kicking
                .goal_distance_kick_power_threshold
    } else {
        false
    }
}

pub fn is_closest_to_ball(_blackboard: &mut Blackboard) -> bool {
    // TODO
    true
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

pub fn has_ball_position(blackboard: &mut Blackboard) -> bool {
    blackboard.world_state.ball.is_some()
}

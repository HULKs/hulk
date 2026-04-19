use types::primary_state::PrimaryState;

use crate::behavior::node::Blackboard;

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

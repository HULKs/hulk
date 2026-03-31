use types::primary_state::PrimaryState;

use crate::behavior::node::CaptainBlackboard;

pub fn is_fallen(context: &mut CaptainBlackboard) -> bool {
    context
        .world_state
        .fall_down_state
        .is_some_and(|fall_down_state| fall_down_state.is_recovery_available)
}

pub fn is_primary_state(context: &mut CaptainBlackboard, primary_state: PrimaryState) -> bool {
    context.world_state.robot.primary_state == primary_state
}

pub fn has_ball_position(context: &mut CaptainBlackboard) -> bool {
    context.world_state.ball.is_some()
}

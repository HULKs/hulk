use std::time::Duration;

use types::motion_type::MotionType;

use crate::behavior::node::Blackboard;

pub fn is_allowed_to_switch(blackboard: &mut Blackboard) -> bool {
    let parameters = &blackboard.parameters.allow_switch;
    let time_since_last_switch = blackboard
        .world_state
        .now
        .duration_since(blackboard.last_motion_switch_time)
        .unwrap_or(Duration::ZERO);

    blackboard.time_since_last_switch = time_since_last_switch;
    match blackboard.last_motion_type {
        Some(MotionType::Kick) => parameters.kick < time_since_last_switch,
        Some(MotionType::Prepare) => parameters.prepare < time_since_last_switch,
        Some(MotionType::Stand) => parameters.stand < time_since_last_switch,
        Some(MotionType::StandUp) => parameters.stand_up < time_since_last_switch,
        Some(MotionType::Walk) => parameters.walk < time_since_last_switch,
        None => true,
    }
}

pub fn is_last_motion_type(blackboard: &mut Blackboard, motion_type: MotionType) -> bool {
    matches!(&blackboard.last_motion_type, Some(last_motion_type) if *last_motion_type == motion_type)
}

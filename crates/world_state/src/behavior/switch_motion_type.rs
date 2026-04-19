use std::time::Duration;

use types::motion_type::MotionType;

use crate::{
    behavior::{behavior_tree::Node, node::Blackboard},
    condition, selection, sequence,
};

pub fn switch_motion_type(
    motion_type: MotionType,
    action: Node<Blackboard>,
    alternatives: Node<Blackboard>,
) -> Node<Blackboard> {
    let is_last_motion_type = match motion_type {
        MotionType::Kick => condition!(is_last_motion_type, MotionType::Kick),
        MotionType::Prepare => condition!(is_last_motion_type, MotionType::Prepare),
        MotionType::Stand => condition!(is_last_motion_type, MotionType::Stand),
        MotionType::StandUp => condition!(is_last_motion_type, MotionType::StandUp),
        MotionType::Walk => condition!(is_last_motion_type, MotionType::Walk),
    };

    selection!(
        sequence!(
            selection!(is_last_motion_type, condition!(is_allowed_to_switch)),
            action
        ),
        alternatives
    )
}

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

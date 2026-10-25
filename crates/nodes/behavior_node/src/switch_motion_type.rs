use types::motion_type::MotionType;

use crate::{behavior_tree::Node, condition, node::Blackboard, selection, sequence};

pub fn switch_motion_type(
    motion_type: MotionType,
    action: Node<Blackboard>,
    alternatives: Node<Blackboard>,
) -> Node<Blackboard> {
    let is_last_motion_type = match motion_type {
        MotionType::Kick => condition!(is_last_motion_type, MotionType::Kick),
        MotionType::Prepare => condition!(is_last_motion_type, MotionType::Prepare),
        MotionType::Damping => condition!(is_last_motion_type, MotionType::Damping),
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

pub fn is_allowed_to_start_kick(blackboard: &mut Blackboard) -> bool {
    // A selected kick can take over walking immediately; recovery motions keep their lock.
    blackboard.last_motion_type == Some(MotionType::Walk) || is_allowed_to_switch(blackboard)
}

pub fn is_allowed_to_switch(blackboard: &mut Blackboard) -> bool {
    let parameters = &blackboard.parameters.control.allow_switch;
    let time_since_last_switch = blackboard
        .world_state
        .now
        .duration_since(blackboard.last_motion_switch_time);

    blackboard.time_since_last_switch = time_since_last_switch;
    match blackboard.last_motion_type {
        Some(MotionType::Damping) => parameters.damping < time_since_last_switch,
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn kicking_can_take_over_walk_without_bypassing_other_motion_locks() {
        let mut board = Blackboard::default();
        board.world_state.now = ros_z::time::Time::from_nanos(1_000_000_000);
        board.last_motion_switch_time = board.world_state.now - Duration::from_millis(100);
        let lock = Duration::from_millis(500);
        board.parameters.control.allow_switch.walk = lock;
        board.parameters.control.allow_switch.damping = lock;
        board.parameters.control.allow_switch.kick = lock;
        board.parameters.control.allow_switch.prepare = lock;
        board.parameters.control.allow_switch.stand = lock;
        board.parameters.control.allow_switch.stand_up = lock;
        board.last_motion_type = Some(MotionType::Walk);
        assert!(!is_allowed_to_switch(&mut board));
        assert!(is_allowed_to_start_kick(&mut board));

        for motion in [
            MotionType::Damping,
            MotionType::Kick,
            MotionType::Prepare,
            MotionType::Stand,
            MotionType::StandUp,
        ] {
            board.last_motion_type = Some(motion);
            assert!(!is_allowed_to_start_kick(&mut board), "{motion:?}");
        }
        board.world_state.now = board.world_state.now + lock;
        assert!(is_allowed_to_start_kick(&mut board));
    }
}

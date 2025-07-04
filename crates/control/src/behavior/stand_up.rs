use types::{
    fall_state::{FallState, FallenDirection, StandUpSpeed},
    motion_command::MotionCommand,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState, maximum_standup_attempts: u32) -> Option<MotionCommand> {
    if world_state.robot.stand_up_count > maximum_standup_attempts {
        return Some(MotionCommand::Unstiff);
    }
    let kind = match world_state.robot.fall_state {
        FallState::Fallen { kind } => kind,
        FallState::StandingUp { kind, .. } => kind,
        _ => return None,
    };
    let speed = match (kind, world_state.robot.stand_up_count) {
        (_, 0) => StandUpSpeed::Default,
        (FallenDirection::Sitting, 1) => StandUpSpeed::Default,
        (FallenDirection::Sitting, _) => StandUpSpeed::Slow,
        (FallenDirection::FacingDown, _) => StandUpSpeed::Slow,
        (FallenDirection::FacingUp, _) => StandUpSpeed::Default,
    };
    Some(MotionCommand::StandUp { kind, speed })
}

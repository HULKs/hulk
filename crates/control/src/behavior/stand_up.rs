use types::{
    fall_state::{FallState, FallenKind, StandUpSpeed},
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
        (FallenKind::Sitting, 0) => StandUpSpeed::Default,
        (_, 1) => StandUpSpeed::Default,
        (FallenKind::Sitting, _) => StandUpSpeed::Slow,
        (FallenKind::FacingDown, _) => StandUpSpeed::Slow,
        (FallenKind::FacingUp, _) => StandUpSpeed::Default,
    };
    Some(MotionCommand::StandUp { kind, speed })
}

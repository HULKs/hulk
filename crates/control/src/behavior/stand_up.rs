use types::{
    fall_state::{FallState, Kind, StandUpSpeed},
    motion_command::MotionCommand,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    let kind = match world_state.robot.fall_state {
        FallState::Fallen { kind } => kind,
        FallState::StandingUp { kind, .. } => kind,
        _ => return None,
    };
    let speed = match (kind, world_state.robot.stand_up_count) {
        (_, 0) => StandUpSpeed::Default,
        (Kind::Sitting, 1) => StandUpSpeed::Default,
        (Kind::Sitting, _) => StandUpSpeed::Slow,
        (Kind::FacingDown, _) => StandUpSpeed::Slow,
        (Kind::FacingUp, _) => StandUpSpeed::Default,
    };
    Some(MotionCommand::StandUp { kind, speed })
}

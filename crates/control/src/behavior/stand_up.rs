use types::{
    fall_state::{FallState, Kind, StandUpSpeed},
    motion_command::MotionCommand,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        world_state.robot.fall_state,
        world_state.robot.stand_up_count,
    ) {
        (FallState::Fallen { kind }, 0) => Some(MotionCommand::StandUp {
            kind,
            speed: StandUpSpeed::Fast,
        }),
        (FallState::StandingUp { kind, .. }, 0) => Some(MotionCommand::StandUp {
            kind,
            speed: StandUpSpeed::Fast,
        }),

        (
            FallState::Fallen {
                kind: Kind::Sitting,
            },
            1,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::Sitting,
            speed: StandUpSpeed::Fast,
        }),
        (
            FallState::StandingUp {
                kind: Kind::Sitting,
                ..
            },
            1,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::Sitting,
            speed: StandUpSpeed::Fast,
        }),

        (
            FallState::Fallen {
                kind: Kind::FacingDown,
            },
            1..,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::FacingDown,
            speed: StandUpSpeed::Slow,
        }),
        (
            FallState::StandingUp {
                kind: Kind::FacingDown,
                ..
            },
            1..,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::FacingDown,
            speed: StandUpSpeed::Slow,
        }),

        (
            FallState::Fallen {
                kind: Kind::FacingUp,
            },
            1..,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::FacingUp,
            speed: StandUpSpeed::Fast,
        }),
        (
            FallState::StandingUp {
                kind: Kind::FacingUp,
                ..
            },
            1..,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::FacingUp,
            speed: StandUpSpeed::Fast,
        }),

        (
            FallState::Fallen {
                kind: Kind::Sitting,
            },
            2..,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::Sitting,
            speed: StandUpSpeed::Slow,
        }),
        (
            FallState::StandingUp {
                kind: Kind::Sitting,
                ..
            },
            2..,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::Sitting,
            speed: StandUpSpeed::Slow,
        }),
        _ => None,
    }
}

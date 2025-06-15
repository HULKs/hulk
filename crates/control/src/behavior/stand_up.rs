use types::{
    fall_state::{FallState, Kind},
    motion_command::MotionCommand,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        world_state.robot.fall_state,
        world_state.robot.stand_up_count,
    ) {
        (FallState::Fallen { kind }, 0) => Some(MotionCommand::StandUp { kind }),
        (FallState::StandingUp { kind, .. }, 0) => Some(MotionCommand::StandUp { kind }),

        (
            FallState::Fallen {
                kind: Kind::Sitting,
            },
            1,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::Sitting,
        }),
        (
            FallState::StandingUp {
                kind: Kind::Sitting,
                ..
            },
            1,
        ) => Some(MotionCommand::StandUp {
            kind: Kind::Sitting,
        }),

        (
            FallState::Fallen {
                kind: Kind::FacingDown,
            },
            1..,
        ) => Some(MotionCommand::Penalized),
        (
            FallState::StandingUp {
                kind: Kind::FacingDown,
                ..
            },
            1..,
        ) => Some(MotionCommand::Penalized),

        (
            FallState::Fallen {
                kind: Kind::FacingUp,
            },
            1..,
        ) => Some(MotionCommand::Penalized),
        (
            FallState::StandingUp {
                kind: Kind::FacingUp,
                ..
            },
            1..,
        ) => Some(MotionCommand::Penalized),

        (
            FallState::Fallen {
                kind: Kind::Sitting,
            },
            2..,
        ) => Some(MotionCommand::Penalized),
        (
            FallState::StandingUp {
                kind: Kind::Sitting,
                ..
            },
            2..,
        ) => Some(MotionCommand::Penalized),
        _ => None,
    }
}

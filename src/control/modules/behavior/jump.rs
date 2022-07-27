use types::{JumpDirection, MotionCommand, PenaltyShotDirection, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    world_state
        .ball
        .and_then(|ball| match ball.penalty_shot_direction {
            Some(PenaltyShotDirection::Left) => Some(MotionCommand::Jump {
                direction: JumpDirection::Left,
            }),
            Some(PenaltyShotDirection::Right) => Some(MotionCommand::Jump {
                direction: JumpDirection::Right,
            }),
            Some(PenaltyShotDirection::NotMoving) | None => None,
        })
}

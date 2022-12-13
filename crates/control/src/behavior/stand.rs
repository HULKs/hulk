use nalgebra::Point2;
use types::{HeadMotion, MotionCommand, PrimaryState, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        }),
        PrimaryState::Set => {
            let robot_to_field = world_state.robot.robot_to_field?;
            Some(MotionCommand::Stand {
                head: HeadMotion::LookAt {
                    target: robot_to_field.inverse() * Point2::origin(),
                },
            })
        }
        _ => None,
    }
}

use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::{CalibrationCommand, WorldState},
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        world_state.robot.primary_state,
        &world_state.calibration_command,
    ) {
        (PrimaryState::Calibration, Some(calibration_command)) => {
            let head_motion = match *calibration_command {
                CalibrationCommand::LOOKAT { target, camera, .. } => HeadMotion::LookAt {
                    target,
                    camera,
                    image_region_target: ImageRegion::Bottom,
                },
                _ => HeadMotion::Center,
            };

            Some(MotionCommand::Stand { head: head_motion })
        }
        (_, _) => None,
    }
}

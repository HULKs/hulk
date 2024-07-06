use types::{
    calibration::CalibrationCommand,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    if PrimaryState::Calibration != world_state.robot.primary_state {
        return None;
    }

    let head = if let Some(CalibrationCommand::LookAt { target, camera, .. }) = world_state.calibration_command {
        HeadMotion::Center
    } else {
        HeadMotion::LookAt {
            target,
            camera: Some(camera),
            image_region_target: ImageRegion::Bottom,
        }
    };
    Some(MotionCommand::Stand {
        head
    })
}

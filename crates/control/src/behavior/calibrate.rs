use types::{
    calibration::CalibrationCommand,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    let (PrimaryState::Calibration, Some(CalibrationCommand::LookAt { target, camera, .. })) = (
        world_state.robot.primary_state,
        world_state.calibration_command.clone(),
    ) else {
        return None;
    };
    Some(MotionCommand::Stand {
        head: HeadMotion::LookAt {
            target,
            camera: Some(camera),
            image_region_target: ImageRegion::Bottom,
        },
    })
}

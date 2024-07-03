use types::{
    calibration::CalibrationCommand,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Calibration => {
            let head_motion = match world_state.calibration_command {
                Some(CalibrationCommand::LookAt { target, camera, .. }) => HeadMotion::LookAt {
                    target,
                    camera: Some(camera),
                    image_region_target: ImageRegion::Bottom,
                },
                // TODO Add walk-to-penalty area/ centre circle during CalibrationCommand::INITIALIZE
                _ => HeadMotion::Center,
            };

            Some(MotionCommand::Stand { head: head_motion })
        }
        _ => None,
    }
}

use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::{CalibrationPhase, WorldState},
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (world_state.robot.primary_state, &world_state.calibration) {
        (PrimaryState::Calibration, Some(calibration_state)) => {
            let head_motion = match calibration_state.phase {
                CalibrationPhase::LOOKAT { target, camera, .. } => HeadMotion::LookAt {
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

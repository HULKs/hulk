use types::{
    calibration::CalibrationCommand,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    use_stand_head_unstiff_calibration: bool,
) -> Option<MotionCommand> {
    if PrimaryState::Calibration != world_state.robot.primary_state {
        return None;
    }
    if use_stand_head_unstiff_calibration {
        return Some(MotionCommand::Stand {
            head: HeadMotion::Unstiff,
            should_look_for_referee: false,
        });
    }

    let head =
        if let Some(CalibrationCommand { target, camera, .. }) = world_state.calibration_command {
            HeadMotion::LookAt {
                target,
                camera: Some(camera),
                image_region_target: ImageRegion::Bottom,
            }
        } else {
            HeadMotion::Unstiff
        };
    Some(MotionCommand::Stand {
        head,
        should_look_for_referee: false,
    })
}

use linear_algebra::vector;
use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::RemoteControlParameters,
};

pub fn execute(remote_control_parameters: &RemoteControlParameters) -> Option<MotionCommand> {
    let remote_control_motion_command = MotionCommand::WalkWithVelocity {
        head: HeadMotion::Center {
            image_region_target: ImageRegion::Top,
        },
        velocity: vector![
            remote_control_parameters.walk.forward,
            remote_control_parameters.walk.left,
        ],
        angular_velocity: remote_control_parameters.walk.turn,
    };

    Some(remote_control_motion_command)
}

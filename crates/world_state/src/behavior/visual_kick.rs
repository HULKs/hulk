use linear_algebra::{Orientation2, point};
use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    last_motion_command: &MotionCommand,
) -> Option<MotionCommand> {
    match (last_motion_command, world_state.ball) {
        // (MotionCommand::VisualKick { .. }, _) => None,
        (_, None) => None,
        (_, Some(ball_position)) => {
            let ball_in_ground = ball_position.ball_in_ground;
            let head = HeadMotion::LookAt {
                target: ball_in_ground,
                image_region_target: ImageRegion::Center,
            };
            Some(MotionCommand::VisualKick {
                head,
                ball_position: ball_in_ground,
                kick_direction: Orientation2::new(0.0),
                target_position: point!(0.0, 4.0),
                robot_theta_to_field: Orientation2::new(0.0),
                kick_power: 10.0,
            })
        }
    }
}

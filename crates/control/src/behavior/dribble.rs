use types::{
    camera_position::CameraPosition,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::DribblingParameters,
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    parameters: &DribblingParameters,
    walk_speed: f32,
    max_turning_angular_velocity: f32,
) -> Option<MotionCommand> {
    let ball_position = world_state.ball?.ball_in_ground;
    let distance_to_ball = ball_position.coords().norm();
    let head = if distance_to_ball < parameters.distance_to_look_directly_at_the_ball {
        HeadMotion::LookAt {
            target: ball_position,
            image_region_target: ImageRegion::Center,
            camera: Some(CameraPosition::Bottom),
        }
    } else {
        HeadMotion::LookLeftAndRightOf {
            target: ball_position,
        }
    };

    Some(MotionCommand::WalkWithVelocity {
        head,
        velocity: ball_position.coords().normalize() * walk_speed,
        angular_velocity: ball_position
            .coords()
            .y()
            .clamp(-max_turning_angular_velocity, max_turning_angular_velocity),
    })
}

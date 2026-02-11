use std::f32::consts::PI;

use coordinate_systems::Ground;
use linear_algebra::{Rotation2, Vector2};
use types::{
    ball_position::BallPosition,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::WalkWithVelocityParameters,
};

pub fn execute(
    ball_position: Option<BallPosition<Ground>>,
    walk_with_velocity_parameter: WalkWithVelocityParameters,
) -> Option<MotionCommand> {
    let next_motion_command = match ball_position {
        Some(ball_position) => {
            let ball_coordinates_in_ground = ball_position.position.coords();
            let head = HeadMotion::LookAt {
                target: ball_position.position,
                image_region_target: ImageRegion::Center,
            };
            let max_angular_velocity_abs = walk_with_velocity_parameter.max_angular_velocity.abs();
            let normalized_angle_to_ball =
                Rotation2::rotation_between(Vector2::x_axis(), ball_coordinates_in_ground).angle()
                    / (0.5 * PI);
            MotionCommand::WalkWithVelocity {
                head,
                velocity: ball_coordinates_in_ground.normalize()
                    * walk_with_velocity_parameter.max_velocity,
                angular_velocity: (normalized_angle_to_ball
                    * walk_with_velocity_parameter.angular_velocity_scaling_factor)
                    .clamp(-max_angular_velocity_abs, max_angular_velocity_abs),
            }
        }
        None => MotionCommand::Stand {
            head: HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            },
        },
    };
    Some(next_motion_command)
}

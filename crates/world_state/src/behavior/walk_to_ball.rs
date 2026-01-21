use std::f32::consts::PI;

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::MainOutput;
use linear_algebra::{Rotation2, Vector2};
use serde::{Deserialize, Serialize};
use types::ball_position::BallPosition;
use types::motion_command::{HeadMotion, MotionCommand};
use types::parameters::WalkWithVelocityParameters;

#[derive(Deserialize, Serialize)]
pub struct WalkToBall {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,

    walk_with_velocity_parameter:
        Parameter<WalkWithVelocityParameters, "behavior.walk_with_velocity">,
    angular_velocitiy_scaling_factor: Parameter<f32, "behavior.angular_velocitiy_scaling_factor">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl WalkToBall {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let next_motion_command = match context.ball_position {
            Some(ball_position) => {
                let ball_coordinates_in_ground = ball_position.position.coords();
                let head = HeadMotion::Center;
                let max_angular_velocity_abs = context
                    .walk_with_velocity_parameter
                    .max_angular_velocity
                    .abs();
                let normalized_angle_to_ball =
                    Rotation2::rotation_between(Vector2::x_axis(), ball_coordinates_in_ground)
                        .angle()
                        / (0.5 * PI);
                MotionCommand::WalkWithVelocity {
                    head,
                    velocity: ball_coordinates_in_ground.normalize()
                        * context.walk_with_velocity_parameter.max_velocity,
                    angular_velocity: (normalized_angle_to_ball
                        * context.angular_velocitiy_scaling_factor)
                        .clamp(-max_angular_velocity_abs, max_angular_velocity_abs),
                }
            }
            None => MotionCommand::Stand {
                head: HeadMotion::Center,
            },
        };

        Ok(MainOutputs {
            motion_command: next_motion_command.into(),
        })
    }
}

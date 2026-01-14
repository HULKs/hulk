use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::ball_position::BallPosition;
use types::motion_command::{HeadMotion, MotionCommand};

#[derive(Deserialize, Serialize)]
pub struct WalkToBall {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
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
                MotionCommand::WalkWithVelocity {
                    head,
                    velocity: ball_coordinates_in_ground.normalize() * 0.5, // TODO: parameterize
                    angular_velocity: ball_coordinates_in_ground.y().clamp(-0.25, 0.25), // TODO: parameterize
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

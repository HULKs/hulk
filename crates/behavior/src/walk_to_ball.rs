use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use serde::{Deserialize, Serialize};
use types::cycle_time::CycleTime;
use types::world_state::BallState;
use types::{
    camera_position::CameraPosition,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::DribblingParameters,
    world_state::WorldState,
};

#[derive(Deserialize, Serialize)]
pub struct WalkToBall {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    world_state: Input<WorldState, "world_state">,
    ball_state: Input<BallState, "World_state", "ball_state">,
    // dribble_walk_speed: Parameter<WalkSpeed, "walk_speed.dribble">,
    // parameters: Parameter<BehaviorParameters, "behavior">,
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
        let ball_position = context.ball_state.ball_in_ground;
        let head = HeadMotion::LookAt {
            target: ball_position,
            image_region_target: ImageRegion::Center,
            camera: Some(CameraPosition::Bottom),
        };

        Ok(MainOutputs {
            motion_command: MotionCommand::WalkWithVelocity {
                head,
                velocity: ball_position.coords().normalize() * 0.1,
                angular_velocity: ball_position.coords().y().clamp(-0.25, 0.25), // TODO: parameterize
            }
            .into(),
        })
    }
}

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Ground;
use framework::{AdditionalOutput, MainOutput};
use types::{
    action::Action, ball_position::BallPosition, motion_command::MotionCommand,
    parameters::WalkWithVelocityParameters,
};

use crate::behavior::walk_to_ball;

#[derive(Deserialize, Serialize)]
pub struct Behavior {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    walk_with_velocity_parameter:
        Parameter<WalkWithVelocityParameters, "behavior.walk_with_velocity">,
    active_action_output: AdditionalOutput<Action, "active_action">,
    last_motion_command: CyclerState<MotionCommand, "last_motion_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl Behavior {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        #[allow(clippy::useless_vec, unused_mut)]
        let mut actions = vec![Action::WalkToBall];

        let (action, motion_command) = actions
            .iter()
            .find_map(|action| {
                let motion_command = match action {
                    Action::WalkToBall => walk_to_ball::execute(
                        context.ball_position.copied(),
                        context.walk_with_velocity_parameter.clone(),
                    ),
                }?;
                Some((action, motion_command))
            })
            .unwrap_or_else(|| panic!("there has to be at least one action available",));
        context.active_action_output.fill_if_subscribed(|| *action);

        *context.last_motion_command = motion_command.clone();

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}

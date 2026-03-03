use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Ground;
use framework::{AdditionalOutput, MainOutput};
use types::{
    action::Action, ball_position::BallPosition, motion_command::MotionCommand,
    parameters::{RemoteControlParameters, WalkWithVelocityParameters}, world_state::WorldState,
};

use crate::behavior::{
    finish_pose, initial, look_around, penalize, safe, stand_at_penalty_kick, stand_up,
    walk_to_ball,
};

#[derive(Deserialize, Serialize)]
pub struct Behavior {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    active_action_output: AdditionalOutput<Action, "active_action">,
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,

    walk_with_velocity_parameter:
        Parameter<WalkWithVelocityParameters, "behavior.walk_with_velocity">,
    remote_control_parameters: Parameter<RemoteControlParameters, "behavior.remote_control">,

    active_action_output: AdditionalOutput<Action, "active_action">,

    last_motion_command: CyclerState<MotionCommand, "last_motion_command">,
    world_state: Input<WorldState, "world_state">,
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
        let world_state = context.world_state;

        #[allow(clippy::useless_vec, unused_mut)]
        let mut actions = vec![
            Action::Safe,
            Action::FinishPose,
            Action::Penalize,
            Action::Initial,
            Action::StandUp,
            Action::StandAtPenaltyKick,
            Action::WalkToBall,
        ];

        if context.remote_control_parameters.enable {
            actions.insert(0, Action::RemoteControl);
        }

        let (action, motion_command) = actions
            .iter()
            .find_map(|action| {
                let motion_command = match action {
                    Action::Safe => safe::execute(world_state),
                    Action::Penalize => penalize::execute(world_state),
                    Action::Initial => initial::execute(world_state),
                    Action::FinishPose => finish_pose::execute(world_state),
                    Action::StandUp => stand_up::execute(world_state),
                    Action::StandAtPenaltyKick => stand_at_penalty_kick::execute(
                        world_state,
                        context.field_dimensions,
                        &context.world_state.robot.role,
                    ),
                    Action::LookAround => look_around::execute(world_state),

                    Action::WalkToBall => walk_to_ball::execute(
                        context.ball_position.copied(),
                        context.walk_with_velocity_parameter.clone(),
                    ),
                    Action::RemoteControl => {
                        remote_control::execute(context.remote_control_parameters)
                    }
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

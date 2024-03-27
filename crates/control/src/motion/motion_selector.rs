use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::{JumpDirection, MotionCommand, StandUpVariant},
    motion_selection::{MotionSafeExits, MotionSelection, MotionVariant},
};

#[derive(Deserialize, Serialize)]
pub struct MotionSelector {
    current_motion: MotionVariant,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,
    has_ground_contact: Input<bool, "has_ground_contact">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_selection: MainOutput<MotionSelection>,
}

impl MotionSelector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_motion: MotionVariant::Unstiff,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let is_current_motion_safe_to_exit = context.motion_safe_exits[self.current_motion];
        let requested_motion = motion_type_from_command(context.motion_command);

        self.current_motion = transition_motion(
            self.current_motion,
            requested_motion,
            is_current_motion_safe_to_exit,
            *context.has_ground_contact,
        );

        let dispatching_motion = if self.current_motion == MotionVariant::Dispatching {
            if requested_motion == MotionVariant::Unstiff {
                Some(MotionVariant::SitDown)
            } else {
                Some(requested_motion)
            }
        } else {
            None
        };

        Ok(MainOutputs {
            motion_selection: MotionSelection {
                current_motion: self.current_motion,
                dispatching_motion,
            }
            .into(),
        })
    }
}

fn motion_type_from_command(command: &MotionCommand) -> MotionVariant {
    match command {
        MotionCommand::ArmsUpSquat => MotionVariant::ArmsUpSquat,
        MotionCommand::FallProtection { .. } => MotionVariant::FallProtection,
        MotionCommand::Initial => MotionVariant::Initial,
        MotionCommand::Jump { direction } => match direction {
            JumpDirection::Left => MotionVariant::JumpLeft,
            JumpDirection::Right => MotionVariant::JumpRight,
        },
        MotionCommand::Penalized => MotionVariant::Penalized,
        MotionCommand::SitDown { .. } => MotionVariant::SitDown,
        MotionCommand::Stand { .. } => MotionVariant::Stand,
        MotionCommand::StandUp {
            variant: StandUpVariant::Front,
        } => MotionVariant::StandUpFront,
        MotionCommand::StandUp {
            variant: StandUpVariant::Back,
        } => MotionVariant::StandUpBack,
        MotionCommand::StandUp {
            variant: StandUpVariant::Sitting,
        } => MotionVariant::StandUpSitting,
        MotionCommand::StandUp {
            variant: StandUpVariant::Squatting,
        } => MotionVariant::StandUpSquatting,
        MotionCommand::Unstiff => MotionVariant::Unstiff,
        MotionCommand::Walk { .. } => MotionVariant::Walk,
        MotionCommand::InWalkKick { .. } => MotionVariant::Walk,
    }
}

fn transition_motion(
    from: MotionVariant,
    to: MotionVariant,
    motion_safe_to_exit: bool,
    has_ground_contact: bool,
) -> MotionVariant {
    match (from, motion_safe_to_exit, to) {
        (MotionVariant::Unstiff, _, MotionVariant::Unstiff) => MotionVariant::Unstiff,
        (MotionVariant::SitDown, true, MotionVariant::Unstiff) => MotionVariant::Unstiff,
        (MotionVariant::Dispatching, true, MotionVariant::Unstiff) => MotionVariant::SitDown,
        (_, _, MotionVariant::Unstiff) if !has_ground_contact => MotionVariant::Unstiff,
        (_, _, MotionVariant::FallProtection) => MotionVariant::FallProtection,
        (MotionVariant::Stand, _, MotionVariant::Walk) => MotionVariant::Walk,
        (MotionVariant::Walk, _, MotionVariant::Stand) => MotionVariant::Stand,
        (MotionVariant::Dispatching, true, _) => to,
        (from, true, to) if from != to => MotionVariant::Dispatching,
        _ => from,
    }
}

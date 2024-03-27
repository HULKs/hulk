use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::{JumpDirection, MotionCommand, StandUpVariant},
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
};

#[derive(Deserialize, Serialize)]
pub struct MotionSelector {
    current_motion: MotionType,
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
            current_motion: MotionType::Unstiff,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let is_current_motion_safe_to_exit = context.motion_safe_exits[self.current_motion];
        let requested_motion = motion_type_from_command(context.motion_command);

        self.current_motion = transition_motion(
            self.current_motion,
            requested_motion,
            is_current_motion_safe_to_exit,
        );

        let dispatching_motion = if self.current_motion == MotionType::Dispatching {
            if requested_motion == MotionType::Unstiff {
                Some(MotionType::SitDown)
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

fn motion_type_from_command(command: &MotionCommand) -> MotionType {
    match command {
        MotionCommand::ArmsUpSquat => MotionType::ArmsUpSquat,
        MotionCommand::FallProtection { .. } => MotionType::FallProtection,
        MotionCommand::Initial => MotionType::Initial,
        MotionCommand::Jump { direction } => match direction {
            JumpDirection::Left => MotionType::JumpLeft,
            JumpDirection::Right => MotionType::JumpRight,
        },
        MotionCommand::Penalized => MotionType::Penalized,
        MotionCommand::SitDown { .. } => MotionType::SitDown,
        MotionCommand::Stand { .. } => MotionType::Stand,
        MotionCommand::StandUp {
            variant: StandUpVariant::Front,
        } => MotionType::StandUpFront,
        MotionCommand::StandUp {
            variant: StandUpVariant::Back,
        } => MotionType::StandUpBack,
        MotionCommand::StandUp {
            variant: StandUpVariant::Sitting,
        } => MotionType::StandUpSitting,
        MotionCommand::StandUp {
            variant: StandUpVariant::Squatting,
        } => MotionType::StandUpSquatting,
        MotionCommand::Unstiff => MotionType::Unstiff,
        MotionCommand::Walk { .. } => MotionType::Walk,
        MotionCommand::InWalkKick { .. } => MotionType::Walk,
    }
}

fn transition_motion(from: MotionType, to: MotionType, motion_safe_to_exit: bool) -> MotionType {
    match (from, motion_safe_to_exit, to) {
        (_, _, MotionType::Unstiff) => MotionType::Unstiff,
        (_, _, MotionType::FallProtection) => MotionType::FallProtection,
        (MotionType::Stand, _, MotionType::Walk) => MotionType::Walk,
        (MotionType::Walk, _, MotionType::Stand) => MotionType::Stand,
        (MotionType::Dispatching, true, _) => to,
        (from, true, to) if from != to => MotionType::Dispatching,
        _ => from,
    }
}

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{Facing, JumpDirection, MotionCommand, MotionSafeExits, MotionSelection, MotionType};

pub struct MotionSelector {
    current_motion: MotionType,
    dispatching_motion: Option<MotionType>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,
    has_ground_contact: Input<bool, "has_ground_contact">,

    motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    enable_energy_saving_stand: Parameter<bool, "energy_saving_stand.enabled">,
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
            dispatching_motion: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let motion_safe_to_exit = context.motion_safe_exits[self.current_motion];
        let requested_motion =
            motion_type_from_command(context.motion_command, *context.enable_energy_saving_stand);

        self.current_motion = transition_motion(
            self.current_motion,
            requested_motion,
            motion_safe_to_exit,
            *context.has_ground_contact,
        );

        self.dispatching_motion = if self.current_motion == MotionType::Dispatching {
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
                dispatching_motion: self.dispatching_motion,
            }
            .into(),
        })
    }
}

fn motion_type_from_command(
    command: &MotionCommand,
    enable_energy_saving_stand: bool,
) -> MotionType {
    match command {
        MotionCommand::ArmsUpSquat => MotionType::ArmsUpSquat,
        MotionCommand::FallProtection { .. } => MotionType::FallProtection,
        MotionCommand::Jump { direction } => match direction {
            JumpDirection::Left => MotionType::JumpLeft,
            JumpDirection::Right => MotionType::JumpRight,
        },
        MotionCommand::Penalized => MotionType::Penalized,
        MotionCommand::SitDown { .. } => MotionType::SitDown,
        MotionCommand::Stand {
            is_energy_saving, ..
        } => {
            if enable_energy_saving_stand && *is_energy_saving {
                MotionType::EnergySavingStand
            } else {
                MotionType::Stand
            }
        }
        MotionCommand::StandUp { facing } => match facing {
            Facing::Down => MotionType::StandUpFront,
            Facing::Up => MotionType::StandUpBack,
        },
        MotionCommand::Unstiff => MotionType::Unstiff,
        MotionCommand::Walk { .. } => MotionType::Walk,
        MotionCommand::InWalkKick { .. } => MotionType::Walk,
    }
}

fn transition_motion(
    from: MotionType,
    to: MotionType,
    motion_safe_to_exit: bool,
    has_ground_contact: bool,
) -> MotionType {
    match (from, motion_safe_to_exit, to, has_ground_contact) {
        (MotionType::SitDown, true, MotionType::Unstiff, _) => MotionType::Unstiff,
        (_, _, MotionType::Unstiff, false) => MotionType::Unstiff,
        (MotionType::Dispatching, true, MotionType::Unstiff, true) => MotionType::SitDown,
        (MotionType::StandUpFront, _, MotionType::FallProtection, _) => MotionType::StandUpFront,
        (MotionType::StandUpBack, _, MotionType::FallProtection, _) => MotionType::StandUpBack,
        (MotionType::StandUpFront, true, MotionType::StandUpFront, _) => MotionType::Dispatching,
        (MotionType::StandUpBack, true, MotionType::StandUpBack, _) => MotionType::Dispatching,
        (_, _, MotionType::FallProtection, _) => MotionType::FallProtection,
        (MotionType::Dispatching, true, _, _) => to,
        (MotionType::Stand, _, MotionType::Walk, _) => MotionType::Walk,
        (MotionType::Walk, _, MotionType::Stand, _) => MotionType::Stand,
        (from, true, to, _) if from != to => MotionType::Dispatching,
        _ => from,
    }
}

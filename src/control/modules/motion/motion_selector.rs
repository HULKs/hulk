use module_derive::{module, require_some};
use types::{Facing, JumpDirection, MotionCommand, MotionSafeExits, MotionSelection, MotionType};

pub struct MotionSelector {
    current_motion: MotionType,
    dispatching_motion: Option<MotionType>,
}

#[module(control)]
#[input(path = motion_command, data_type = MotionCommand)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[main_output(data_type = MotionSelection)]
impl MotionSelector {}

impl MotionSelector {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            current_motion: MotionType::Unstiff,
            dispatching_motion: None,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion = require_some!(context.motion_command);

        let is_active_motion_safe_to_exit = context.motion_safe_exits[self.current_motion];
        let requested_motion = motion_type_from_command(motion);
        self.current_motion = transition_motion(
            self.current_motion,
            requested_motion,
            is_active_motion_safe_to_exit,
        );
        self.dispatching_motion = if self.current_motion == MotionType::Dispatching {
            Some(requested_motion)
        } else {
            None
        };

        Ok(MainOutputs {
            motion_selection: Some(MotionSelection {
                current_motion: self.current_motion,
                dispatching_motion: self.dispatching_motion,
            }),
        })
    }
}

fn motion_type_from_command(command: &MotionCommand) -> MotionType {
    match command {
        MotionCommand::ArmsUpSquat => MotionType::ArmsUpSquat,
        MotionCommand::FallProtection { .. } => MotionType::FallProtection,
        MotionCommand::Jump { direction } => match direction {
            JumpDirection::Left => MotionType::JumpLeft,
            JumpDirection::Right => MotionType::JumpRight,
        },
        MotionCommand::Penalized => MotionType::Penalized,
        MotionCommand::SitDown { .. } => MotionType::SitDown,
        MotionCommand::Stand { .. } => MotionType::Stand,
        MotionCommand::StandUp { facing } => match facing {
            Facing::Down => MotionType::StandUpFront,
            Facing::Up => MotionType::StandUpBack,
        },
        MotionCommand::Unstiff => MotionType::Unstiff,
        MotionCommand::Walk { .. } => MotionType::Walk,
        MotionCommand::InWalkKick { .. } => MotionType::Walk,
    }
}

fn transition_motion(from: MotionType, to: MotionType, is_safe_to_exit: bool) -> MotionType {
    match (from, is_safe_to_exit, to) {
        (_, _, MotionType::Unstiff) => MotionType::Unstiff,
        (MotionType::StandUpFront, _, MotionType::FallProtection) => MotionType::StandUpFront,
        (MotionType::StandUpBack, _, MotionType::FallProtection) => MotionType::StandUpBack,
        (_, _, MotionType::FallProtection) => MotionType::FallProtection,
        (MotionType::Dispatching, true, _) => to,
        (MotionType::Stand, _, MotionType::Walk) => MotionType::Walk,
        (MotionType::Walk, _, MotionType::Stand) => MotionType::Stand,
        (from, true, to) if from != to => MotionType::Dispatching,
        _ => from,
    }
}

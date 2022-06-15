use macros::{module, require_some};

use crate::types::{
    BodyMotionSafeExits, BodyMotionType, Facing, HeadMotion, HeadMotionSafeExits, HeadMotionType,
    Motion, MotionCommand, MotionSelection,
};

pub struct MotionSelector {
    current_head_motion: HeadMotionType,
    current_body_motion: BodyMotionType,
    dispatching_head_motion: Option<HeadMotionType>,
    dispatching_body_motion: Option<BodyMotionType>,
}

#[module(control)]
#[input(path = motion_command, data_type = MotionCommand)]
#[persistent_state(path = body_motion_safe_exits, data_type = BodyMotionSafeExits)]
#[persistent_state(path = head_motion_safe_exits, data_type = HeadMotionSafeExits)]
#[main_output(data_type = MotionSelection)]
impl MotionSelector {}

impl MotionSelector {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            current_head_motion: HeadMotionType::Unstiff,
            current_body_motion: BodyMotionType::Unstiff,
            dispatching_head_motion: None,
            dispatching_body_motion: None,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion = require_some!(context.motion_command).motion;

        let is_active_head_motion_safe_to_exit =
            context.head_motion_safe_exits[self.current_head_motion];
        self.current_head_motion =
            self.transition_head_motion(motion, is_active_head_motion_safe_to_exit);
        self.dispatching_head_motion = if self.current_head_motion == HeadMotionType::Dispatching {
            Some(head_motion_type_from_motion(motion))
        } else {
            None
        };
        let is_active_body_motion_safe_to_exit =
            context.body_motion_safe_exits[self.current_body_motion];
        self.current_body_motion =
            self.transition_body_motion(motion, is_active_body_motion_safe_to_exit);
        self.dispatching_body_motion = if self.current_body_motion == BodyMotionType::Dispatching {
            Some(body_motion_type_from_motion(motion))
        } else {
            None
        };

        Ok(MainOutputs {
            motion_selection: Some(MotionSelection {
                current_head_motion: self.current_head_motion,
                current_body_motion: self.current_body_motion,
                dispatching_body_motion: self.dispatching_body_motion,
                dispatching_head_motion: self.dispatching_head_motion,
            }),
        })
    }

    fn transition_head_motion(&self, motion: Motion, is_safe_to_exit: bool) -> HeadMotionType {
        let from = self.current_head_motion;
        let to = head_motion_type_from_motion(motion);

        match (from, is_safe_to_exit, to) {
            (_, _, HeadMotionType::Unstiff) => HeadMotionType::Unstiff,
            (_, _, HeadMotionType::FallProtection)
                if (from != HeadMotionType::StandUpFront
                    && from != HeadMotionType::StandUpBack)
                    || is_safe_to_exit =>
            {
                HeadMotionType::FallProtection
            }
            (HeadMotionType::Dispatching, true, _) => to,
            (from, true, to) if from != to => HeadMotionType::Dispatching,
            _ => from,
        }
    }

    fn transition_body_motion(&self, motion: Motion, is_safe_to_exit: bool) -> BodyMotionType {
        let from = self.current_body_motion;
        let to = body_motion_type_from_motion(motion);
        match (from, is_safe_to_exit, to) {
            (_, _, BodyMotionType::Unstiff) => BodyMotionType::Unstiff,
            (BodyMotionType::StandUpFront, _, BodyMotionType::FallProtection) => {
                BodyMotionType::StandUpFront
            }
            (BodyMotionType::StandUpBack, _, BodyMotionType::FallProtection) => {
                BodyMotionType::StandUpBack
            }
            (_, _, BodyMotionType::FallProtection) => BodyMotionType::FallProtection,
            (BodyMotionType::Dispatching, true, _) => to,
            (BodyMotionType::Stand, _, BodyMotionType::Walk) => BodyMotionType::Walk,
            (BodyMotionType::Walk, _, BodyMotionType::Stand) => BodyMotionType::Stand,
            (from, true, to) if from != to => BodyMotionType::Dispatching,
            _ => from,
        }
    }
}

fn head_motion_type_from_head_motion(motion: HeadMotion) -> HeadMotionType {
    match motion {
        HeadMotion::ZeroAngles => HeadMotionType::ZeroAngles,
        HeadMotion::Center => HeadMotionType::Center,
        HeadMotion::LookAround => HeadMotionType::LookAround,
        HeadMotion::LookAt { .. } => HeadMotionType::LookAt,
        HeadMotion::Unstiff => HeadMotionType::Unstiff,
    }
}

fn head_motion_type_from_motion(motion: Motion) -> HeadMotionType {
    match motion {
        Motion::FallProtection { .. } => HeadMotionType::FallProtection,
        Motion::Jump { .. } => HeadMotionType::Unstiff,
        Motion::Kick { head, .. } => head_motion_type_from_head_motion(head),
        Motion::Penalized => HeadMotionType::Center,
        Motion::SitDown { head } => head_motion_type_from_head_motion(head),
        Motion::Stand { head, .. } => head_motion_type_from_head_motion(head),
        Motion::StandUp { facing } => match facing {
            Facing::Down => HeadMotionType::StandUpFront,
            Facing::Up => HeadMotionType::StandUpBack,
        },
        Motion::Unstiff => HeadMotionType::Unstiff,
        Motion::Walk { head, .. } => head_motion_type_from_head_motion(head),
    }
}

fn body_motion_type_from_motion(motion: Motion) -> BodyMotionType {
    match motion {
        Motion::FallProtection { .. } => BodyMotionType::FallProtection,
        Motion::Jump { .. } => BodyMotionType::Jump,
        Motion::Kick { .. } => BodyMotionType::Kick,
        Motion::Penalized => BodyMotionType::Penalized,
        Motion::SitDown { .. } => BodyMotionType::SitDown,
        Motion::Stand { .. } => BodyMotionType::Stand,
        Motion::StandUp { facing } => match facing {
            Facing::Down => BodyMotionType::StandUpFront,
            Facing::Up => BodyMotionType::StandUpBack,
        },
        Motion::Unstiff => BodyMotionType::Unstiff,
        Motion::Walk { .. } => BodyMotionType::Walk,
    }
}

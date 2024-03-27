use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct MotionSelection {
    pub current_motion: MotionVariant,
    pub dispatching_motion: Option<MotionVariant>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum MotionVariant {
    ArmsUpSquat,
    Dispatching,
    FallProtection,
    Initial,
    JumpLeft,
    JumpRight,
    Penalized,
    SitDown,
    Stand,
    StandUpBack,
    StandUpFront,
    StandUpSitting,
    StandUpSquatting,
    Unstiff,
    Walk,
}

impl Default for MotionVariant {
    fn default() -> Self {
        Self::Unstiff
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct MotionSafeExits {
    arms_up_squat: bool,
    dispatching: bool,
    fall_protection: bool,
    initial: bool,
    jump_left: bool,
    jump_right: bool,
    penalized: bool,
    sit_down: bool,
    stand_up_back: bool,
    stand_up_front: bool,
    stand_up_sitting: bool,
    stand_up_squatting: bool,
    stand: bool,
    unstiff: bool,
    walk: bool,
}

impl Default for MotionSafeExits {
    fn default() -> Self {
        Self {
            arms_up_squat: true,
            dispatching: false,
            fall_protection: true,
            initial: true,
            jump_left: false,
            jump_right: false,
            penalized: true,
            sit_down: false,
            stand_up_back: false,
            stand_up_front: false,
            stand_up_sitting: false,
            stand_up_squatting: false,
            stand: true,
            unstiff: true,
            walk: false,
        }
    }
}

impl Index<MotionVariant> for MotionSafeExits {
    type Output = bool;

    fn index(&self, motion_type: MotionVariant) -> &Self::Output {
        match motion_type {
            MotionVariant::ArmsUpSquat => &self.arms_up_squat,
            MotionVariant::Dispatching => &self.dispatching,
            MotionVariant::Initial => &self.initial,
            MotionVariant::JumpLeft => &self.jump_left,
            MotionVariant::JumpRight => &self.jump_right,
            MotionVariant::FallProtection => &self.fall_protection,
            MotionVariant::Penalized => &self.penalized,
            MotionVariant::SitDown => &self.sit_down,
            MotionVariant::Stand => &self.stand,
            MotionVariant::StandUpBack => &self.stand_up_back,
            MotionVariant::StandUpFront => &self.stand_up_front,
            MotionVariant::StandUpSitting => &self.stand_up_sitting,
            MotionVariant::StandUpSquatting => &self.stand_up_squatting,
            MotionVariant::Unstiff => &self.unstiff,
            MotionVariant::Walk => &self.walk,
        }
    }
}

impl IndexMut<MotionVariant> for MotionSafeExits {
    fn index_mut(&mut self, motion_type: MotionVariant) -> &mut Self::Output {
        match motion_type {
            MotionVariant::ArmsUpSquat => &mut self.arms_up_squat,
            MotionVariant::Dispatching => &mut self.dispatching,
            MotionVariant::Initial => &mut self.initial,
            MotionVariant::JumpLeft => &mut self.jump_left,
            MotionVariant::JumpRight => &mut self.jump_right,
            MotionVariant::FallProtection => &mut self.fall_protection,
            MotionVariant::Penalized => &mut self.penalized,
            MotionVariant::SitDown => &mut self.sit_down,
            MotionVariant::Stand => &mut self.stand,
            MotionVariant::StandUpBack => &mut self.stand_up_back,
            MotionVariant::StandUpFront => &mut self.stand_up_front,
            MotionVariant::StandUpSitting => &mut self.stand_up_sitting,
            MotionVariant::StandUpSquatting => &mut self.stand_up_squatting,
            MotionVariant::Unstiff => &mut self.unstiff,
            MotionVariant::Walk => &mut self.walk,
        }
    }
}

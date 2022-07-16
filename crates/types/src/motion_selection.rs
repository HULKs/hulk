use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct MotionSelection {
    pub current_motion: MotionType,
    pub dispatching_motion: Option<MotionType>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum MotionType {
    ArmsUpSquat,
    Dispatching,
    FallProtection,
    JumpLeft,
    JumpRight,
    Penalized,
    SitDown,
    Stand,
    StandUpBack,
    StandUpFront,
    Unstiff,
    Walk,
}

impl Default for MotionType {
    fn default() -> Self {
        Self::Unstiff
    }
}

#[derive(Clone, Debug)]
pub struct MotionSafeExits {
    arms_up_squat: bool,
    dispatching: bool,
    fall_protection: bool,
    jump_left: bool,
    jump_right: bool,
    penalized: bool,
    sit_down: bool,
    stand_up_back: bool,
    stand_up_front: bool,
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
            jump_left: false,
            jump_right: false,
            penalized: true,
            sit_down: false,
            stand_up_back: false,
            stand_up_front: false,
            stand: true,
            unstiff: true,
            walk: false,
        }
    }
}

impl Index<MotionType> for MotionSafeExits {
    type Output = bool;

    fn index(&self, motion_type: MotionType) -> &Self::Output {
        match motion_type {
            MotionType::ArmsUpSquat => &self.arms_up_squat,
            MotionType::Dispatching => &self.dispatching,
            MotionType::JumpLeft => &self.jump_left,
            MotionType::JumpRight => &self.jump_right,
            MotionType::FallProtection => &self.fall_protection,
            MotionType::Penalized => &self.penalized,
            MotionType::SitDown => &self.sit_down,
            MotionType::Stand => &self.stand,
            MotionType::StandUpBack => &self.stand_up_back,
            MotionType::StandUpFront => &self.stand_up_front,
            MotionType::Unstiff => &self.unstiff,
            MotionType::Walk => &self.walk,
        }
    }
}

impl IndexMut<MotionType> for MotionSafeExits {
    fn index_mut(&mut self, motion_type: MotionType) -> &mut Self::Output {
        match motion_type {
            MotionType::ArmsUpSquat => &mut self.arms_up_squat,
            MotionType::Dispatching => &mut self.dispatching,
            MotionType::JumpLeft => &mut self.jump_left,
            MotionType::JumpRight => &mut self.jump_right,
            MotionType::FallProtection => &mut self.fall_protection,
            MotionType::Penalized => &mut self.penalized,
            MotionType::SitDown => &mut self.sit_down,
            MotionType::Stand => &mut self.stand,
            MotionType::StandUpBack => &mut self.stand_up_back,
            MotionType::StandUpFront => &mut self.stand_up_front,
            MotionType::Unstiff => &mut self.unstiff,
            MotionType::Walk => &mut self.walk,
        }
    }
}

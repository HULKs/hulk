use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct MotionSelection {
    #[leaf]
    pub current_motion: MotionType,
    #[leaf]
    pub dispatching_motion: Option<MotionType>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum MotionType {
    Dispatching,
    FallProtection,
    Jump,
    Kick,
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
    dispatching: bool,
    fall_protection: bool,
    jump: bool,
    kick: bool,
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
            dispatching: false,
            fall_protection: true,
            jump: false,
            kick: false,
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
            MotionType::Dispatching => &self.dispatching,
            MotionType::FallProtection => &self.fall_protection,
            MotionType::Jump => &self.jump,
            MotionType::Kick => &self.kick,
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
            MotionType::Dispatching => &mut self.dispatching,
            MotionType::FallProtection => &mut self.fall_protection,
            MotionType::Jump => &mut self.jump,
            MotionType::Kick => &mut self.kick,
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

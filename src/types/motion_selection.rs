use std::ops::{Index, IndexMut};

use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct MotionSelection {
    #[leaf]
    pub current_body_motion: BodyMotionType,
    #[leaf]
    pub current_head_motion: HeadMotionType,
    #[leaf]
    pub dispatching_body_motion: Option<BodyMotionType>,
    #[leaf]
    pub dispatching_head_motion: Option<HeadMotionType>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum BodyMotionType {
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

impl Default for BodyMotionType {
    fn default() -> Self {
        Self::Unstiff
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum HeadMotionType {
    Center,
    Dispatching,
    FallProtection,
    LookAround,
    LookAt,
    StandUpBack,
    StandUpFront,
    Unstiff,
    ZeroAngles,
}

impl Default for HeadMotionType {
    fn default() -> Self {
        Self::Unstiff
    }
}

#[derive(Clone, Debug)]
pub struct BodyMotionSafeExits {
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

impl Default for BodyMotionSafeExits {
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

impl Index<BodyMotionType> for BodyMotionSafeExits {
    type Output = bool;

    fn index(&self, motion_type: BodyMotionType) -> &Self::Output {
        match motion_type {
            BodyMotionType::Dispatching => &self.dispatching,
            BodyMotionType::FallProtection => &self.fall_protection,
            BodyMotionType::Jump => &self.jump,
            BodyMotionType::Kick => &self.kick,
            BodyMotionType::Penalized => &self.penalized,
            BodyMotionType::SitDown => &self.sit_down,
            BodyMotionType::Stand => &self.stand,
            BodyMotionType::StandUpBack => &self.stand_up_back,
            BodyMotionType::StandUpFront => &self.stand_up_front,
            BodyMotionType::Unstiff => &self.unstiff,
            BodyMotionType::Walk => &self.walk,
        }
    }
}

impl IndexMut<BodyMotionType> for BodyMotionSafeExits {
    fn index_mut(&mut self, motion_type: BodyMotionType) -> &mut Self::Output {
        match motion_type {
            BodyMotionType::Dispatching => &mut self.dispatching,
            BodyMotionType::FallProtection => &mut self.fall_protection,
            BodyMotionType::Jump => &mut self.jump,
            BodyMotionType::Kick => &mut self.kick,
            BodyMotionType::Penalized => &mut self.penalized,
            BodyMotionType::SitDown => &mut self.sit_down,
            BodyMotionType::Stand => &mut self.stand,
            BodyMotionType::StandUpBack => &mut self.stand_up_back,
            BodyMotionType::StandUpFront => &mut self.stand_up_front,
            BodyMotionType::Unstiff => &mut self.unstiff,
            BodyMotionType::Walk => &mut self.walk,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HeadMotionSafeExits {
    center: bool,
    dispatching: bool,
    look_around: bool,
    look_at: bool,
    protect: bool,
    stand_up_back: bool,
    stand_up_front: bool,
    unstiff: bool,
    zero_angles: bool,
}

impl Default for HeadMotionSafeExits {
    fn default() -> Self {
        Self {
            center: true,
            dispatching: false,
            look_around: false,
            look_at: false,
            protect: true,
            stand_up_back: false,
            stand_up_front: false,
            unstiff: true,
            zero_angles: false,
        }
    }
}

impl Index<HeadMotionType> for HeadMotionSafeExits {
    type Output = bool;

    fn index(&self, motion_type: HeadMotionType) -> &Self::Output {
        match motion_type {
            HeadMotionType::Center => &self.center,
            HeadMotionType::Dispatching => &self.dispatching,
            HeadMotionType::FallProtection => &self.protect,
            HeadMotionType::LookAround => &self.look_around,
            HeadMotionType::LookAt => &self.look_at,
            HeadMotionType::StandUpBack => &self.stand_up_back,
            HeadMotionType::StandUpFront => &self.stand_up_front,
            HeadMotionType::Unstiff => &self.unstiff,
            HeadMotionType::ZeroAngles => &self.zero_angles,
        }
    }
}

impl IndexMut<HeadMotionType> for HeadMotionSafeExits {
    fn index_mut(&mut self, motion_type: HeadMotionType) -> &mut Self::Output {
        match motion_type {
            HeadMotionType::Center => &mut self.center,
            HeadMotionType::Dispatching => &mut self.dispatching,
            HeadMotionType::FallProtection => &mut self.protect,
            HeadMotionType::LookAround => &mut self.look_around,
            HeadMotionType::LookAt => &mut self.look_at,
            HeadMotionType::StandUpBack => &mut self.stand_up_back,
            HeadMotionType::StandUpFront => &mut self.stand_up_front,
            HeadMotionType::Unstiff => &mut self.unstiff,
            HeadMotionType::ZeroAngles => &mut self.zero_angles,
        }
    }
}

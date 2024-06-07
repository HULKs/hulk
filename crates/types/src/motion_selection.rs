use std::ops::{Index, IndexMut};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, Serialize, PathSerialize, PathDeserialize, PathIntrospect, Deserialize,
)]
pub struct MotionSelection {
    pub current_motion: MotionType,
    pub dispatching_motion: Option<MotionType>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum MotionType {
    Animation,
    AnimationStiff,
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
    Unstiff,
    Walk,
}

impl Default for MotionType {
    fn default() -> Self {
        Self::Unstiff
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct MotionSafeExits {
    animation: bool,
    animationstiff: bool,
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
    stand: bool,
    unstiff: bool,
    walk: bool,
}

impl MotionSafeExits {
    pub fn fill(value: bool) -> Self {
        Self {
            animation: value,
            animationstiff: value,
            arms_up_squat: value,
            dispatching: value,
            fall_protection: value,
            initial: value,
            jump_left: value,
            jump_right: value,
            penalized: value,
            sit_down: value,
            stand_up_back: value,
            stand_up_front: value,
            stand_up_sitting: value,
            stand: value,
            unstiff: value,
            walk: value,
        }
    }
}

impl Default for MotionSafeExits {
    fn default() -> Self {
        Self {
            animation: true,
            animationstiff: true,
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
            MotionType::Animation => &self.animation,
            MotionType::AnimationStiff => &self.animationstiff,
            MotionType::ArmsUpSquat => &self.arms_up_squat,
            MotionType::Dispatching => &self.dispatching,
            MotionType::Initial => &self.initial,
            MotionType::JumpLeft => &self.jump_left,
            MotionType::JumpRight => &self.jump_right,
            MotionType::FallProtection => &self.fall_protection,
            MotionType::Penalized => &self.penalized,
            MotionType::SitDown => &self.sit_down,
            MotionType::Stand => &self.stand,
            MotionType::StandUpBack => &self.stand_up_back,
            MotionType::StandUpFront => &self.stand_up_front,
            MotionType::StandUpSitting => &self.stand_up_sitting,
            MotionType::Unstiff => &self.unstiff,
            MotionType::Walk => &self.walk,
        }
    }
}

impl IndexMut<MotionType> for MotionSafeExits {
    fn index_mut(&mut self, motion_type: MotionType) -> &mut Self::Output {
        match motion_type {
            MotionType::Animation => &mut self.animation,
            MotionType::AnimationStiff => &mut self.animationstiff,
            MotionType::ArmsUpSquat => &mut self.arms_up_squat,
            MotionType::Dispatching => &mut self.dispatching,
            MotionType::Initial => &mut self.initial,
            MotionType::JumpLeft => &mut self.jump_left,
            MotionType::JumpRight => &mut self.jump_right,
            MotionType::FallProtection => &mut self.fall_protection,
            MotionType::Penalized => &mut self.penalized,
            MotionType::SitDown => &mut self.sit_down,
            MotionType::Stand => &mut self.stand,
            MotionType::StandUpBack => &mut self.stand_up_back,
            MotionType::StandUpFront => &mut self.stand_up_front,
            MotionType::StandUpSitting => &mut self.stand_up_sitting,
            MotionType::Unstiff => &mut self.unstiff,
            MotionType::Walk => &mut self.walk,
        }
    }
}

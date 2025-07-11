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
    ArmsUpStand,
    Dispatching,
    FallProtection,
    Initial,
    JumpLeft,
    JumpRight,
    CenterJump,
    Penalized,
    SitDown,
    Stand,
    StandUpBack,
    StandUpFront,
    StandUpSitting,
    StandUpFrontSlow,
    StandUpSittingSlow,
    Unstiff,
    Walk,
    WideStance,
    KeeperJumpLeft,
    KeeperJumpRight,
}

impl Default for MotionType {
    fn default() -> Self {
        Self::Unstiff
    }
}

impl MotionType {
    pub fn is_standup_motion(self) -> bool {
        self == MotionType::StandUpBack
            || self == MotionType::StandUpFront
            || self == MotionType::StandUpSitting
            || self == MotionType::StandUpSittingSlow
            || self == MotionType::StandUpFrontSlow
    }

    pub fn is_dispatching(self) -> bool {
        self == MotionType::Dispatching
    }

    pub fn is_stable(self) -> bool {
        self == MotionType::Stand
            || self == MotionType::Walk
            || self == MotionType::Initial
            || self == MotionType::Unstiff
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct MotionSafeExits {
    animation: bool,
    animation_stiff: bool,
    arms_up_squat: bool,
    arms_up_stand: bool,
    dispatching: bool,
    fall_protection: bool,
    initial: bool,
    jump_left: bool,
    jump_right: bool,
    center_jump: bool,
    penalized: bool,
    sit_down: bool,
    stand_up_back: bool,
    stand_up_front: bool,
    stand_up_sitting: bool,
    stand_up_front_slow: bool,
    stand_up_sitting_slow: bool,
    stand: bool,
    unstiff: bool,
    walk: bool,
    wide_stance: bool,
    keeper_jump_left: bool,
    keeper_jump_right: bool,
}

impl MotionSafeExits {
    pub fn fill(value: bool) -> Self {
        Self {
            animation: value,
            animation_stiff: value,
            arms_up_squat: value,
            arms_up_stand: value,
            dispatching: value,
            fall_protection: value,
            initial: value,
            jump_left: value,
            jump_right: value,
            center_jump: value,
            penalized: value,
            sit_down: value,
            stand_up_back: value,
            stand_up_front: value,
            stand_up_sitting: value,
            stand_up_front_slow: value,
            stand_up_sitting_slow: value,
            stand: value,
            unstiff: value,
            walk: value,
            wide_stance: value,
            keeper_jump_left: value,
            keeper_jump_right: value,
        }
    }
}

impl Default for MotionSafeExits {
    fn default() -> Self {
        Self {
            animation: true,
            animation_stiff: true,
            arms_up_squat: true,
            arms_up_stand: true,
            dispatching: false,
            fall_protection: true,
            initial: true,
            jump_left: false,
            jump_right: false,
            center_jump: false,
            penalized: true,
            sit_down: false,
            stand_up_back: false,
            stand_up_front: false,
            stand_up_sitting: false,
            stand_up_front_slow: false,
            stand_up_sitting_slow: false,
            stand: true,
            unstiff: true,
            walk: false,
            wide_stance: false,
            keeper_jump_left: false,
            keeper_jump_right: false,
        }
    }
}

impl Index<MotionType> for MotionSafeExits {
    type Output = bool;

    fn index(&self, motion_type: MotionType) -> &Self::Output {
        match motion_type {
            MotionType::Animation => &self.animation,
            MotionType::AnimationStiff => &self.animation_stiff,
            MotionType::ArmsUpSquat => &self.arms_up_squat,
            MotionType::ArmsUpStand => &self.arms_up_stand,
            MotionType::Dispatching => &self.dispatching,
            MotionType::Initial => &self.initial,
            MotionType::JumpLeft => &self.jump_left,
            MotionType::JumpRight => &self.jump_right,
            MotionType::CenterJump => &self.center_jump,
            MotionType::FallProtection => &self.fall_protection,
            MotionType::Penalized => &self.penalized,
            MotionType::SitDown => &self.sit_down,
            MotionType::Stand => &self.stand,
            MotionType::StandUpBack => &self.stand_up_back,
            MotionType::StandUpFront => &self.stand_up_front,
            MotionType::StandUpSitting => &self.stand_up_sitting,
            MotionType::StandUpFrontSlow => &self.stand_up_front_slow,
            MotionType::StandUpSittingSlow => &self.stand_up_sitting_slow,
            MotionType::Unstiff => &self.unstiff,
            MotionType::Walk => &self.walk,
            MotionType::WideStance => &self.wide_stance,
            MotionType::KeeperJumpLeft => &self.keeper_jump_left,
            MotionType::KeeperJumpRight => &self.keeper_jump_right,
        }
    }
}

impl IndexMut<MotionType> for MotionSafeExits {
    fn index_mut(&mut self, motion_type: MotionType) -> &mut Self::Output {
        match motion_type {
            MotionType::Animation => &mut self.animation,
            MotionType::AnimationStiff => &mut self.animation_stiff,
            MotionType::ArmsUpSquat => &mut self.arms_up_squat,
            MotionType::ArmsUpStand => &mut self.arms_up_stand,
            MotionType::Dispatching => &mut self.dispatching,
            MotionType::Initial => &mut self.initial,
            MotionType::JumpLeft => &mut self.jump_left,
            MotionType::JumpRight => &mut self.jump_right,
            MotionType::CenterJump => &mut self.center_jump,
            MotionType::FallProtection => &mut self.fall_protection,
            MotionType::Penalized => &mut self.penalized,
            MotionType::SitDown => &mut self.sit_down,
            MotionType::Stand => &mut self.stand,
            MotionType::StandUpBack => &mut self.stand_up_back,
            MotionType::StandUpFront => &mut self.stand_up_front,
            MotionType::StandUpSitting => &mut self.stand_up_sitting,
            MotionType::StandUpFrontSlow => &mut self.stand_up_front_slow,
            MotionType::StandUpSittingSlow => &mut self.stand_up_sitting_slow,
            MotionType::Unstiff => &mut self.unstiff,
            MotionType::Walk => &mut self.walk,
            MotionType::WideStance => &mut self.wide_stance,
            MotionType::KeeperJumpLeft => &mut self.keeper_jump_left,
            MotionType::KeeperJumpRight => &mut self.keeper_jump_right,
        }
    }
}

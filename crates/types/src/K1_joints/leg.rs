use std::{
    ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Neg, Sub},
    time::Duration,
};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use super::mirror::Mirror;

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Eq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum LegJoint {
    Ankle_Down,
    Ankle_Up,
    HipPitch,
    HipRoll,
    HipYaw,
    Knee,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    Eq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct LegJoints<T = f32> {
    pub ankle_down: T,
    pub ankle_up: T,
    pub hip_pitch: T,
    pub hip_roll: T,
    pub hip_yaw: T,
    pub knee: T,
}

impl<T> LegJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            hip_roll: value.clone(),
            hip_pitch: value.clone(),
            hip_yaw: value.clone(),
            knee: value.clone(),
            ankle_down: value,
            ankle_up: value.clone(),
        }
    }
}

impl LegJoints<f32> {
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            hip_roll: self.hip_roll.clamp(min.hip_roll, max.hip_roll),
            hip_pitch: self.hip_pitch.clamp(min.hip_pitch, max.hip_pitch),
            hip_yaw: self.hip_yaw.clamp(min.hip_yaw, max.hip_yaw),
            knee: self.knee.clamp(min.knee, max.knee),
            ankle_down: self.ankle_down.clamp(min.ankle_down, max.ankle_down),
            ankle_up: self.ankle_up.clamp(min.ankle_up, max.ankle_up),
        }
    }
}

impl<T> IntoIterator for LegJoints<T> {
    type Item = T;

    type IntoIter = std::array::IntoIter<T, 6>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.hip_roll,
            self.hip_pitch,
            self.hip_yaw,
            self.knee,
            self.ankle_down,
            self.ankle_up,
        ]
        .into_iter()
    }
}

impl<T> Add for LegJoints<T>
where
    T: Add,
{
    type Output = LegJoints<<T as Add>::Output>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            hip_roll: self.hip_roll + right.hip_roll,
            hip_pitch: self.hip_pitch + right.hip_pitch,
            hip_yaw: self.hip_yaw + right.hip_yaw,
            knee: self.knee + right.knee,
            ankle_down: self.ankle_down + right.ankle_down,
            ankle_up: self.ankle_up + right.ankle_up,
        }
    }
}

impl AddAssign for LegJoints<f32> {
    fn add_assign(&mut self, right: Self) {
        self.hip_roll += right.hip_roll;
        self.hip_pitch += right.hip_pitch;
        self.hip_yaw += right.hip_yaw;
        self.knee += right.knee;
        self.ankle_down += right.ankle_down;
        self.ankle_up += right.ankle_up;
    }
}

impl<T> Sub for LegJoints<T>
where
    T: Sub,
{
    type Output = LegJoints<<T as Sub>::Output>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            hip_roll: self.hip_roll - right.hip_roll,
            hip_pitch: self.hip_pitch - right.hip_pitch,
            hip_yaw: self.hip_yaw - right.hip_yaw,
            knee: self.knee - right.knee,
            ankle_down: self.ankle_down - right.ankle_down,
            ankle_up: self.ankle_up - right.ankle_up,
        }
    }
}

impl<T> Neg for LegJoints<T>
where
    T: Neg,
{
    type Output = LegJoints<<T as Neg>::Output>;

    fn neg(self) -> Self::Output {
        Self::Output {
            hip_roll: -self.hip_roll,
            hip_pitch: -self.hip_pitch,
            hip_yaw: -self.hip_yaw,
            knee: -self.knee,
            ankle_down: -self.ankle_down,
            ankle_up: -self.ankle_up,
        }
    }
}

impl Mul<f32> for LegJoints<f32> {
    type Output = LegJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            hip_roll: self.hip_roll * right,
            hip_pitch: self.hip_pitch * right,
            hip_yaw: self.hip_yaw * right,
            knee: self.knee * right,
            ankle_down: self.ankle_down * right,
            ankle_up: self.ankle_up * right,
        }
    }
}

impl Div<f32> for LegJoints<f32> {
    type Output = LegJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            hip_roll: self.hip_roll / right,
            hip_pitch: self.hip_pitch / right,
            hip_yaw: self.hip_yaw / right,
            knee: self.knee / right,
            ankle_down: self.ankle_down / right,
            ankle_up: self.ankle_up / right,
        }
    }
}

impl Div<LegJoints<f32>> for LegJoints<f32> {
    type Output = LegJoints<Duration>;

    fn div(self, right: LegJoints<f32>) -> Self::Output {
        Self::Output {
            hip_roll: Duration::from_secs_f32((self.hip_roll / right.hip_roll).abs()),
            hip_pitch: Duration::from_secs_f32((self.hip_pitch / right.hip_pitch).abs()),
            hip_yaw: Duration::from_secs_f32((self.hip_yaw / right.hip_yaw).abs()),
            knee: Duration::from_secs_f32((self.knee / right.knee).abs()),
            ankle_down: Duration::from_secs_f32((self.ankle_down / right.ankle_down).abs()),
            ankle_up: Duration::from_secs_f32((self.ankle_up / right.ankle_up).abs()),
        }
    }
}

impl Mirror for LegJoints<f32> {
    fn mirrored(self) -> Self {
        Self {
            hip_roll: -self.hip_roll,
            hip_pitch: self.hip_pitch,
            hip_yaw: self.hip_yaw,
            knee: self.knee,
            ankle_down: -self.ankle_down,
            ankle_up: self.ankle_up,
        }
    }
}

impl<T> Index<LegJoint> for LegJoints<T> {
    type Output = T;

    fn index(&self, index: LegJoint) -> &Self::Output {
        match index {
            LegJoint::Ankle_Down => &self.ankle_down,
            LegJoint::Ankle_Up => &self.ankle_up,
            LegJoint::HipPitch => &self.hip_pitch,
            LegJoint::HipRoll => &self.hip_roll,
            LegJoint::HipYaw => &self.hip_yaw,
            LegJoint::Knee => &self.knee,
        }
    }
}

impl<T> IndexMut<LegJoint> for LegJoints<T> {
    fn index_mut(&mut self, index: LegJoint) -> &mut Self::Output {
        match index {
            LegJoint::Ankle_Down => &mut self.ankle_down,
            LegJoint::Ankle_Up => &mut self.ankle_up,
            LegJoint::HipPitch => &mut self.hip_pitch,
            LegJoint::HipRoll => &mut self.hip_roll,
            LegJoint::HipYaw => &mut self.hip_yaw,
            LegJoint::Knee => &mut self.knee,
        }
    }
}

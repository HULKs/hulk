use std::{
    ops::{Add, Div, Index, IndexMut, Mul, Sub},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::mirror::Mirror;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy)]
pub enum LegJoint {
    AnklePitch,
    AnkleRoll,
    HipPitch,
    HipRoll,
    HipYawPitch,
    KneePitch,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct LegJoints<T> {
    pub ankle_pitch: T,
    pub ankle_roll: T,
    pub hip_pitch: T,
    pub hip_roll: T,
    pub hip_yaw_pitch: T,
    pub knee_pitch: T,
}

impl<T> LegJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            hip_yaw_pitch: value.clone(),
            hip_roll: value.clone(),
            hip_pitch: value.clone(),
            knee_pitch: value.clone(),
            ankle_pitch: value.clone(),
            ankle_roll: value,
        }
    }
}

impl<T> IntoIterator for LegJoints<T> {
    type Item = T;

    type IntoIter = std::array::IntoIter<T, 6>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.hip_yaw_pitch,
            self.hip_roll,
            self.hip_pitch,
            self.knee_pitch,
            self.ankle_pitch,
            self.ankle_roll,
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
            hip_yaw_pitch: self.hip_yaw_pitch + right.hip_yaw_pitch,
            hip_roll: self.hip_roll + right.hip_roll,
            hip_pitch: self.hip_pitch + right.hip_pitch,
            knee_pitch: self.knee_pitch + right.knee_pitch,
            ankle_pitch: self.ankle_pitch + right.ankle_pitch,
            ankle_roll: self.ankle_roll + right.ankle_roll,
        }
    }
}

impl<T> Sub for LegJoints<T>
where
    T: Sub,
{
    type Output = LegJoints<<T as Sub>::Output>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch - right.hip_yaw_pitch,
            hip_roll: self.hip_roll - right.hip_roll,
            hip_pitch: self.hip_pitch - right.hip_pitch,
            knee_pitch: self.knee_pitch - right.knee_pitch,
            ankle_pitch: self.ankle_pitch - right.ankle_pitch,
            ankle_roll: self.ankle_roll - right.ankle_roll,
        }
    }
}

impl Mul<f32> for LegJoints<f32> {
    type Output = LegJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch * right,
            hip_roll: self.hip_roll * right,
            hip_pitch: self.hip_pitch * right,
            knee_pitch: self.knee_pitch * right,
            ankle_pitch: self.ankle_pitch * right,
            ankle_roll: self.ankle_roll * right,
        }
    }
}

impl Div<f32> for LegJoints<f32> {
    type Output = LegJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch / right,
            hip_roll: self.hip_roll / right,
            hip_pitch: self.hip_pitch / right,
            knee_pitch: self.knee_pitch / right,
            ankle_pitch: self.ankle_pitch / right,
            ankle_roll: self.ankle_roll / right,
        }
    }
}

impl Div<LegJoints<f32>> for LegJoints<f32> {
    type Output = LegJoints<Duration>;

    fn div(self, right: LegJoints<f32>) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: Duration::from_secs_f32(
                (self.hip_yaw_pitch / right.hip_yaw_pitch).abs(),
            ),
            hip_roll: Duration::from_secs_f32((self.hip_roll / right.hip_roll).abs()),
            hip_pitch: Duration::from_secs_f32((self.hip_pitch / right.hip_pitch).abs()),
            knee_pitch: Duration::from_secs_f32((self.knee_pitch / right.knee_pitch).abs()),
            ankle_pitch: Duration::from_secs_f32((self.ankle_pitch / right.ankle_pitch).abs()),
            ankle_roll: Duration::from_secs_f32((self.ankle_roll / right.ankle_roll).abs()),
        }
    }
}

impl Mirror for LegJoints<f32> {
    fn mirrored(self) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch,
            hip_roll: -self.hip_roll,
            hip_pitch: self.hip_pitch,
            knee_pitch: self.knee_pitch,
            ankle_pitch: self.ankle_pitch,
            ankle_roll: -self.ankle_roll,
        }
    }
}
impl LegJoints<f32> {
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            hip_yaw_pitch: self
                .hip_yaw_pitch
                .clamp(min.hip_yaw_pitch, max.hip_yaw_pitch),
            hip_roll: self.hip_roll.clamp(min.hip_roll, max.hip_roll),
            hip_pitch: self.hip_pitch.clamp(min.hip_pitch, max.hip_pitch),
            knee_pitch: self.knee_pitch.clamp(min.knee_pitch, max.knee_pitch),
            ankle_pitch: self.ankle_pitch.clamp(min.ankle_pitch, max.ankle_pitch),
            ankle_roll: self.ankle_roll.clamp(min.ankle_roll, max.ankle_roll),
        }
    }
}

impl<T> Index<LegJoint> for LegJoints<T> {
    type Output = T;

    fn index(&self, index: LegJoint) -> &Self::Output {
        match index {
            LegJoint::AnklePitch => &self.ankle_pitch,
            LegJoint::AnkleRoll => &self.ankle_roll,
            LegJoint::HipPitch => &self.hip_pitch,
            LegJoint::HipRoll => &self.hip_roll,
            LegJoint::HipYawPitch => &self.hip_yaw_pitch,
            LegJoint::KneePitch => &self.knee_pitch,
        }
    }
}

impl<T> IndexMut<LegJoint> for LegJoints<T> {
    fn index_mut(&mut self, index: LegJoint) -> &mut Self::Output {
        match index {
            LegJoint::AnklePitch => &mut self.ankle_pitch,
            LegJoint::AnkleRoll => &mut self.ankle_roll,
            LegJoint::HipPitch => &mut self.hip_pitch,
            LegJoint::HipRoll => &mut self.hip_roll,
            LegJoint::HipYawPitch => &mut self.hip_yaw_pitch,
            LegJoint::KneePitch => &mut self.knee_pitch,
        }
    }
}

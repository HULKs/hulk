use std::{
    f32::consts::PI,
    ops::{Add, Div, Index, IndexMut, Mul, Sub},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

use super::mirror::Mirror;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy)]
pub enum ArmJoint {
    ShoulderPitch,
    ShoulderRoll,
    ElbowYaw,
    ElbowRoll,
    WristYaw,
    Hand,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct ArmJoints<T> {
    pub shoulder_pitch: T,
    pub shoulder_roll: T,
    pub elbow_yaw: T,
    pub elbow_roll: T,
    pub wrist_yaw: T,
    pub hand: T,
}

impl_Interpolate!(f32, ArmJoints<f32>, PI);

impl<T> ArmJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            shoulder_pitch: value.clone(),
            shoulder_roll: value.clone(),
            elbow_yaw: value.clone(),
            elbow_roll: value.clone(),
            wrist_yaw: value.clone(),
            hand: value,
        }
    }
}

impl<T> IntoIterator for ArmJoints<T> {
    type Item = T;

    type IntoIter = std::array::IntoIter<T, 6>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.shoulder_pitch,
            self.shoulder_roll,
            self.elbow_yaw,
            self.elbow_roll,
            self.wrist_yaw,
            self.hand,
        ]
        .into_iter()
    }
}

impl<T> Add for ArmJoints<T>
where
    T: Add,
{
    type Output = ArmJoints<<T as Add>::Output>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch + right.shoulder_pitch,
            shoulder_roll: self.shoulder_roll + right.shoulder_roll,
            elbow_yaw: self.elbow_yaw + right.elbow_yaw,
            elbow_roll: self.elbow_roll + right.elbow_roll,
            wrist_yaw: self.wrist_yaw + right.wrist_yaw,
            hand: self.hand + right.hand,
        }
    }
}

impl<T> Sub for ArmJoints<T>
where
    T: Sub,
{
    type Output = ArmJoints<<T as Sub>::Output>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch - right.shoulder_pitch,
            shoulder_roll: self.shoulder_roll - right.shoulder_roll,
            elbow_yaw: self.elbow_yaw - right.elbow_yaw,
            elbow_roll: self.elbow_roll - right.elbow_roll,
            wrist_yaw: self.wrist_yaw - right.wrist_yaw,
            hand: self.hand - right.hand,
        }
    }
}

impl Mul<f32> for ArmJoints<f32> {
    type Output = ArmJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch * right,
            shoulder_roll: self.shoulder_roll * right,
            elbow_yaw: self.elbow_yaw * right,
            elbow_roll: self.elbow_roll * right,
            wrist_yaw: self.wrist_yaw * right,
            hand: self.hand * right,
        }
    }
}

impl Div<f32> for ArmJoints<f32> {
    type Output = ArmJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch / right,
            shoulder_roll: self.shoulder_roll / right,
            elbow_yaw: self.elbow_yaw / right,
            elbow_roll: self.elbow_roll / right,
            wrist_yaw: self.wrist_yaw / right,
            hand: self.hand / right,
        }
    }
}

impl Div<ArmJoints<f32>> for ArmJoints<f32> {
    type Output = ArmJoints<Duration>;

    fn div(self, right: ArmJoints<f32>) -> Self::Output {
        Self::Output {
            shoulder_pitch: Duration::from_secs_f32(
                (self.shoulder_pitch / right.shoulder_pitch).abs(),
            ),
            shoulder_roll: Duration::from_secs_f32(
                (self.shoulder_roll / right.shoulder_roll).abs(),
            ),
            elbow_yaw: Duration::from_secs_f32((self.elbow_yaw / right.elbow_yaw).abs()),
            elbow_roll: Duration::from_secs_f32((self.elbow_roll / right.elbow_roll).abs()),
            wrist_yaw: Duration::from_secs_f32((self.wrist_yaw / right.wrist_yaw).abs()),
            hand: Duration::from_secs_f32((self.hand / right.hand).abs()),
        }
    }
}

impl Mirror for ArmJoints<f32> {
    fn mirrored(self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch,
            shoulder_roll: -self.shoulder_roll,
            elbow_yaw: -self.elbow_yaw,
            elbow_roll: -self.elbow_roll,
            wrist_yaw: -self.wrist_yaw,
            hand: self.hand,
        }
    }
}

impl<T> Index<ArmJoint> for ArmJoints<T> {
    type Output = T;

    fn index(&self, index: ArmJoint) -> &Self::Output {
        match index {
            ArmJoint::ShoulderPitch => &self.shoulder_pitch,
            ArmJoint::ShoulderRoll => &self.shoulder_roll,
            ArmJoint::ElbowYaw => &self.elbow_yaw,
            ArmJoint::ElbowRoll => &self.elbow_roll,
            ArmJoint::WristYaw => &self.wrist_yaw,
            ArmJoint::Hand => &self.hand,
        }
    }
}

impl<T> IndexMut<ArmJoint> for ArmJoints<T> {
    fn index_mut(&mut self, index: ArmJoint) -> &mut Self::Output {
        match index {
            ArmJoint::ShoulderPitch => &mut self.shoulder_pitch,
            ArmJoint::ShoulderRoll => &mut self.shoulder_roll,
            ArmJoint::ElbowYaw => &mut self.elbow_yaw,
            ArmJoint::ElbowRoll => &mut self.elbow_roll,
            ArmJoint::WristYaw => &mut self.wrist_yaw,
            ArmJoint::Hand => &mut self.hand,
        }
    }
}

use std::{
    f32::consts::PI,
    ops::{Add, Div, Index, IndexMut, Mul, Sub},
    time::Duration,
};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::impl_Interpolate;

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
pub enum ArmJoint {
    Elbow,
    ShoulderPitch,
    ShoulderRoll,
    ShoulderYaw,
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
pub struct ArmJoints<T = f32> {
    pub shoulder_pitch: T,
    pub shoulder_roll: T,
    pub shoulder_yaw: T,
    pub elbow: T,
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
            shoulder_yaw: value.clone(),
            elbow: value,
        }
    }
}

impl<T> IntoIterator for ArmJoints<T> {
    type Item = T;

    type IntoIter = std::array::IntoIter<T, 4>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.shoulder_pitch,
            self.shoulder_roll,
            self.shoulder_yaw,
            self.elbow,
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
            shoulder_yaw: self.shoulder_yaw + right.shoulder_yaw,
            elbow: self.elbow + right.elbow,
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
            shoulder_yaw: self.shoulder_yaw - right.shoulder_yaw,
            elbow: self.elbow - right.elbow,
        }
    }
}

impl Mul<f32> for ArmJoints<f32> {
    type Output = ArmJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch * right,
            shoulder_roll: self.shoulder_roll * right,
            shoulder_yaw: self.shoulder_yaw * right,
            elbow: self.elbow * right,
        }
    }
}

impl Div<f32> for ArmJoints<f32> {
    type Output = ArmJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch / right,
            shoulder_roll: self.shoulder_roll / right,
            shoulder_yaw: self.shoulder_yaw / right,
            elbow: self.elbow / right,
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
            shoulder_yaw: Duration::from_secs_f32((self.shoulder_yaw / right.shoulder_yaw).abs()),
            elbow: Duration::from_secs_f32((self.elbow / right.elbow).abs()),
        }
    }
}

impl Mirror for ArmJoints<f32> {
    fn mirrored(self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch,
            shoulder_roll: -self.shoulder_roll,
            shoulder_yaw: -self.shoulder_yaw,
            elbow: -self.elbow,
        }
    }
}

impl<T> Index<ArmJoint> for ArmJoints<T> {
    type Output = T;

    fn index(&self, index: ArmJoint) -> &Self::Output {
        match index {
            ArmJoint::ShoulderPitch => &self.shoulder_pitch,
            ArmJoint::ShoulderRoll => &self.shoulder_roll,
            ArmJoint::ShoulderYaw => &self.shoulder_yaw,
            ArmJoint::Elbow => &self.elbow,
        }
    }
}

impl<T> IndexMut<ArmJoint> for ArmJoints<T> {
    fn index_mut(&mut self, index: ArmJoint) -> &mut Self::Output {
        match index {
            ArmJoint::ShoulderPitch => &mut self.shoulder_pitch,
            ArmJoint::ShoulderRoll => &mut self.shoulder_roll,
            ArmJoint::ShoulderYaw => &mut self.shoulder_yaw,
            ArmJoint::Elbow => &mut self.elbow,
        }
    }
}

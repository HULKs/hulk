use std::{
    array::IntoIter,
    f32::consts::PI,
    iter::Chain,
    ops::{Add, Div, Mul, Sub},
};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::impl_Interpolate;

use super::{
    Joints,
    arm::{ArmJoint, ArmJoints},
    leg::{LegJoint, LegJoints},
};

pub enum BodyJointsName {
    LeftArm(ArmJoint),
    RightArm(ArmJoint),
    LeftLeg(LegJoint),
    RightLeg(LegJoint),
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
    ros_z::Message,
)]
pub struct BodyJoints<T = f32> {
    pub left_arm: ArmJoints<T>,
    pub right_arm: ArmJoints<T>,
    pub left_leg: LegJoints<T>,
    pub right_leg: LegJoints<T>,
}

impl<T> BodyJoints<T> {
    pub fn from_lower_and_upper(lower: LowerBodyJoints<T>, upper: UpperBodyJoints<T>) -> Self {
        Self {
            left_arm: upper.left_arm,
            right_arm: upper.right_arm,
            left_leg: lower.left_leg,
            right_leg: lower.right_leg,
        }
    }
}

impl<T> BodyJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            left_arm: ArmJoints::fill(value.clone()),
            right_arm: ArmJoints::fill(value.clone()),
            left_leg: LegJoints::fill(value.clone()),
            right_leg: LegJoints::fill(value),
        }
    }
}

impl<T> BodyJoints<T>
where
    T: Clone,
{
    pub fn fill_mirrored(arm: T, leg: T) -> Self {
        Self {
            left_arm: ArmJoints::fill(arm.clone()),
            right_arm: ArmJoints::fill(arm),
            left_leg: LegJoints::fill(leg.clone()),
            right_leg: LegJoints::fill(leg),
        }
    }
}

impl<T> From<Joints<T>> for BodyJoints<T> {
    fn from(joints: Joints<T>) -> Self {
        Self {
            left_arm: joints.left_arm,
            right_arm: joints.right_arm,
            left_leg: joints.left_leg,
            right_leg: joints.right_leg,
        }
    }
}

impl<T, O> Add for BodyJoints<T>
where
    ArmJoints<T>: Add<Output = ArmJoints<O>>,
    LegJoints<T>: Add<Output = LegJoints<O>>,
{
    type Output = BodyJoints<O>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm + right.left_arm,
            right_arm: self.right_arm + right.right_arm,
            left_leg: self.left_leg + right.left_leg,
            right_leg: self.right_leg + right.right_leg,
        }
    }
}

impl<T, O> Sub for BodyJoints<T>
where
    ArmJoints<T>: Sub<Output = ArmJoints<O>>,
    LegJoints<T>: Sub<Output = LegJoints<O>>,
{
    type Output = BodyJoints<O>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm - right.left_arm,
            right_arm: self.right_arm - right.right_arm,
            left_leg: self.left_leg - right.left_leg,
            right_leg: self.right_leg - right.right_leg,
        }
    }
}

impl Mul<f32> for BodyJoints<f32> {
    type Output = BodyJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm * right,
            right_arm: self.right_arm * right,
            left_leg: self.left_leg * right,
            right_leg: self.right_leg * right,
        }
    }
}

impl Div<f32> for BodyJoints<f32> {
    type Output = BodyJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm / right,
            right_arm: self.right_arm / right,
            left_leg: self.left_leg / right,
            right_leg: self.right_leg / right,
        }
    }
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
pub struct LowerBodyJoints<T = f32> {
    pub left_leg: LegJoints<T>,
    pub right_leg: LegJoints<T>,
}

impl<T> LowerBodyJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            left_leg: LegJoints::fill(value.clone()),
            right_leg: LegJoints::fill(value),
        }
    }
}

impl<T> From<BodyJoints<T>> for LowerBodyJoints<T> {
    fn from(joints: BodyJoints<T>) -> Self {
        Self {
            left_leg: joints.left_leg,
            right_leg: joints.right_leg,
        }
    }
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
    ros_z::Message,
)]
pub struct UpperBodyJoints<T = f32> {
    pub left_arm: ArmJoints<T>,
    pub right_arm: ArmJoints<T>,
}

impl_Interpolate!(f32, UpperBodyJoints<f32>, PI);

impl UpperBodyJoints<f32> {
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            left_arm: ArmJoints {
                shoulder_pitch: self
                    .left_arm
                    .shoulder_pitch
                    .clamp(min.left_arm.shoulder_pitch, max.left_arm.shoulder_pitch),
                shoulder_roll: self
                    .left_arm
                    .shoulder_roll
                    .clamp(min.left_arm.shoulder_roll, max.left_arm.shoulder_roll),
                shoulder_yaw: self
                    .left_arm
                    .shoulder_yaw
                    .clamp(min.left_arm.shoulder_yaw, max.left_arm.shoulder_yaw),
                elbow: self
                    .left_arm
                    .elbow
                    .clamp(min.left_arm.elbow, max.left_arm.elbow),
            },
            right_arm: ArmJoints {
                shoulder_pitch: self
                    .right_arm
                    .shoulder_pitch
                    .clamp(min.right_arm.shoulder_pitch, max.right_arm.shoulder_pitch),
                shoulder_roll: self
                    .right_arm
                    .shoulder_roll
                    .clamp(min.right_arm.shoulder_roll, max.right_arm.shoulder_roll),
                shoulder_yaw: self
                    .right_arm
                    .shoulder_yaw
                    .clamp(min.right_arm.shoulder_yaw, max.right_arm.shoulder_yaw),
                elbow: self
                    .right_arm
                    .elbow
                    .clamp(min.right_arm.elbow, max.right_arm.elbow),
            },
        }
    }
}

impl<T> UpperBodyJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            left_arm: ArmJoints::fill(value.clone()),
            right_arm: ArmJoints::fill(value),
        }
    }
}

impl<T> IntoIterator for UpperBodyJoints<T> {
    type Item = T;

    type IntoIter = Chain<IntoIter<T, 4>, IntoIter<T, 4>>;

    fn into_iter(self) -> Self::IntoIter {
        self.left_arm.into_iter().chain(self.right_arm)
    }
}

impl<T> From<Joints<T>> for UpperBodyJoints<T> {
    fn from(joints: Joints<T>) -> Self {
        Self {
            left_arm: joints.left_arm,
            right_arm: joints.right_arm,
        }
    }
}

impl<T> From<BodyJoints<T>> for UpperBodyJoints<T> {
    fn from(joints: BodyJoints<T>) -> Self {
        Self {
            left_arm: joints.left_arm,
            right_arm: joints.right_arm,
        }
    }
}

impl From<[f32; 8]> for UpperBodyJoints<f32> {
    fn from(values: [f32; 8]) -> Self {
        Self {
            left_arm: ArmJoints {
                shoulder_pitch: values[0],
                shoulder_roll: values[1],
                shoulder_yaw: values[2],
                elbow: values[3],
            },
            right_arm: ArmJoints {
                shoulder_pitch: values[4],
                shoulder_roll: values[5],
                shoulder_yaw: values[6],
                elbow: values[7],
            },
        }
    }
}

impl<T, O> Add for UpperBodyJoints<T>
where
    ArmJoints<T>: Add<Output = ArmJoints<O>>,
{
    type Output = UpperBodyJoints<O>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm + right.left_arm,
            right_arm: self.right_arm + right.right_arm,
        }
    }
}

impl<T, O> Sub for UpperBodyJoints<T>
where
    ArmJoints<T>: Sub<Output = ArmJoints<O>>,
{
    type Output = UpperBodyJoints<O>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm - right.left_arm,
            right_arm: self.right_arm - right.right_arm,
        }
    }
}

impl Mul<f32> for UpperBodyJoints<f32> {
    type Output = UpperBodyJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm * right,
            right_arm: self.right_arm * right,
        }
    }
}

impl Div<f32> for UpperBodyJoints<f32> {
    type Output = UpperBodyJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm / right,
            right_arm: self.right_arm / right,
        }
    }
}

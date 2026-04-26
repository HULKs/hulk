use std::{
    array::IntoIter,
    iter::Chain,
    ops::{Add, Div, Mul, Sub},
};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

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

impl<T> BodyJoints<T>
where
    T: Default,
{
    pub fn to_booster_deploy_joint_array(self) -> [T; 21] {
        [
            self.left_arm.shoulder_pitch,
            self.right_arm.shoulder_pitch,
            T::default(),
            self.left_arm.shoulder_roll,
            self.right_arm.shoulder_roll,
            self.left_leg.hip_pitch,
            self.right_leg.hip_pitch,
            self.left_arm.elbow,
            self.right_arm.elbow,
            self.left_leg.hip_roll,
            self.right_leg.hip_roll,
            self.left_arm.shoulder_yaw,
            self.right_arm.shoulder_yaw,
            self.left_leg.hip_yaw,
            self.right_leg.hip_yaw,
            self.left_leg.knee,
            self.right_leg.knee,
            self.left_leg.ankle_up,
            self.right_leg.ankle_up,
            self.left_leg.ankle_down,
            self.right_leg.ankle_down,
        ]
    }

    pub fn from_booster_deploy_joint_array(joint_vector: [T; 21]) -> Self {
        #[rustfmt::skip]
        let [
                left_shoulder_pitch, 
                right_shoulder_pitch, 
                _, 
                left_shoulder_roll, 
                right_shoulder_roll, 
                left_hip_pitch, 
                right_hip_pitch, 
                left_elbow, 
                right_elbow, 
                left_hip_roll, 
                right_hip_roll, 
                left_shoulder_yaw, 
                right_shoulder_yaw, 
                left_hip_yaw, 
                right_hip_yaw, 
                left_knee, 
                right_knee, 
                left_ankle_up, 
                right_ankle_up, 
                left_ankle_down, 
                right_ankle_down
            ] = joint_vector;

        BodyJoints {
            left_arm: ArmJoints {
                shoulder_pitch: left_shoulder_pitch,
                shoulder_roll: left_shoulder_roll,
                shoulder_yaw: left_shoulder_yaw,
                elbow: left_elbow,
            },
            right_arm: ArmJoints {
                shoulder_pitch: right_shoulder_pitch,
                shoulder_roll: right_shoulder_roll,
                shoulder_yaw: right_shoulder_yaw,
                elbow: right_elbow,
            },
            left_leg: LegJoints {
                hip_pitch: left_hip_pitch,
                hip_roll: left_hip_roll,
                hip_yaw: left_hip_yaw,
                knee: left_knee,
                ankle_up: left_ankle_up,
                ankle_down: left_ankle_down,
            },
            right_leg: LegJoints {
                hip_pitch: right_hip_pitch,
                hip_roll: right_hip_roll,
                hip_yaw: right_hip_yaw,
                knee: right_knee,
                ankle_up: right_ankle_up,
                ankle_down: right_ankle_down,
            },
        }
    }
}

impl<T> IntoIterator for BodyJoints<T> {
    type Item = T;

    type IntoIter =
        Chain<Chain<Chain<IntoIter<T, 4>, IntoIter<T, 4>>, IntoIter<T, 6>>, IntoIter<T, 6>>;

    fn into_iter(self) -> Self::IntoIter {
        self.left_arm
            .into_iter()
            .chain(self.right_arm)
            .chain(self.left_leg)
            .chain(self.right_leg)
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

impl<T> IntoIterator for LowerBodyJoints<T> {
    type Item = T;

    type IntoIter = Chain<IntoIter<T, 6>, IntoIter<T, 6>>;

    fn into_iter(self) -> Self::IntoIter {
        self.left_leg.into_iter().chain(self.right_leg)
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
pub struct UpperBodyJoints<T> {
    pub left_arm: ArmJoints<T>,
    pub right_arm: ArmJoints<T>,
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

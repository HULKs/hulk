pub mod arm;
pub mod body;
pub mod head;
pub mod leg;

use std::{
    array::IntoIter,
    f32::consts::PI,
    iter::{Chain, Sum},
    ops::{Add, Div, Index, IndexMut, Mul, Sub},
};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

use self::{
    arm::{ArmJoints, ArmJointsName},
    body::BodyJoints,
    head::{HeadJoints, HeadJointsName},
    leg::{LegJoints, LegJointsName},
};

#[derive(Clone, Copy)]
pub enum JointsName {
    Head(HeadJointsName),
    LeftArm(ArmJointsName),
    RightArm(ArmJointsName),
    LeftLeg(LegJointsName),
    RightLeg(LegJointsName),
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct Joints<T> {
    pub head: HeadJoints<T>,
    pub left_arm: ArmJoints<T>,
    pub right_arm: ArmJoints<T>,
    pub left_leg: LegJoints<T>,
    pub right_leg: LegJoints<T>,
}

impl<T> Joints<T> {
    pub fn from_head_and_body(head: HeadJoints<T>, body: BodyJoints<T>) -> Self {
        Self {
            head,
            left_arm: body.left_arm,
            right_arm: body.right_arm,
            left_leg: body.left_leg,
            right_leg: body.right_leg,
        }
    }

    pub fn enumerate(self) -> <Joints<(JointsName, T)> as IntoIterator>::IntoIter {
        Joints {
            head: HeadJoints {
                yaw: (JointsName::Head(HeadJointsName::Yaw), self.head.yaw),
                pitch: (JointsName::Head(HeadJointsName::Pitch), self.head.pitch),
            },
            left_arm: ArmJoints {
                shoulder_pitch: (
                    JointsName::LeftArm(ArmJointsName::ShoulderPitch),
                    self.left_arm.shoulder_pitch,
                ),
                shoulder_roll: (
                    JointsName::LeftArm(ArmJointsName::ShoulderRoll),
                    self.left_arm.shoulder_roll,
                ),
                elbow_yaw: (
                    JointsName::LeftArm(ArmJointsName::ElbowYaw),
                    self.left_arm.elbow_yaw,
                ),
                elbow_roll: (
                    JointsName::LeftArm(ArmJointsName::ElbowRoll),
                    self.left_arm.elbow_roll,
                ),
                wrist_yaw: (
                    JointsName::LeftArm(ArmJointsName::WristYaw),
                    self.left_arm.wrist_yaw,
                ),
                hand: (JointsName::LeftArm(ArmJointsName::Hand), self.left_arm.hand),
            },
            right_arm: ArmJoints {
                shoulder_pitch: (
                    JointsName::RightArm(ArmJointsName::ShoulderPitch),
                    self.right_arm.shoulder_pitch,
                ),
                shoulder_roll: (
                    JointsName::RightArm(ArmJointsName::ShoulderRoll),
                    self.right_arm.shoulder_roll,
                ),
                elbow_yaw: (
                    JointsName::RightArm(ArmJointsName::ElbowYaw),
                    self.right_arm.elbow_yaw,
                ),
                elbow_roll: (
                    JointsName::RightArm(ArmJointsName::ElbowRoll),
                    self.right_arm.elbow_roll,
                ),
                wrist_yaw: (
                    JointsName::RightArm(ArmJointsName::WristYaw),
                    self.right_arm.wrist_yaw,
                ),
                hand: (
                    JointsName::RightArm(ArmJointsName::Hand),
                    self.right_arm.hand,
                ),
            },
            left_leg: LegJoints {
                ankle_pitch: (
                    JointsName::LeftLeg(LegJointsName::AnklePitch),
                    self.left_leg.ankle_pitch,
                ),
                ankle_roll: (
                    JointsName::LeftLeg(LegJointsName::AnkleRoll),
                    self.left_leg.ankle_roll,
                ),
                hip_pitch: (
                    JointsName::LeftLeg(LegJointsName::HipPitch),
                    self.left_leg.hip_pitch,
                ),
                hip_roll: (
                    JointsName::LeftLeg(LegJointsName::HipRoll),
                    self.left_leg.hip_roll,
                ),
                hip_yaw_pitch: (
                    JointsName::LeftLeg(LegJointsName::HipYawPitch),
                    self.left_leg.hip_yaw_pitch,
                ),
                knee_pitch: (
                    JointsName::LeftLeg(LegJointsName::KneePitch),
                    self.left_leg.knee_pitch,
                ),
            },
            right_leg: LegJoints {
                ankle_pitch: (
                    JointsName::RightLeg(LegJointsName::AnklePitch),
                    self.right_leg.ankle_pitch,
                ),
                ankle_roll: (
                    JointsName::RightLeg(LegJointsName::AnkleRoll),
                    self.right_leg.ankle_roll,
                ),
                hip_pitch: (
                    JointsName::RightLeg(LegJointsName::HipPitch),
                    self.right_leg.hip_pitch,
                ),
                hip_roll: (
                    JointsName::RightLeg(LegJointsName::HipRoll),
                    self.right_leg.hip_roll,
                ),
                hip_yaw_pitch: (
                    JointsName::RightLeg(LegJointsName::HipYawPitch),
                    self.right_leg.hip_yaw_pitch,
                ),
                knee_pitch: (
                    JointsName::RightLeg(LegJointsName::KneePitch),
                    self.right_leg.knee_pitch,
                ),
            },
        }
        .into_iter()
    }
}

impl<T> Index<JointsName> for Joints<T> {
    type Output = T;

    fn index(&self, index: JointsName) -> &Self::Output {
        match index {
            JointsName::Head(index) => &self.head[index],
            JointsName::LeftArm(index) => &self.left_arm[index],
            JointsName::RightArm(index) => &self.right_arm[index],
            JointsName::LeftLeg(index) => &self.left_leg[index],
            JointsName::RightLeg(index) => &self.right_leg[index],
        }
    }
}

impl<T> IndexMut<JointsName> for Joints<T> {
    fn index_mut(&mut self, index: JointsName) -> &mut Self::Output {
        match index {
            JointsName::Head(index) => &mut self.head[index],
            JointsName::LeftArm(index) => &mut self.left_arm[index],
            JointsName::RightArm(index) => &mut self.right_arm[index],
            JointsName::LeftLeg(index) => &mut self.left_leg[index],
            JointsName::RightLeg(index) => &mut self.right_leg[index],
        }
    }
}

impl<T> Joints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            head: HeadJoints::fill(value.clone()),
            left_arm: ArmJoints::fill(value.clone()),
            right_arm: ArmJoints::fill(value.clone()),
            left_leg: LegJoints::fill(value.clone()),
            right_leg: LegJoints::fill(value),
        }
    }
}

impl<T> IntoIterator for Joints<T> {
    type Item = T;

    type IntoIter = Chain<
        Chain<Chain<Chain<IntoIter<T, 2>, IntoIter<T, 6>>, IntoIter<T, 6>>, IntoIter<T, 6>>,
        IntoIter<T, 6>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.head
            .into_iter()
            .chain(self.left_arm)
            .chain(self.right_arm)
            .chain(self.left_leg)
            .chain(self.right_leg)
    }
}

impl<T, O> Add for Joints<T>
where
    HeadJoints<T>: Add<Output = HeadJoints<O>>,
    ArmJoints<T>: Add<Output = ArmJoints<O>>,
    LegJoints<T>: Add<Output = LegJoints<O>>,
{
    type Output = Joints<O>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            head: self.head + right.head,
            left_arm: self.left_arm + right.left_arm,
            right_arm: self.right_arm + right.right_arm,
            left_leg: self.left_leg + right.left_leg,
            right_leg: self.right_leg + right.right_leg,
        }
    }
}

impl<T, O> Sub for Joints<T>
where
    HeadJoints<T>: Sub<Output = HeadJoints<O>>,
    ArmJoints<T>: Sub<Output = ArmJoints<O>>,
    LegJoints<T>: Sub<Output = LegJoints<O>>,
{
    type Output = Joints<O>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            head: self.head - right.head,
            left_arm: self.left_arm - right.left_arm,
            right_arm: self.right_arm - right.right_arm,
            left_leg: self.left_leg - right.left_leg,
            right_leg: self.right_leg - right.right_leg,
        }
    }
}

impl<T, O> Div for Joints<T>
where
    HeadJoints<T>: Div<Output = HeadJoints<O>>,
    ArmJoints<T>: Div<Output = ArmJoints<O>>,
    LegJoints<T>: Div<Output = LegJoints<O>>,
{
    type Output = Joints<O>;

    fn div(self, right: Self) -> Self::Output {
        Self::Output {
            head: self.head / right.head,
            left_arm: self.left_arm / right.left_arm,
            right_arm: self.right_arm / right.right_arm,
            left_leg: self.left_leg / right.left_leg,
            right_leg: self.right_leg / right.right_leg,
        }
    }
}

impl<I, O> Sum<Joints<I>> for Joints<O>
where
    Joints<O>: Add<Joints<I>, Output = Joints<O>> + Default,
{
    fn sum<It>(iter: It) -> Self
    where
        It: Iterator<Item = Joints<I>>,
    {
        iter.fold(Joints::default(), |acc, x| acc + x)
    }
}

impl Mul<f32> for Joints<f32> {
    type Output = Joints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            head: self.head * right,
            left_arm: self.left_arm * right,
            right_arm: self.right_arm * right,
            left_leg: self.left_leg * right,
            right_leg: self.right_leg * right,
        }
    }
}

impl Div<f32> for Joints<f32> {
    type Output = Joints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            head: self.head / right,
            left_arm: self.left_arm / right,
            right_arm: self.right_arm / right,
            left_leg: self.left_leg / right,
            right_leg: self.right_leg / right,
        }
    }
}

impl_Interpolate!(f32, Joints<f32>, PI);

impl Joints<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            head: self.head.mirrored(),
            left_arm: self.right_arm.mirrored(),
            right_arm: self.left_arm.mirrored(),
            left_leg: self.right_leg.mirrored(),
            right_leg: self.left_leg.mirrored(),
        }
    }
}

impl<T> From<Joints<T>> for HeadJoints<T> {
    fn from(joints: Joints<T>) -> Self {
        Self {
            yaw: joints.head.yaw,
            pitch: joints.head.pitch,
        }
    }
}

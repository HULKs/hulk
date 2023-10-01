pub mod arm;
pub mod body;
pub mod head;
pub mod leg;

use std::{
    array::IntoIter,
    f32::consts::PI,
    iter::{Chain, Sum},
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

use self::{arm::ArmJoints, body::BodyJoints, head::HeadJoints, leg::LegJoints};

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

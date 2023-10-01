use std::ops::{Add, Div, Mul, Sub};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::{arm::ArmJoints, leg::LegJoints, Joints};

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct BodyJoints<T> {
    pub left_arm: ArmJoints<T>,
    pub right_arm: ArmJoints<T>,
    pub left_leg: LegJoints<T>,
    pub right_leg: LegJoints<T>,
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

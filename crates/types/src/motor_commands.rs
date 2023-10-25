use std::{
    f32::consts::PI,
    ops::{Add, Div, Mul, Sub},
};

use crate::joints::{body::BodyJoints, head::HeadJoints, Joints};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct MotorCommands<T> {
    pub positions: Joints<T>,
    pub stiffnesses: Joints<T>,
}

impl_Interpolate!(f32, MotorCommands<f32>, PI);

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct HeadMotorCommands<T> {
    pub positions: HeadJoints<T>,
    pub stiffnesses: HeadJoints<T>,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct BodyMotorCommands<T> {
    pub positions: BodyJoints<T>,
    pub stiffnesses: BodyJoints<T>,
}

impl MotorCommands<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            positions: Joints::mirrored(self.positions),
            stiffnesses: Joints::mirrored(self.stiffnesses),
        }
    }
}

impl Mul<f32> for MotorCommands<f32> {
    type Output = MotorCommands<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            positions: Joints::mul(self.positions, right),
            stiffnesses: self.stiffnesses * right,
        }
    }
}

impl Add<MotorCommands<f32>> for MotorCommands<f32> {
    type Output = MotorCommands<f32>;

    fn add(self, right: MotorCommands<f32>) -> Self::Output {
        Self::Output {
            positions: self.positions + right.positions,
            stiffnesses: self.stiffnesses + right.stiffnesses,
        }
    }
}

impl Sub<MotorCommands<f32>> for MotorCommands<f32> {
    type Output = MotorCommands<f32>;

    fn sub(self, right: MotorCommands<f32>) -> Self::Output {
        Self::Output {
            positions: self.positions - right.positions,
            stiffnesses: self.stiffnesses - right.stiffnesses,
        }
    }
}

impl Div<f32> for MotorCommands<f32> {
    type Output = MotorCommands<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            positions: self.positions / right,
            stiffnesses: self.stiffnesses / right,
        }
    }
}

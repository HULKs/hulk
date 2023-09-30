use std::{
    f32::consts::PI,
    ops::{Add, Div, Mul, Sub},
};

use crate::joints::{BodyJoints, HeadJoints, Joints};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct MotorCommand<T> {
    pub positions: Joints<T>,
    pub stiffnesses: Joints<T>,
}

impl_Interpolate!(f32, MotorCommand<f32>, PI);

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct HeadMotorCommand<T> {
    pub positions: HeadJoints<T>,
    pub stiffnesses: HeadJoints<T>,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct BodyMotorCommand<T> {
    pub positions: BodyJoints<T>,
    pub stiffnesses: BodyJoints<T>,
}

impl MotorCommand<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            positions: Joints::mirrored(self.positions),
            stiffnesses: Joints::mirrored(self.stiffnesses),
        }
    }
}

impl Mul<f32> for MotorCommand<f32> {
    type Output = MotorCommand<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            positions: Joints::mul(self.positions, right),
            stiffnesses: self.stiffnesses * right,
        }
    }
}

impl Add<MotorCommand<f32>> for MotorCommand<f32> {
    type Output = MotorCommand<f32>;

    fn add(self, right: MotorCommand<f32>) -> Self::Output {
        Self::Output {
            positions: self.positions + right.positions,
            stiffnesses: self.stiffnesses + right.stiffnesses,
        }
    }
}

impl Sub<MotorCommand<f32>> for MotorCommand<f32> {
    type Output = MotorCommand<f32>;

    fn sub(self, right: MotorCommand<f32>) -> Self::Output {
        Self::Output {
            positions: self.positions - right.positions,
            stiffnesses: self.stiffnesses - right.stiffnesses,
        }
    }
}

impl Div<f32> for MotorCommand<f32> {
    type Output = MotorCommand<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            positions: self.positions / right,
            stiffnesses: self.stiffnesses / right,
        }
    }
}

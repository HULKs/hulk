use std::{
    f32::consts::PI,
    ops::{Add, Div, Mul, Sub},
};

use crate::joints::{
    body::{BodyJoints, LowerBodyJoints, UpperBodyJoints},
    mirror::Mirror,
    Joints,
};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::impl_Interpolate;

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
pub struct MotorCommands<T> {
    pub positions: T,
    pub stiffnesses: T,
}

impl_Interpolate!(f32, MotorCommands<Joints<f32>>, PI);

impl<T> Mirror for MotorCommands<T>
where
    T: Mirror,
{
    fn mirrored(self) -> Self {
        Self {
            positions: T::mirrored(self.positions),
            stiffnesses: T::mirrored(self.stiffnesses),
        }
    }
}

impl<T> Mul<f32> for MotorCommands<T>
where
    T: Mul<f32>,
{
    type Output = MotorCommands<T::Output>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            positions: T::mul(self.positions, right),
            stiffnesses: self.stiffnesses * right,
        }
    }
}

impl<T> Add<MotorCommands<T>> for MotorCommands<T>
where
    T: Add<T>,
{
    type Output = MotorCommands<T::Output>;

    fn add(self, right: MotorCommands<T>) -> Self::Output {
        Self::Output {
            positions: self.positions + right.positions,
            stiffnesses: self.stiffnesses + right.stiffnesses,
        }
    }
}

impl<T> Sub<MotorCommands<T>> for MotorCommands<T>
where
    T: Sub<T>,
{
    type Output = MotorCommands<T::Output>;

    fn sub(self, right: MotorCommands<T>) -> Self::Output {
        Self::Output {
            positions: self.positions - right.positions,
            stiffnesses: self.stiffnesses - right.stiffnesses,
        }
    }
}

impl<T> Div<f32> for MotorCommands<T>
where
    T: Div<f32>,
{
    type Output = MotorCommands<T::Output>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            positions: self.positions / right,
            stiffnesses: self.stiffnesses / right,
        }
    }
}

impl MotorCommands<BodyJoints<f32>> {
    pub fn from_lower_and_upper(
        lower: MotorCommands<LowerBodyJoints<f32>>,
        upper: MotorCommands<UpperBodyJoints<f32>>,
    ) -> Self {
        Self {
            positions: BodyJoints::from_lower_and_upper(lower.positions, upper.positions),
            stiffnesses: BodyJoints::from_lower_and_upper(lower.stiffnesses, upper.stiffnesses),
        }
    }
}

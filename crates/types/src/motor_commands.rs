use std::{
    f32::consts::PI,
    ops::{Add, Div, Mul, Sub},
};

use crate::joints::{
    body::{BodyJoints, LowerBodyJoints, UpperBodyJoints},
    mirror::Mirror,
    Joints,
};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(
    bound = "Joints: SerializeHierarchy + Serialize, for<'de> Joints: Deserialize<'de>"
)]
pub struct MotorCommands<Joints> {
    pub positions: Joints,
    pub stiffnesses: Joints,
}

impl_Interpolate!(f32, MotorCommands<Joints<f32>>, PI);

impl<Joints> Mirror for MotorCommands<Joints>
where
    Joints: Mirror,
{
    fn mirrored(self) -> Self {
        Self {
            positions: Joints::mirrored(self.positions),
            stiffnesses: Joints::mirrored(self.stiffnesses),
        }
    }
}

impl<Joints> Mul<f32> for MotorCommands<Joints>
where
    Joints: Mul<f32>,
{
    type Output = MotorCommands<Joints::Output>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            positions: Joints::mul(self.positions, right),
            stiffnesses: self.stiffnesses * right,
        }
    }
}

impl<Joints> Add<MotorCommands<Joints>> for MotorCommands<Joints>
where
    Joints: Add<Joints>,
{
    type Output = MotorCommands<Joints::Output>;

    fn add(self, right: MotorCommands<Joints>) -> Self::Output {
        Self::Output {
            positions: self.positions + right.positions,
            stiffnesses: self.stiffnesses + right.stiffnesses,
        }
    }
}

impl<Joints> Sub<MotorCommands<Joints>> for MotorCommands<Joints>
where
    Joints: Sub<Joints>,
{
    type Output = MotorCommands<Joints::Output>;

    fn sub(self, right: MotorCommands<Joints>) -> Self::Output {
        Self::Output {
            positions: self.positions - right.positions,
            stiffnesses: self.stiffnesses - right.stiffnesses,
        }
    }
}

impl<Joints> Div<f32> for MotorCommands<Joints>
where
    Joints: Div<f32>,
{
    type Output = MotorCommands<Joints::Output>;

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

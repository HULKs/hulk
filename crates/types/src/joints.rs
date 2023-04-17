use std::{
    f32::consts::PI,
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct HeadJoints<T = f32> {
    pub yaw: T,
    pub pitch: T,
}

impl<T> HeadJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            yaw: value.clone(),
            pitch: value,
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

impl<T> Add for HeadJoints<T>
where
    T: Add,
{
    type Output = HeadJoints<<T as Add>::Output>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw + right.yaw,
            pitch: self.pitch + right.pitch,
        }
    }
}

impl<T> Sub for HeadJoints<T>
where
    T: Sub,
{
    type Output = HeadJoints<<T as Sub>::Output>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw - right.yaw,
            pitch: self.pitch - right.pitch,
        }
    }
}

impl Mul<f32> for HeadJoints<f32> {
    type Output = HeadJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            yaw: self.yaw * right,
            pitch: self.pitch * right,
        }
    }
}

impl Div<f32> for HeadJoints<f32> {
    type Output = HeadJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            yaw: self.yaw / right,
            pitch: self.pitch / right,
        }
    }
}

impl HeadJoints<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            yaw: -self.yaw,
            pitch: self.pitch,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct ArmJoints<T = f32> {
    pub shoulder_pitch: T,
    pub shoulder_roll: T,
    pub elbow_yaw: T,
    pub elbow_roll: T,
    pub wrist_yaw: T,
    pub hand: T,
}

impl<T> ArmJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            shoulder_pitch: value.clone(),
            shoulder_roll: value.clone(),
            elbow_yaw: value.clone(),
            elbow_roll: value.clone(),
            wrist_yaw: value.clone(),
            hand: value,
        }
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
            elbow_yaw: self.elbow_yaw + right.elbow_yaw,
            elbow_roll: self.elbow_roll + right.elbow_roll,
            wrist_yaw: self.wrist_yaw + right.wrist_yaw,
            hand: self.hand + right.hand,
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
            elbow_yaw: self.elbow_yaw - right.elbow_yaw,
            elbow_roll: self.elbow_roll - right.elbow_roll,
            wrist_yaw: self.wrist_yaw - right.wrist_yaw,
            hand: self.hand - right.hand,
        }
    }
}

impl Mul<f32> for ArmJoints<f32> {
    type Output = ArmJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch * right,
            shoulder_roll: self.shoulder_roll * right,
            elbow_yaw: self.elbow_yaw * right,
            elbow_roll: self.elbow_roll * right,
            wrist_yaw: self.wrist_yaw * right,
            hand: self.hand * right,
        }
    }
}

impl Div<f32> for ArmJoints<f32> {
    type Output = ArmJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch / right,
            shoulder_roll: self.shoulder_roll / right,
            elbow_yaw: self.elbow_yaw / right,
            elbow_roll: self.elbow_roll / right,
            wrist_yaw: self.wrist_yaw / right,
            hand: self.hand / right,
        }
    }
}

impl ArmJoints<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch,
            shoulder_roll: -self.shoulder_roll,
            elbow_yaw: -self.elbow_yaw,
            elbow_roll: -self.elbow_roll,
            wrist_yaw: -self.wrist_yaw,
            hand: self.hand,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct LegJoints<T = f32> {
    pub hip_yaw_pitch: T,
    pub hip_roll: T,
    pub hip_pitch: T,
    pub knee_pitch: T,
    pub ankle_pitch: T,
    pub ankle_roll: T,
}

impl<T> LegJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            hip_yaw_pitch: value.clone(),
            hip_roll: value.clone(),
            hip_pitch: value.clone(),
            knee_pitch: value.clone(),
            ankle_pitch: value.clone(),
            ankle_roll: value,
        }
    }
}

impl<T> Add for LegJoints<T>
where
    T: Add,
{
    type Output = LegJoints<<T as Add>::Output>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch + right.hip_yaw_pitch,
            hip_roll: self.hip_roll + right.hip_roll,
            hip_pitch: self.hip_pitch + right.hip_pitch,
            knee_pitch: self.knee_pitch + right.knee_pitch,
            ankle_pitch: self.ankle_pitch + right.ankle_pitch,
            ankle_roll: self.ankle_roll + right.ankle_roll,
        }
    }
}

impl<T> Sub for LegJoints<T>
where
    T: Sub,
{
    type Output = LegJoints<<T as Sub>::Output>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch - right.hip_yaw_pitch,
            hip_roll: self.hip_roll - right.hip_roll,
            hip_pitch: self.hip_pitch - right.hip_pitch,
            knee_pitch: self.knee_pitch - right.knee_pitch,
            ankle_pitch: self.ankle_pitch - right.ankle_pitch,
            ankle_roll: self.ankle_roll - right.ankle_roll,
        }
    }
}

impl Mul<f32> for LegJoints<f32> {
    type Output = LegJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch * right,
            hip_roll: self.hip_roll * right,
            hip_pitch: self.hip_pitch * right,
            knee_pitch: self.knee_pitch * right,
            ankle_pitch: self.ankle_pitch * right,
            ankle_roll: self.ankle_roll * right,
        }
    }
}

impl Div<f32> for LegJoints<f32> {
    type Output = LegJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch / right,
            hip_roll: self.hip_roll / right,
            hip_pitch: self.hip_pitch / right,
            knee_pitch: self.knee_pitch / right,
            ankle_pitch: self.ankle_pitch / right,
            ankle_roll: self.ankle_roll / right,
        }
    }
}

impl LegJoints<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch,
            hip_roll: -self.hip_roll,
            hip_pitch: self.hip_pitch,
            knee_pitch: self.knee_pitch,
            ankle_pitch: self.ankle_pitch,
            ankle_roll: -self.ankle_roll,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct BodyJoints<T = f32> {
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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct Joints<T = f32> {
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

    pub fn from_angles(angles: [f32; 26]) -> Self {
        Self {
            head: HeadJoints {
                yaw: angles[0],
                pitch: angles[1],
            },
            left_arm: ArmJoints {
                shoulder_pitch: angles[2],
                shoulder_roll: angles[3],
                elbow_yaw: angles[4],
                elbow_roll: angles[5],
                wrist_yaw: angles[6],
                hand: angles[7],
            },
            right_arm: ArmJoints {
                shoulder_pitch: angles[14],
                shoulder_roll: angles[15],
                elbow_yaw: angles[16],
                elbow_roll: angles[17],
                wrist_yaw: angles[18],
                hand: angles[19],
            },
            left_leg: LegJoints {
                hip_yaw_pitch: angles[8],
                hip_roll: angles[9],
                hip_pitch: angles[10],
                knee_pitch: angles[11],
                ankle_pitch: angles[12],
                ankle_roll: angles[13],
            },
            right_leg: LegJoints {
                hip_yaw_pitch: angles[20],
                hip_roll: angles[21],
                hip_pitch: angles[22],
                knee_pitch: angles[23],
                ankle_pitch: angles[24],
                ankle_roll: angles[25],
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct JointsCommand<T = f32> {
    pub positions: Joints<T>,
    pub stiffnesses: Joints<T>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct HeadJointsCommand<T = f32> {
    pub positions: HeadJoints<T>,
    pub stiffnesses: HeadJoints<T>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct BodyJointsCommand<T = f32> {
    pub positions: BodyJoints<T>,
    pub stiffnesses: BodyJoints<T>,
}

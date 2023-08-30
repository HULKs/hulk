use std::{
    f32::consts::PI,
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::impl_Interpolate;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct HeadJoints<T> {
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

    pub fn as_vec(&self) -> Vec<T> {
        vec![self.yaw.clone(), self.pitch.clone()]
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

impl Div<HeadJoints<f32>> for HeadJoints<f32> {
    type Output = HeadJoints<Duration>;

    fn div(self, right: HeadJoints<f32>) -> Self::Output {
        Self::Output {
            yaw: Duration::from_secs_f32((self.yaw / right.yaw).abs()),
            pitch: Duration::from_secs_f32((self.pitch / right.pitch).abs()),
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

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct ArmJoints<T> {
    pub shoulder_pitch: T,
    pub shoulder_roll: T,
    pub elbow_yaw: T,
    pub elbow_roll: T,
    pub wrist_yaw: T,
    pub hand: T,
}

impl_Interpolate!(f32, ArmJoints<f32>, PI);

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

    pub fn as_vec(&self) -> Vec<T> {
        vec![
            self.shoulder_pitch.clone(),
            self.shoulder_roll.clone(),
            self.elbow_yaw.clone(),
            self.elbow_roll.clone(),
            self.wrist_yaw.clone(),
            self.hand.clone(),
        ]
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

impl Div<ArmJoints<f32>> for ArmJoints<f32> {
    type Output = ArmJoints<Duration>;

    fn div(self, right: ArmJoints<f32>) -> Self::Output {
        Self::Output {
            shoulder_pitch: Duration::from_secs_f32(
                (self.shoulder_pitch / right.shoulder_pitch).abs(),
            ),
            shoulder_roll: Duration::from_secs_f32(
                (self.shoulder_roll / right.shoulder_roll).abs(),
            ),
            elbow_yaw: Duration::from_secs_f32((self.elbow_yaw / right.elbow_yaw).abs()),
            elbow_roll: Duration::from_secs_f32((self.elbow_roll / right.elbow_roll).abs()),
            wrist_yaw: Duration::from_secs_f32((self.wrist_yaw / right.wrist_yaw).abs()),
            hand: Duration::from_secs_f32((self.hand / right.hand).abs()),
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

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct LegJoints<T> {
    pub ankle_pitch: T,
    pub ankle_roll: T,
    pub hip_pitch: T,
    pub hip_roll: T,
    pub hip_yaw_pitch: T,
    pub knee_pitch: T,
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

    pub fn as_vec(&self) -> Vec<T> {
        vec![
            self.hip_yaw_pitch.clone(),
            self.hip_roll.clone(),
            self.hip_pitch.clone(),
            self.knee_pitch.clone(),
            self.ankle_pitch.clone(),
            self.ankle_roll.clone(),
        ]
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

impl Div<LegJoints<f32>> for LegJoints<f32> {
    type Output = LegJoints<Duration>;

    fn div(self, right: LegJoints<f32>) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: Duration::from_secs_f32(
                (self.hip_yaw_pitch / right.hip_yaw_pitch).abs(),
            ),
            hip_roll: Duration::from_secs_f32((self.hip_roll / right.hip_roll).abs()),
            hip_pitch: Duration::from_secs_f32((self.hip_pitch / right.hip_pitch).abs()),
            knee_pitch: Duration::from_secs_f32((self.knee_pitch / right.knee_pitch).abs()),
            ankle_pitch: Duration::from_secs_f32((self.ankle_pitch / right.ankle_pitch).abs()),
            ankle_roll: Duration::from_secs_f32((self.ankle_roll / right.ankle_roll).abs()),
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

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            hip_yaw_pitch: self
                .hip_yaw_pitch
                .clamp(min.hip_yaw_pitch, max.hip_yaw_pitch),
            hip_roll: self.hip_roll.clamp(min.hip_roll, max.hip_roll),
            hip_pitch: self.hip_pitch.clamp(min.hip_pitch, max.hip_pitch),
            knee_pitch: self.knee_pitch.clamp(min.knee_pitch, max.knee_pitch),
            ankle_pitch: self.ankle_pitch.clamp(min.ankle_pitch, max.ankle_pitch),
            ankle_roll: self.ankle_roll.clamp(min.ankle_roll, max.ankle_roll),
        }
    }
}

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

    pub fn as_vec(&self) -> Vec<Vec<T>> {
        vec![
            self.head.as_vec(),
            self.left_arm.as_vec(),
            self.right_arm.as_vec(),
            self.left_leg.as_vec(),
            self.right_leg.as_vec(),
        ]
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

impl FromIterator<f32> for HeadJoints<f32> {
    fn from_iter<I: IntoIterator<Item = f32>>(angles: I) -> Self {
        let mut angle_iter = angles.into_iter();
        let error_message = "Two HeadJoints expected!";
        Self {
            yaw: angle_iter.next().expect(error_message),
            pitch: angle_iter.next().expect(error_message),
        }
    }
}

impl FromIterator<f32> for ArmJoints<f32> {
    fn from_iter<I: IntoIterator<Item = f32>>(angles: I) -> Self {
        let mut angle_iter = angles.into_iter();
        let error_message = "Six ArmJoints expected!";
        Self {
            shoulder_pitch: angle_iter.next().expect(error_message),
            shoulder_roll: angle_iter.next().expect(error_message),
            elbow_yaw: angle_iter.next().expect(error_message),
            elbow_roll: angle_iter.next().expect(error_message),
            wrist_yaw: angle_iter.next().expect(error_message),
            hand: angle_iter.next().expect(error_message),
        }
    }
}

impl FromIterator<f32> for LegJoints<f32> {
    fn from_iter<I: IntoIterator<Item = f32>>(angles: I) -> Self {
        let mut angle_iter = angles.into_iter();
        let error_message = "Six LegJoints expected!";
        Self {
            hip_yaw_pitch: angle_iter.next().expect(error_message),
            hip_roll: angle_iter.next().expect(error_message),
            hip_pitch: angle_iter.next().expect(error_message),
            knee_pitch: angle_iter.next().expect(error_message),
            ankle_pitch: angle_iter.next().expect(error_message),
            ankle_roll: angle_iter.next().expect(error_message),
        }
    }
}

impl FromIterator<f32> for BodyJoints<f32> {
    fn from_iter<I: IntoIterator<Item = f32>>(angles: I) -> Self {
        let mut angle_iter = angles.into_iter();
        Self {
            left_arm: ArmJoints::from_iter(&mut angle_iter),
            left_leg: LegJoints::from_iter(&mut angle_iter),
            right_arm: ArmJoints::from_iter(&mut angle_iter),
            right_leg: LegJoints::from_iter(&mut angle_iter),
        }
    }
}

impl FromIterator<f32> for Joints<f32> {
    fn from_iter<I: IntoIterator<Item = f32>>(angles: I) -> Self {
        let mut angle_iter = angles.into_iter();
        Self {
            head: HeadJoints::from_iter(&mut angle_iter),
            left_arm: ArmJoints::from_iter(&mut angle_iter),
            right_arm: ArmJoints::from_iter(&mut angle_iter),
            left_leg: LegJoints::from_iter(&mut angle_iter),
            right_leg: LegJoints::from_iter(&mut angle_iter),
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct JointsCommand<T> {
    pub positions: Joints<T>,
    pub stiffnesses: Joints<T>,
}
impl_Interpolate!(f32, JointsCommand<f32>, PI);

impl JointsCommand<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            positions: Joints::mirrored(self.positions),
            stiffnesses: Joints::mirrored(self.stiffnesses),
        }
    }
}

impl Mul<f32> for JointsCommand<f32> {
    type Output = JointsCommand<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            positions: Joints::mul(self.positions, right),
            stiffnesses: self.stiffnesses * right,
        }
    }
}

impl Add<JointsCommand<f32>> for JointsCommand<f32> {
    type Output = JointsCommand<f32>;

    fn add(self, right: JointsCommand<f32>) -> Self::Output {
        Self::Output {
            positions: self.positions + right.positions,
            stiffnesses: self.stiffnesses + right.stiffnesses,
        }
    }
}

impl Sub<JointsCommand<f32>> for JointsCommand<f32> {
    type Output = JointsCommand<f32>;

    fn sub(self, right: JointsCommand<f32>) -> Self::Output {
        Self::Output {
            positions: self.positions - right.positions,
            stiffnesses: self.stiffnesses - right.stiffnesses,
        }
    }
}

impl Div<f32> for JointsCommand<f32> {
    type Output = JointsCommand<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            positions: self.positions / right,
            stiffnesses: self.stiffnesses / right,
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct HeadJointsCommand<T> {
    pub positions: HeadJoints<T>,
    pub stiffnesses: HeadJoints<T>,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct BodyJointsCommand<T> {
    pub positions: BodyJoints<T>,
    pub stiffnesses: BodyJoints<T>,
}

pub mod arm;
pub mod body;
pub mod head;
pub mod leg;
pub mod mirror;

use std::{
    array::IntoIter,
    f32::consts::PI,
    iter::{Chain, Sum},
    ops::{Add, Div, Index, IndexMut, Mul, Sub},
};

use booster::MotorState;
use mirror::SwapSides;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::impl_Interpolate;

use self::{
    arm::{ArmJoint, ArmJoints},
    body::BodyJoints,
    head::{HeadJoint, HeadJoints},
    leg::{LegJoint, LegJoints},
    mirror::Mirror,
};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Eq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum JointsName {
    Head(HeadJoint),
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
    PartialEq,
    Eq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct Joints<T = f32> {
    pub head: HeadJoints<T>,
    pub left_arm: ArmJoints<T>,
    pub right_arm: ArmJoints<T>,
    pub left_leg: LegJoints<T>,
    pub right_leg: LegJoints<T>,
}

impl<'de> Deserialize<'de> for Joints<f32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct JointsVisitor;

        impl<'de> serde::de::Visitor<'de> for JointsVisitor {
            type Value = Joints<f32>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of 22 floats or a map")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::with_capacity(22);
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }

                if values.len() != 22 {
                    return Err(serde::de::Error::invalid_length(values.len(), &self));
                }

                Ok(Joints {
                    head: HeadJoints {
                        yaw: values[0],
                        pitch: values[1],
                    },
                    left_arm: ArmJoints {
                        shoulder_pitch: values[2],
                        shoulder_roll: values[3],
                        shoulder_yaw: values[4],
                        elbow: values[5],
                    },
                    right_arm: ArmJoints {
                        shoulder_pitch: values[6],
                        shoulder_roll: values[7],
                        shoulder_yaw: values[8],
                        elbow: values[9],
                    },
                    left_leg: LegJoints {
                        hip_pitch: values[10],
                        hip_roll: values[11],
                        hip_yaw: values[12],
                        knee: values[13],
                        ankle_up: values[14],
                        ankle_down: values[15],
                    },
                    right_leg: LegJoints {
                        hip_pitch: values[16],
                        hip_roll: values[17],
                        hip_yaw: values[18],
                        knee: values[19],
                        ankle_up: values[20],
                        ankle_down: values[21],
                    },
                })
            }
        }

        deserializer.deserialize_any(JointsVisitor)
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

    pub fn enumerate(self) -> <Joints<(JointsName, T)> as IntoIterator>::IntoIter {
        Joints {
            head: HeadJoints {
                yaw: (JointsName::Head(HeadJoint::Yaw), self.head.yaw),
                pitch: (JointsName::Head(HeadJoint::Pitch), self.head.pitch),
            },
            left_arm: ArmJoints {
                shoulder_pitch: (
                    JointsName::LeftArm(ArmJoint::ShoulderPitch),
                    self.left_arm.shoulder_pitch,
                ),
                shoulder_roll: (
                    JointsName::LeftArm(ArmJoint::ShoulderRoll),
                    self.left_arm.shoulder_roll,
                ),
                shoulder_yaw: (
                    JointsName::LeftArm(ArmJoint::ShoulderYaw),
                    self.left_arm.shoulder_yaw,
                ),
                elbow: (JointsName::LeftArm(ArmJoint::Elbow), self.left_arm.elbow),
            },
            right_arm: ArmJoints {
                shoulder_pitch: (
                    JointsName::RightArm(ArmJoint::ShoulderPitch),
                    self.right_arm.shoulder_pitch,
                ),
                shoulder_roll: (
                    JointsName::RightArm(ArmJoint::ShoulderRoll),
                    self.right_arm.shoulder_roll,
                ),
                shoulder_yaw: (
                    JointsName::RightArm(ArmJoint::ShoulderYaw),
                    self.right_arm.shoulder_yaw,
                ),
                elbow: (JointsName::RightArm(ArmJoint::Elbow), self.right_arm.elbow),
            },
            left_leg: LegJoints {
                ankle_down: (
                    JointsName::LeftLeg(LegJoint::AnkleDown),
                    self.left_leg.ankle_down,
                ),
                ankle_up: (
                    JointsName::LeftLeg(LegJoint::AnkleUp),
                    self.left_leg.ankle_up,
                ),
                hip_pitch: (
                    JointsName::LeftLeg(LegJoint::HipPitch),
                    self.left_leg.hip_pitch,
                ),
                hip_roll: (
                    JointsName::LeftLeg(LegJoint::HipRoll),
                    self.left_leg.hip_roll,
                ),
                hip_yaw: (JointsName::LeftLeg(LegJoint::HipYaw), self.left_leg.hip_yaw),
                knee: (JointsName::LeftLeg(LegJoint::Knee), self.left_leg.knee),
            },
            right_leg: LegJoints {
                ankle_down: (
                    JointsName::RightLeg(LegJoint::AnkleDown),
                    self.right_leg.ankle_down,
                ),
                ankle_up: (
                    JointsName::RightLeg(LegJoint::AnkleUp),
                    self.right_leg.ankle_up,
                ),
                hip_pitch: (
                    JointsName::RightLeg(LegJoint::HipPitch),
                    self.right_leg.hip_pitch,
                ),
                hip_roll: (
                    JointsName::RightLeg(LegJoint::HipRoll),
                    self.right_leg.hip_roll,
                ),
                hip_yaw: (
                    JointsName::RightLeg(LegJoint::HipYaw),
                    self.right_leg.hip_yaw,
                ),
                knee: (JointsName::RightLeg(LegJoint::Knee), self.right_leg.knee),
            },
        }
        .into_iter()
    }

    pub fn body(self) -> BodyJoints<T> {
        BodyJoints {
            left_arm: self.left_arm,
            right_arm: self.right_arm,
            left_leg: self.left_leg,
            right_leg: self.right_leg,
        }
    }

    pub fn joint_positions(motor_states_serial: &[MotorState]) -> Joints {
        let ms = &motor_states_serial;
        if ms.len() != 22 {
            panic!("expected 22 motor states, got {}", ms.len());
        }

        let head_yaw = ms[0].position;
        let head_pitch = ms[1].position;
        let left_shoulder_pitch = ms[2].position;
        let left_shoulder_roll = ms[3].position;
        let left_shoulder_yaw = ms[4].position;
        let left_elbow = ms[5].position;
        let right_shoulder_pitch = ms[6].position;
        let right_shoulder_roll = ms[7].position;
        let right_shoulder_yaw = ms[8].position;
        let right_elbow = ms[9].position;
        let left_hip_pitch = ms[10].position;
        let left_hip_roll = ms[11].position;
        let left_hip_yaw = ms[12].position;
        let left_knee = ms[13].position;
        let left_ankle_up = ms[14].position;
        let left_ankle_down = ms[15].position;
        let right_hip_pitch = ms[16].position;
        let right_hip_roll = ms[17].position;
        let right_hip_yaw = ms[18].position;
        let right_knee = ms[19].position;
        let right_ankle_up = ms[20].position;
        let right_ankle_down = ms[21].position;

        Joints {
            head: HeadJoints {
                yaw: head_yaw,
                pitch: head_pitch,
            },
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
                hip_yaw: left_hip_yaw,
                hip_roll: left_hip_roll,
                knee: left_knee,
                ankle_up: left_ankle_up,
                ankle_down: left_ankle_down,
            },
            right_leg: LegJoints {
                hip_pitch: right_hip_pitch,
                hip_yaw: right_hip_yaw,
                hip_roll: right_hip_roll,
                knee: right_knee,
                ankle_up: right_ankle_up,
                ankle_down: right_ankle_down,
            },
        }
    }

    pub fn joint_velocities(motor_states_serial: &[MotorState]) -> Joints {
        let ms = &motor_states_serial;
        if ms.len() != 22 {
            panic!("expected 22 motor states, got {}", ms.len());
        }

        let head_yaw = ms[0].velocity;
        let head_pitch = ms[1].velocity;
        let left_shoulder_pitch = ms[2].velocity;
        let left_shoulder_roll = ms[3].velocity;
        let left_shoulder_yaw = ms[4].velocity;
        let left_elbow = ms[5].velocity;
        let right_shoulder_pitch = ms[6].velocity;
        let right_shoulder_roll = ms[7].velocity;
        let right_shoulder_yaw = ms[8].velocity;
        let right_elbow = ms[9].velocity;
        let left_hip_pitch = ms[10].velocity;
        let left_hip_roll = ms[11].velocity;
        let left_hip_yaw = ms[12].velocity;
        let left_knee = ms[13].velocity;
        let left_ankle_up = ms[14].velocity;
        let left_ankle_down = ms[15].velocity;
        let right_hip_pitch = ms[16].velocity;
        let right_hip_roll = ms[17].velocity;
        let right_hip_yaw = ms[18].velocity;
        let right_knee = ms[19].velocity;
        let right_ankle_up = ms[20].velocity;
        let right_ankle_down = ms[21].velocity;

        Joints {
            head: HeadJoints {
                yaw: head_yaw,
                pitch: head_pitch,
            },
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
                hip_yaw: left_hip_yaw,
                hip_roll: left_hip_roll,
                knee: left_knee,
                ankle_up: left_ankle_up,
                ankle_down: left_ankle_down,
            },
            right_leg: LegJoints {
                hip_pitch: right_hip_pitch,
                hip_yaw: right_hip_yaw,
                hip_roll: right_hip_roll,
                knee: right_knee,
                ankle_up: right_ankle_up,
                ankle_down: right_ankle_down,
            },
        }
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
        Chain<Chain<Chain<IntoIter<T, 2>, IntoIter<T, 4>>, IntoIter<T, 4>>, IntoIter<T, 6>>,
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

impl Mirror for Joints<f32> {
    fn mirrored(self) -> Self {
        Self {
            head: self.head.mirrored(),
            left_arm: self.right_arm.mirrored(),
            right_arm: self.left_arm.mirrored(),
            left_leg: self.right_leg.mirrored(),
            right_leg: self.left_leg.mirrored(),
        }
    }
}

impl SwapSides for Joints<f32> {
    fn swapped_sides(self) -> Self {
        Self {
            head: self.head,
            left_arm: self.right_arm,
            right_arm: self.left_arm,
            left_leg: self.right_leg,
            right_leg: self.left_leg,
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

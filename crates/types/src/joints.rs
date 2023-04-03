use std::{
    iter::Sum,
    ops::{Add, Mul, Sub},
};

use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct HeadJoints {
    pub yaw: f32,
    pub pitch: f32,
}

impl HeadJoints {
    pub fn mirrored(self) -> Self {
        Self {
            yaw: -self.yaw,
            pitch: self.pitch,
        }
    }

    pub fn fill(value: f32) -> Self {
        Self {
            yaw: value,
            pitch: value,
        }
    }
}

impl From<Joints> for HeadJoints {
    fn from(joints: Joints) -> Self {
        Self {
            yaw: joints.head.yaw,
            pitch: joints.head.pitch,
        }
    }
}

impl Mul<f32> for HeadJoints {
    type Output = HeadJoints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            yaw: self.yaw * scale_factor,
            pitch: self.pitch * scale_factor,
        }
    }
}

impl Add for HeadJoints {
    type Output = HeadJoints;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw + right.yaw,
            pitch: self.pitch + right.pitch,
        }
    }
}

impl Sub for HeadJoints {
    type Output = HeadJoints;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw - right.yaw,
            pitch: self.pitch - right.pitch,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct ArmJoints {
    pub shoulder_pitch: f32,
    pub shoulder_roll: f32,
    pub elbow_yaw: f32,
    pub elbow_roll: f32,
    pub wrist_yaw: f32,
    pub hand: f32,
}

impl ArmJoints {
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

    pub fn fill(value: f32) -> Self {
        Self {
            shoulder_pitch: value,
            shoulder_roll: value,
            elbow_yaw: value,
            elbow_roll: value,
            wrist_yaw: value,
            hand: value,
        }
    }
}

impl Mul<f32> for ArmJoints {
    type Output = ArmJoints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch * scale_factor,
            shoulder_roll: self.shoulder_roll * scale_factor,
            elbow_yaw: self.elbow_yaw * scale_factor,
            elbow_roll: self.elbow_roll * scale_factor,
            wrist_yaw: self.wrist_yaw * scale_factor,
            hand: self.hand * scale_factor,
        }
    }
}

impl Add for ArmJoints {
    type Output = ArmJoints;

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

impl Sub for ArmJoints {
    type Output = ArmJoints;

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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct LegJoints {
    pub hip_yaw_pitch: f32,
    pub hip_roll: f32,
    pub hip_pitch: f32,
    pub knee_pitch: f32,
    pub ankle_pitch: f32,
    pub ankle_roll: f32,
}

impl LegJoints {
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

    pub fn fill(value: f32) -> Self {
        Self {
            hip_yaw_pitch: value,
            hip_roll: value,
            hip_pitch: value,
            knee_pitch: value,
            ankle_pitch: value,
            ankle_roll: value,
        }
    }
}

impl Mul<f32> for LegJoints {
    type Output = LegJoints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch * scale_factor,
            hip_roll: self.hip_roll * scale_factor,
            hip_pitch: self.hip_pitch * scale_factor,
            knee_pitch: self.knee_pitch * scale_factor,
            ankle_pitch: self.ankle_pitch * scale_factor,
            ankle_roll: self.ankle_roll * scale_factor,
        }
    }
}

impl Add for LegJoints {
    type Output = LegJoints;

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

impl Sub for LegJoints {
    type Output = LegJoints;

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

impl AbsDiffEq for LegJoints {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        f32::abs_diff_eq(&self.hip_yaw_pitch, &other.hip_yaw_pitch, epsilon)
            && f32::abs_diff_eq(&self.hip_roll, &other.hip_roll, epsilon)
            && f32::abs_diff_eq(&self.hip_pitch, &other.hip_pitch, epsilon)
            && f32::abs_diff_eq(&self.knee_pitch, &other.knee_pitch, epsilon)
            && f32::abs_diff_eq(&self.ankle_pitch, &other.ankle_pitch, epsilon)
            && f32::abs_diff_eq(&self.ankle_roll, &other.ankle_roll, epsilon)
    }
}

impl RelativeEq for LegJoints {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        f32::relative_eq(
            &self.hip_yaw_pitch,
            &other.hip_yaw_pitch,
            epsilon,
            max_relative,
        ) && f32::relative_eq(&self.hip_roll, &other.hip_roll, epsilon, max_relative)
            && f32::relative_eq(&self.hip_pitch, &other.hip_pitch, epsilon, max_relative)
            && f32::relative_eq(&self.knee_pitch, &other.knee_pitch, epsilon, max_relative)
            && f32::relative_eq(&self.ankle_pitch, &other.ankle_pitch, epsilon, max_relative)
            && f32::relative_eq(&self.ankle_roll, &other.ankle_roll, epsilon, max_relative)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct BodyJoints {
    pub left_arm: ArmJoints,
    pub right_arm: ArmJoints,
    pub left_leg: LegJoints,
    pub right_leg: LegJoints,
}

impl BodyJoints {
    pub fn fill(value: f32) -> Self {
        Self {
            left_arm: ArmJoints::fill(value),
            right_arm: ArmJoints::fill(value),
            left_leg: LegJoints::fill(value),
            right_leg: LegJoints::fill(value),
        }
    }

    pub fn selective_fill(value_arm: f32, value_leg: f32) -> Self {
        Self {
            left_arm: ArmJoints::fill(value_arm),
            right_arm: ArmJoints::fill(value_arm),
            left_leg: LegJoints::fill(value_leg),
            right_leg: LegJoints::fill(value_leg),
        }
    }
}

impl From<Joints> for BodyJoints {
    fn from(joints: Joints) -> Self {
        Self {
            left_arm: joints.left_arm,
            right_arm: joints.right_arm,
            left_leg: joints.left_leg,
            right_leg: joints.right_leg,
        }
    }
}

impl Mul<f32> for BodyJoints {
    type Output = BodyJoints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm * scale_factor,
            right_arm: self.right_arm * scale_factor,
            left_leg: self.left_leg * scale_factor,
            right_leg: self.right_leg * scale_factor,
        }
    }
}

impl Add for BodyJoints {
    type Output = BodyJoints;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm + right.left_arm,
            right_arm: self.right_arm + right.right_arm,
            left_leg: self.left_leg + right.left_leg,
            right_leg: self.right_leg + right.right_leg,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Joints {
    pub head: HeadJoints,
    pub left_arm: ArmJoints,
    pub right_arm: ArmJoints,
    pub left_leg: LegJoints,
    pub right_leg: LegJoints,
}

impl Joints {
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

    pub fn to_angles(self) -> [f32; 26] {
        [
            self.head.yaw,
            self.head.pitch,
            self.left_arm.shoulder_pitch,
            self.left_arm.shoulder_roll,
            self.left_arm.elbow_yaw,
            self.left_arm.elbow_roll,
            self.left_arm.wrist_yaw,
            self.left_arm.hand,
            self.right_arm.shoulder_pitch,
            self.right_arm.shoulder_roll,
            self.right_arm.elbow_yaw,
            self.right_arm.elbow_roll,
            self.right_arm.wrist_yaw,
            self.right_arm.hand,
            self.left_leg.hip_yaw_pitch,
            self.left_leg.hip_roll,
            self.left_leg.hip_pitch,
            self.left_leg.knee_pitch,
            self.left_leg.ankle_pitch,
            self.left_leg.ankle_roll,
            self.right_leg.hip_yaw_pitch,
            self.right_leg.hip_roll,
            self.right_leg.hip_pitch,
            self.right_leg.knee_pitch,
            self.right_leg.ankle_pitch,
            self.right_leg.ankle_roll,
        ]
    }

    pub fn from_head_and_body(head: HeadJoints, body: BodyJoints) -> Self {
        Self {
            head,
            left_arm: body.left_arm,
            right_arm: body.right_arm,
            left_leg: body.left_leg,
            right_leg: body.right_leg,
        }
    }

    pub fn fill(value: f32) -> Self {
        Self::from_head_and_body(HeadJoints::fill(value), BodyJoints::fill(value))
    }

    pub fn selectively_fill(value: f32, value_arm: f32, value_leg: f32) -> Self {
        Self::from_head_and_body(
            HeadJoints::fill(value),
            BodyJoints::selective_fill(value_arm, value_leg),
        )
    }
}

impl Mul<f32> for Joints {
    type Output = Joints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            head: self.head * scale_factor,
            left_arm: self.left_arm * scale_factor,
            right_arm: self.right_arm * scale_factor,
            left_leg: self.left_leg * scale_factor,
            right_leg: self.right_leg * scale_factor,
        }
    }
}

impl Add for Joints {
    type Output = Joints;

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

impl Sub for Joints {
    type Output = Joints;

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

impl Sum for Joints {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Joints::default(), |acc, x| acc + x)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct JointsCommand {
    pub positions: Joints,
    pub stiffnesses: Joints,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct HeadJointsCommand {
    pub positions: HeadJoints,
    pub stiffnesses: HeadJoints,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct BodyJointsCommand {
    pub positions: BodyJoints,
    pub stiffnesses: BodyJoints,
}

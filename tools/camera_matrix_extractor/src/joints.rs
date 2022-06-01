use std::{
    iter::Sum,
    ops::{Add, Mul},
};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, Value};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct HeadJoints {
    pub yaw: f32,
    pub pitch: f32,
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

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw + rhs.yaw,
            pitch: self.pitch + rhs.pitch,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ArmJoints {
    pub shoulder_pitch: f32,
    pub shoulder_roll: f32,
    pub elbow_yaw: f32,
    pub elbow_roll: f32,
    pub wrist_yaw: f32,
    pub hand: f32,
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

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch + rhs.shoulder_pitch,
            shoulder_roll: self.shoulder_roll + rhs.shoulder_roll,
            elbow_yaw: self.elbow_yaw + rhs.elbow_yaw,
            elbow_roll: self.elbow_roll + rhs.elbow_roll,
            wrist_yaw: self.wrist_yaw + rhs.wrist_yaw,
            hand: self.hand + rhs.hand,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LegJoints {
    pub hip_yaw_pitch: f32,
    pub hip_roll: f32,
    pub hip_pitch: f32,
    pub knee_pitch: f32,
    pub ankle_pitch: f32,
    pub ankle_roll: f32,
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

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch + rhs.hip_yaw_pitch,
            hip_roll: self.hip_roll + rhs.hip_roll,
            hip_pitch: self.hip_pitch + rhs.hip_pitch,
            knee_pitch: self.knee_pitch + rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch + rhs.ankle_pitch,
            ankle_roll: self.ankle_roll + rhs.ankle_roll,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct BodyJoints {
    pub left_arm: ArmJoints,
    pub right_arm: ArmJoints,
    pub left_leg: LegJoints,
    pub right_leg: LegJoints,
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

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm + rhs.left_arm,
            right_arm: self.right_arm + rhs.right_arm,
            left_leg: self.left_leg + rhs.left_leg,
            right_leg: self.right_leg + rhs.right_leg,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Joints {
    pub head: HeadJoints,
    pub left_arm: ArmJoints,
    pub right_arm: ArmJoints,
    pub left_leg: LegJoints,
    pub right_leg: LegJoints,
}

impl TryFrom<&Value> for Joints {
    type Error = anyhow::Error;

    fn try_from(replay_frame: &Value) -> anyhow::Result<Self> {
        let joint_angles = replay_frame
            .get("jointAngles")
            .ok_or_else(|| anyhow!("replay_frame.get(\"jointAngles\")"))?;
        let angles: Vec<f32> =
            from_value(joint_angles.clone()).context("from_value(joint_angles)")?;
        Ok(Self::from_angles(&angles))
    }
}

impl Joints {
    pub fn from_angles(angles: &[f32]) -> Self {
        assert!(angles.len() >= 26);
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

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            head: self.head + rhs.head,
            left_arm: self.left_arm + rhs.left_arm,
            right_arm: self.right_arm + rhs.right_arm,
            left_leg: self.left_leg + rhs.left_leg,
            right_leg: self.right_leg + rhs.right_leg,
        }
    }
}

impl Sum for Joints {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Joints::default(), |acc, x| acc + x)
    }
}

use std::{
    iter::Sum,
    ops::{Add, Mul},
};

use color_eyre::{
    eyre::{eyre, WrapErr},
    Report, Result,
};
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

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw + right.yaw,
            pitch: self.pitch + right.pitch,
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

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            left_arm: self.left_arm + right.left_arm,
            right_arm: self.right_arm + right.right_arm,
            left_leg: self.left_leg + right.left_leg,
            right_leg: self.right_leg + right.right_leg,
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
    type Error = Report;

    fn try_from(replay_frame: &Value) -> Result<Self> {
        let joint_angles = replay_frame
            .get("jointAngles")
            .ok_or_else(|| eyre!("replay_frame.get(\"jointAngles\")"))?;
        let angles: Vec<f32> =
            from_value(joint_angles.clone()).wrap_err("from_value(joint_angles)")?;
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

impl Sum for Joints {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Joints::default(), |acc, x| acc + x)
    }
}

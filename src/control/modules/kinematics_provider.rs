use anyhow::Result;
use macros::{module, require_some};
use nalgebra::{Isometry3, Translation};

use crate::{
    kinematics::{
        head_to_neck, left_ankle_to_left_tibia, left_elbow_to_left_upper_arm,
        left_foot_to_left_ankle, left_forearm_to_left_elbow, left_hip_to_left_pelvis,
        left_pelvis_to_robot, left_shoulder_to_robot, left_thigh_to_left_hip,
        left_tibia_to_left_thigh, left_upper_arm_to_left_shoulder, left_wrist_to_left_forearm,
        neck_to_robot, right_ankle_to_right_tibia, right_elbow_to_right_upper_arm,
        right_foot_to_right_ankle, right_forearm_to_right_elbow, right_hip_to_right_pelvis,
        right_pelvis_to_robot, right_shoulder_to_robot, right_thigh_to_right_hip,
        right_tibia_to_right_thigh, right_upper_arm_to_right_shoulder,
        right_wrist_to_right_forearm,
    },
    types::{RobotDimensions, RobotKinematics, SensorData},
};

pub struct KinematicsProvider;

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[main_output(data_type = RobotKinematics)]
impl KinematicsProvider {}

impl KinematicsProvider {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let joints = require_some!(context.sensor_data).positions;

        // head
        let neck_to_robot = neck_to_robot(&joints.head);
        let head_to_robot = neck_to_robot * head_to_neck(&joints.head);
        // torso
        let torso_to_robot = Isometry3::from(RobotDimensions::TORSO_TO_ROBOT);
        // left arm
        let left_shoulder_to_robot = left_shoulder_to_robot(&joints.left_arm);
        let left_upper_arm_to_robot =
            left_shoulder_to_robot * left_upper_arm_to_left_shoulder(&joints.left_arm);
        let left_elbow_to_robot =
            left_upper_arm_to_robot * left_elbow_to_left_upper_arm(&joints.left_arm);
        let left_forearm_to_robot =
            left_elbow_to_robot * left_forearm_to_left_elbow(&joints.left_arm);
        let left_wrist_to_robot =
            left_forearm_to_robot * left_wrist_to_left_forearm(&joints.left_arm);
        // right arm
        let right_shoulder_to_robot = right_shoulder_to_robot(&joints.right_arm);
        let right_upper_arm_to_robot =
            right_shoulder_to_robot * right_upper_arm_to_right_shoulder(&joints.right_arm);
        let right_elbow_to_robot =
            right_upper_arm_to_robot * right_elbow_to_right_upper_arm(&joints.right_arm);
        let right_forearm_to_robot =
            right_elbow_to_robot * right_forearm_to_right_elbow(&joints.right_arm);
        let right_wrist_to_robot =
            right_forearm_to_robot * right_wrist_to_right_forearm(&joints.right_arm);
        // left leg
        let left_pelvis_to_robot = left_pelvis_to_robot(&joints.left_leg);
        let left_hip_to_robot = left_pelvis_to_robot * left_hip_to_left_pelvis(&joints.left_leg);
        let left_thigh_to_robot = left_hip_to_robot * left_thigh_to_left_hip(&joints.left_leg);
        let left_tibia_to_robot = left_thigh_to_robot * left_tibia_to_left_thigh(&joints.left_leg);
        let left_ankle_to_robot = left_tibia_to_robot * left_ankle_to_left_tibia(&joints.left_leg);
        let left_foot_to_robot = left_ankle_to_robot * left_foot_to_left_ankle(&joints.left_leg);
        let left_sole_to_robot =
            left_foot_to_robot * Translation::from(RobotDimensions::ANKLE_TO_SOLE);
        // right leg
        let right_pelvis_to_robot = right_pelvis_to_robot(&joints.right_leg);
        let right_hip_to_robot =
            right_pelvis_to_robot * right_hip_to_right_pelvis(&joints.right_leg);
        let right_thigh_to_robot = right_hip_to_robot * right_thigh_to_right_hip(&joints.right_leg);
        let right_tibia_to_robot =
            right_thigh_to_robot * right_tibia_to_right_thigh(&joints.right_leg);
        let right_ankle_to_robot =
            right_tibia_to_robot * right_ankle_to_right_tibia(&joints.right_leg);
        let right_foot_to_robot =
            right_ankle_to_robot * right_foot_to_right_ankle(&joints.right_leg);
        let right_sole_to_robot =
            right_foot_to_robot * Translation::from(RobotDimensions::ANKLE_TO_SOLE);

        Ok(MainOutputs {
            robot_kinematics: Some(RobotKinematics {
                neck_to_robot,
                head_to_robot,
                torso_to_robot,
                left_shoulder_to_robot,
                left_upper_arm_to_robot,
                left_elbow_to_robot,
                left_forearm_to_robot,
                left_wrist_to_robot,
                right_shoulder_to_robot,
                right_upper_arm_to_robot,
                right_elbow_to_robot,
                right_forearm_to_robot,
                right_wrist_to_robot,
                left_pelvis_to_robot,
                left_hip_to_robot,
                left_thigh_to_robot,
                left_tibia_to_robot,
                left_ankle_to_robot,
                left_foot_to_robot,
                left_sole_to_robot,
                right_pelvis_to_robot,
                right_hip_to_robot,
                right_thigh_to_robot,
                right_tibia_to_robot,
                right_ankle_to_robot,
                right_foot_to_robot,
                right_sole_to_robot,
            }),
        })
    }
}

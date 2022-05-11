use macros::{module, require_some};
use nalgebra::{Point, Point3};

use crate::types::{RobotKinematics, RobotMass};

pub struct CenterOfMassProvider {}

#[module(control)]
#[input(path = robot_kinematics, data_type = RobotKinematics)]
#[main_output(name = center_of_mass, data_type = Point3<f32>)]
impl CenterOfMassProvider {}

impl CenterOfMassProvider {
    pub fn new() -> Self {
        Self {}
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let robot_kinematics = require_some!(context.robot_kinematics);

        let center_of_mass = (RobotMass::TORSO.mass
            * (robot_kinematics.torso_to_robot * RobotMass::TORSO.center).coords
            + RobotMass::NECK.mass
                * (robot_kinematics.neck_to_robot * RobotMass::NECK.center).coords
            + RobotMass::HEAD.mass
                * (robot_kinematics.head_to_robot * RobotMass::HEAD.center).coords
            + RobotMass::LEFT_SHOULDER.mass
                * (robot_kinematics.left_shoulder_to_robot * RobotMass::LEFT_SHOULDER.center)
                    .coords
            + RobotMass::LEFT_UPPER_ARM.mass
                * (robot_kinematics.left_upper_arm_to_robot * RobotMass::LEFT_UPPER_ARM.center)
                    .coords
            + RobotMass::LEFT_ELBOW.mass
                * (robot_kinematics.left_elbow_to_robot * RobotMass::LEFT_ELBOW.center).coords
            + RobotMass::LEFT_FOREARM.mass
                * (robot_kinematics.left_forearm_to_robot * RobotMass::LEFT_FOREARM.center).coords
            + RobotMass::LEFT_WRIST.mass
                * (robot_kinematics.left_wrist_to_robot * RobotMass::LEFT_WRIST.center).coords
            + RobotMass::RIGHT_SHOULDER.mass
                * (robot_kinematics.right_shoulder_to_robot * RobotMass::RIGHT_SHOULDER.center)
                    .coords
            + RobotMass::RIGHT_UPPER_ARM.mass
                * (robot_kinematics.right_upper_arm_to_robot * RobotMass::RIGHT_UPPER_ARM.center)
                    .coords
            + RobotMass::RIGHT_ELBOW.mass
                * (robot_kinematics.right_elbow_to_robot * RobotMass::RIGHT_ELBOW.center).coords
            + RobotMass::RIGHT_FOREARM.mass
                * (robot_kinematics.right_forearm_to_robot * RobotMass::RIGHT_FOREARM.center)
                    .coords
            + RobotMass::RIGHT_WRIST.mass
                * (robot_kinematics.right_wrist_to_robot * RobotMass::RIGHT_WRIST.center).coords
            + RobotMass::LEFT_PELVIS.mass
                * (robot_kinematics.left_pelvis_to_robot * RobotMass::LEFT_PELVIS.center).coords
            + RobotMass::LEFT_HIP.mass
                * (robot_kinematics.left_hip_to_robot * RobotMass::LEFT_HIP.center).coords
            + RobotMass::LEFT_THIGH.mass
                * (robot_kinematics.left_thigh_to_robot * RobotMass::LEFT_THIGH.center).coords
            + RobotMass::LEFT_TIBIA.mass
                * (robot_kinematics.left_tibia_to_robot * RobotMass::LEFT_TIBIA.center).coords
            + RobotMass::LEFT_ANKLE.mass
                * (robot_kinematics.left_ankle_to_robot * RobotMass::LEFT_ANKLE.center).coords
            + RobotMass::LEFT_FOOT.mass
                * (robot_kinematics.left_foot_to_robot * RobotMass::LEFT_FOOT.center).coords
            + RobotMass::RIGHT_PELVIS.mass
                * (robot_kinematics.right_pelvis_to_robot * RobotMass::RIGHT_PELVIS.center).coords
            + RobotMass::RIGHT_HIP.mass
                * (robot_kinematics.right_hip_to_robot * RobotMass::RIGHT_HIP.center).coords
            + RobotMass::RIGHT_THIGH.mass
                * (robot_kinematics.right_thigh_to_robot * RobotMass::RIGHT_THIGH.center).coords
            + RobotMass::RIGHT_TIBIA.mass
                * (robot_kinematics.right_tibia_to_robot * RobotMass::RIGHT_TIBIA.center).coords
            + RobotMass::RIGHT_ANKLE.mass
                * (robot_kinematics.right_ankle_to_robot * RobotMass::RIGHT_ANKLE.center).coords
            + RobotMass::RIGHT_FOOT.mass
                * (robot_kinematics.right_foot_to_robot * RobotMass::RIGHT_FOOT.center).coords)
            / RobotMass::TOTAL_MASS;

        Ok(MainOutputs {
            center_of_mass: Some(Point::from(center_of_mass)),
        })
    }
}

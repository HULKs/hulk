use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Point3;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{coordinate_systems::Robot, robot_kinematics::RobotKinematics, robot_masses};

#[derive(Deserialize, Serialize)]
pub struct CenterOfMassProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub center_of_mass: MainOutput<Point3<Robot>>,
}

impl CenterOfMassProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let robot_kinematics = context.robot_kinematics;
        let center_of_mass = ((robot_kinematics.torso_to_robot * robot_masses::TORSO.center)
            .coords()
            * robot_masses::TORSO.mass
            + (robot_kinematics.neck_to_robot * robot_masses::NECK.center).coords()
                * robot_masses::NECK.mass
            + (robot_kinematics.head_to_robot * robot_masses::HEAD.center).coords()
                * robot_masses::HEAD.mass
            + (robot_kinematics.left_shoulder_to_robot * robot_masses::LEFT_SHOULDER.center)
                .coords()
                * robot_masses::LEFT_SHOULDER.mass
            + (robot_kinematics.left_upper_arm_to_robot * robot_masses::LEFT_UPPER_ARM.center)
                .coords()
                * robot_masses::LEFT_UPPER_ARM.mass
            + (robot_kinematics.left_elbow_to_robot * robot_masses::LEFT_ELBOW.center).coords()
                * robot_masses::LEFT_ELBOW.mass
            + (robot_kinematics.left_forearm_to_robot * robot_masses::LEFT_FOREARM.center)
                .coords()
                * robot_masses::LEFT_FOREARM.mass
            + (robot_kinematics.left_wrist_to_robot * robot_masses::LEFT_WRIST.center).coords()
                * robot_masses::LEFT_WRIST.mass
            + (robot_kinematics.right_shoulder_to_robot * robot_masses::RIGHT_SHOULDER.center)
                .coords()
                * robot_masses::RIGHT_SHOULDER.mass
            + (robot_kinematics.right_upper_arm_to_robot * robot_masses::RIGHT_UPPER_ARM.center)
                .coords()
                * robot_masses::RIGHT_UPPER_ARM.mass
            + (robot_kinematics.right_elbow_to_robot * robot_masses::RIGHT_ELBOW.center).coords()
                * robot_masses::RIGHT_ELBOW.mass
            + (robot_kinematics.right_forearm_to_robot * robot_masses::RIGHT_FOREARM.center)
                .coords()
                * robot_masses::RIGHT_FOREARM.mass
            + (robot_kinematics.right_wrist_to_robot * robot_masses::RIGHT_WRIST.center).coords()
                * robot_masses::RIGHT_WRIST.mass
            + (robot_kinematics.left_pelvis_to_robot * robot_masses::LEFT_PELVIS.center).coords()
                * robot_masses::LEFT_PELVIS.mass
            + (robot_kinematics.left_hip_to_robot * robot_masses::LEFT_HIP.center).coords()
                * robot_masses::LEFT_HIP.mass
            + (robot_kinematics.left_thigh_to_robot * robot_masses::LEFT_THIGH.center).coords()
                * robot_masses::LEFT_THIGH.mass
            + (robot_kinematics.left_tibia_to_robot * robot_masses::LEFT_TIBIA.center).coords()
                * robot_masses::LEFT_TIBIA.mass
            + (robot_kinematics.left_ankle_to_robot * robot_masses::LEFT_ANKLE.center).coords()
                * robot_masses::LEFT_ANKLE.mass
            + (robot_kinematics.left_foot_to_robot * robot_masses::LEFT_FOOT.center).coords()
                * robot_masses::LEFT_FOOT.mass
            + (robot_kinematics.right_pelvis_to_robot * robot_masses::RIGHT_PELVIS.center)
                .coords()
                * robot_masses::RIGHT_PELVIS.mass
            + (robot_kinematics.right_hip_to_robot * robot_masses::RIGHT_HIP.center).coords()
                * robot_masses::RIGHT_HIP.mass
            + (robot_kinematics.right_thigh_to_robot * robot_masses::RIGHT_THIGH.center).coords()
                * robot_masses::RIGHT_THIGH.mass
            + (robot_kinematics.right_tibia_to_robot * robot_masses::RIGHT_TIBIA.center).coords()
                * robot_masses::RIGHT_TIBIA.mass
            + (robot_kinematics.right_ankle_to_robot * robot_masses::RIGHT_ANKLE.center).coords()
                * robot_masses::RIGHT_ANKLE.mass
            + (robot_kinematics.right_foot_to_robot * robot_masses::RIGHT_FOOT.center).coords()
                * robot_masses::RIGHT_FOOT.mass)
            / robot_masses::TOTAL_MASS;

        Ok(MainOutputs {
            center_of_mass: center_of_mass.as_point().into(),
        })
    }
}

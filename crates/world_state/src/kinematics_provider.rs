use booster::{JointsMotorState, MotorState};
use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use kinematics::forward::{
    head_to_neck, left_ankle_to_left_tibia, left_foot_to_left_ankle,
    left_forearm_to_left_upper_arm, left_hip_to_left_pelvis, left_inner_shoulder_to_robot,
    left_outer_shoulder_to_left_inner_shoulder, left_pelvis_to_robot, left_thigh_to_left_hip,
    left_tibia_to_left_thigh, left_upper_arm_to_left_outer_shoulder, neck_to_robot,
    right_ankle_to_right_tibia, right_foot_to_right_ankle, right_forearm_to_right_upper_arm,
    right_hip_to_right_pelvis, right_inner_shoulder_to_robot,
    right_outer_shoulder_to_right_inner_shoulder, right_pelvis_to_robot, right_thigh_to_right_hip,
    right_tibia_to_right_thigh, right_upper_arm_to_right_outer_shoulder,
};
use linear_algebra::Isometry3;
use serde::{Deserialize, Serialize};
use types::joints::Joints;
use types::robot_kinematics::{
    RobotHeadKinematics, RobotLeftArmKinematics, RobotLeftLegKinematics, RobotRightArmKinematics,
    RobotRightLegKinematics, RobotTorsoKinematics,
};
use types::{robot_dimensions::RobotDimensions, robot_kinematics::RobotKinematics};

#[derive(Deserialize, Serialize)]
pub struct KinematicsProvider {
    last_robot_kinematics: RobotKinematics,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    serial_motor_states: PerceptionInput<Joints<MotorState>, "Control", "serial_motor_states">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_kinematics: MainOutput<RobotKinematics>,
}

impl KinematicsProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_robot_kinematics: RobotKinematics::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        // TOOD USE TEMPORARIES
        let Some(time_tagged_serial_motor_states) = &context
            .serial_motor_states
            .persistent
            .iter()
            .chain(&context.serial_motor_states.temporary)
            .last()
        else {
            return Ok(MainOutputs {
                robot_kinematics: self.last_robot_kinematics.clone().into(),
            });
        };

        let Some(newest_serial_motor_states) = time_tagged_serial_motor_states.1.last() else {
            return Ok(MainOutputs {
                robot_kinematics: self.last_robot_kinematics.clone().into(),
            });
        };

        let measured_positions = newest_serial_motor_states.positions();

        // head
        let neck_to_robot = neck_to_robot(&measured_positions.head);
        let head_to_robot = neck_to_robot * head_to_neck(&measured_positions.head);
        // torso
        let torso_to_robot = Isometry3::from(RobotDimensions::ROBOT_TO_TORSO);
        // left arm
        let left_inner_shoulder_to_robot =
            left_inner_shoulder_to_robot(&measured_positions.left_arm);
        let left_outer_shoulder_to_robot = left_inner_shoulder_to_robot
            * left_outer_shoulder_to_left_inner_shoulder(&measured_positions.left_arm);
        let left_upper_arm_to_robot = left_outer_shoulder_to_robot
            * left_upper_arm_to_left_outer_shoulder(&measured_positions.left_arm);
        let left_forearm_to_robot =
            left_upper_arm_to_robot * left_forearm_to_left_upper_arm(&measured_positions.left_arm);

        // right arm
        let right_inner_shoulder_to_robot =
            right_inner_shoulder_to_robot(&measured_positions.right_arm);
        let right_outer_shoulder_to_robot = right_inner_shoulder_to_robot
            * right_outer_shoulder_to_right_inner_shoulder(&measured_positions.right_arm);
        let right_upper_arm_to_robot = right_outer_shoulder_to_robot
            * right_upper_arm_to_right_outer_shoulder(&measured_positions.right_arm);

        let right_forearm_to_robot = right_upper_arm_to_robot
            * right_forearm_to_right_upper_arm(&measured_positions.right_arm);

        // left leg
        let left_pelvis_to_robot = left_pelvis_to_robot(&measured_positions.left_leg);
        let left_hip_to_robot =
            left_pelvis_to_robot * left_hip_to_left_pelvis(&measured_positions.left_leg);
        let left_thigh_to_robot =
            left_hip_to_robot * left_thigh_to_left_hip(&measured_positions.left_leg);
        let left_tibia_to_robot =
            left_thigh_to_robot * left_tibia_to_left_thigh(&measured_positions.left_leg);
        let left_ankle_to_robot =
            left_tibia_to_robot * left_ankle_to_left_tibia(&measured_positions.left_leg);
        let left_foot_to_robot =
            left_ankle_to_robot * left_foot_to_left_ankle(&measured_positions.left_leg);
        let left_sole_to_robot =
            left_foot_to_robot * Isometry3::from(RobotDimensions::LEFT_FOOT_TO_LEFT_SOLE);
        // right leg
        let right_pelvis_to_robot = right_pelvis_to_robot(&measured_positions.right_leg);
        let right_hip_to_robot =
            right_pelvis_to_robot * right_hip_to_right_pelvis(&measured_positions.right_leg);
        let right_thigh_to_robot =
            right_hip_to_robot * right_thigh_to_right_hip(&measured_positions.right_leg);
        let right_tibia_to_robot =
            right_thigh_to_robot * right_tibia_to_right_thigh(&measured_positions.right_leg);
        let right_ankle_to_robot =
            right_tibia_to_robot * right_ankle_to_right_tibia(&measured_positions.right_leg);
        let right_foot_to_robot =
            right_ankle_to_robot * right_foot_to_right_ankle(&measured_positions.right_leg);
        let right_sole_to_robot =
            right_foot_to_robot * Isometry3::from(RobotDimensions::RIGHT_FOOT_TO_RIGHT_SOLE);

        let head = RobotHeadKinematics {
            neck_to_robot,
            head_to_robot,
        };

        let torso = RobotTorsoKinematics { torso_to_robot };

        let left_arm = RobotLeftArmKinematics {
            inner_shoulder_to_robot: left_inner_shoulder_to_robot,
            outer_shoulder_to_robot: left_outer_shoulder_to_robot,
            upper_arm_to_robot: left_upper_arm_to_robot,
            forearm_to_robot: left_forearm_to_robot,
        };

        let right_arm = RobotRightArmKinematics {
            inner_shoulder_to_robot: right_inner_shoulder_to_robot,
            outer_shoulder_to_robot: right_outer_shoulder_to_robot,
            upper_arm_to_robot: right_upper_arm_to_robot,
            forearm_to_robot: right_forearm_to_robot,
        };

        let left_leg = RobotLeftLegKinematics {
            pelvis_to_robot: left_pelvis_to_robot,
            hip_to_robot: left_hip_to_robot,
            thigh_to_robot: left_thigh_to_robot,
            tibia_to_robot: left_tibia_to_robot,
            ankle_to_robot: left_ankle_to_robot,
            foot_to_robot: left_foot_to_robot,
            sole_to_robot: left_sole_to_robot,
        };

        let right_leg = RobotRightLegKinematics {
            pelvis_to_robot: right_pelvis_to_robot,
            hip_to_robot: right_hip_to_robot,
            thigh_to_robot: right_thigh_to_robot,
            tibia_to_robot: right_tibia_to_robot,
            ankle_to_robot: right_ankle_to_robot,
            foot_to_robot: right_foot_to_robot,
            sole_to_robot: right_sole_to_robot,
        };

        let robot_kinematics = RobotKinematics {
            head,
            torso,
            left_arm,
            right_arm,
            left_leg,
            right_leg,
        };

        self.last_robot_kinematics = robot_kinematics.clone();

        Ok(MainOutputs {
            robot_kinematics: robot_kinematics.into(),
        })
    }
}

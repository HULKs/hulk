use types::{ArmJoints, HeadJoints, Joints, LegJoints};
use webots::{Motor, PositionSensor, Robot};

use super::interface::SIMULATION_TIME_STEP;

pub struct JointDevice {
    pub motor: Motor,
    pub position_sensor: PositionSensor,
}

impl JointDevice {
    fn new(motor_device_name: &str, position_sensor_device_name: &str) -> Self {
        let motor = Robot::get_motor(motor_device_name);

        let position_sensor = Robot::get_position_sensor(position_sensor_device_name);
        position_sensor.enable(SIMULATION_TIME_STEP);

        Self {
            motor,
            position_sensor,
        }
    }

    pub fn get_position(&self) -> f64 {
        self.position_sensor.get_value()
    }
}

pub struct Head {
    pub yaw: JointDevice,
    pub pitch: JointDevice,
}

impl Head {
    pub fn get_positions(&self) -> HeadJoints {
        HeadJoints {
            yaw: self.yaw.get_position() as f32,
            pitch: self.pitch.get_position() as f32,
        }
    }
}

pub struct Arm {
    pub shoulder_pitch: JointDevice,
    pub shoulder_roll: JointDevice,
    pub elbow_yaw: JointDevice,
    pub elbow_roll: JointDevice,
    pub wrist_yaw: JointDevice,
    pub hand: JointDevice,
}

impl Arm {
    pub fn get_positions(&self) -> ArmJoints {
        ArmJoints {
            shoulder_pitch: self.shoulder_pitch.get_position() as f32,
            shoulder_roll: self.shoulder_roll.get_position() as f32,
            elbow_yaw: self.elbow_yaw.get_position() as f32,
            elbow_roll: self.elbow_roll.get_position() as f32,
            wrist_yaw: self.wrist_yaw.get_position() as f32,
            hand: self.hand.get_position() as f32,
        }
    }
}

pub struct Leg {
    pub hip_yaw_pitch: JointDevice,
    pub hip_roll: JointDevice,
    pub hip_pitch: JointDevice,
    pub knee_pitch: JointDevice,
    pub ankle_pitch: JointDevice,
    pub ankle_roll: JointDevice,
}

impl Leg {
    pub fn get_positions(&self) -> LegJoints {
        LegJoints {
            hip_yaw_pitch: self.hip_yaw_pitch.get_position() as f32,
            hip_roll: self.hip_roll.get_position() as f32,
            hip_pitch: self.hip_pitch.get_position() as f32,
            knee_pitch: self.knee_pitch.get_position() as f32,
            ankle_pitch: self.ankle_pitch.get_position() as f32,
            ankle_roll: self.ankle_roll.get_position() as f32,
        }
    }
}

pub struct JointDevices {
    pub head: Head,
    pub left_arm: Arm,
    pub right_arm: Arm,
    pub left_leg: Leg,
    pub right_leg: Leg,
}

impl JointDevices {
    pub fn get_positions(&self) -> Joints {
        Joints {
            head: self.head.get_positions(),
            left_arm: self.left_arm.get_positions(),
            right_arm: self.right_arm.get_positions(),
            left_leg: self.left_leg.get_positions(),
            right_leg: self.right_leg.get_positions(),
        }
    }
}

impl Default for JointDevices {
    fn default() -> Self {
        Self {
            head: Head {
                yaw: JointDevice::new("HeadYaw", "HeadYaw_sensor"),
                pitch: JointDevice::new("HeadPitch", "HeadPitch_sensor"),
            },
            left_arm: Arm {
                shoulder_pitch: JointDevice::new("LShoulderPitch", "LShoulderPitch_sensor"),
                shoulder_roll: JointDevice::new("LShoulderRoll", "LShoulderRoll_sensor"),
                elbow_yaw: JointDevice::new("LElbowYaw", "LElbowYaw_sensor"),
                elbow_roll: JointDevice::new("LElbowRoll", "LElbowRoll_sensor"),
                wrist_yaw: JointDevice::new("LWristYaw", "LWristYaw_sensor"),
                hand: JointDevice::new("LHand", "LHand_sensor"),
            },
            right_arm: Arm {
                shoulder_pitch: JointDevice::new("RShoulderPitch", "RShoulderPitch_sensor"),
                shoulder_roll: JointDevice::new("RShoulderRoll", "RShoulderRoll_sensor"),
                elbow_yaw: JointDevice::new("RElbowYaw", "RElbowYaw_sensor"),
                elbow_roll: JointDevice::new("RElbowRoll", "RElbowRoll_sensor"),
                wrist_yaw: JointDevice::new("RWristYaw", "RWristYaw_sensor"),
                hand: JointDevice::new("RHand", "RHand_sensor"),
            },
            left_leg: Leg {
                hip_yaw_pitch: JointDevice::new("LHipYawPitch", "LHipYawPitch_sensor"),
                hip_roll: JointDevice::new("LHipRoll", "LHipRoll_sensor"),
                hip_pitch: JointDevice::new("LHipPitch", "LHipPitch_sensor"),
                knee_pitch: JointDevice::new("LKneePitch", "LKneePitch_sensor"),
                ankle_pitch: JointDevice::new("LAnklePitch", "LAnklePitch_sensor"),
                ankle_roll: JointDevice::new("LAnkleRoll", "LAnkleRoll_sensor"),
            },
            right_leg: Leg {
                hip_yaw_pitch: JointDevice::new("RHipYawPitch", "RHipYawPitch_sensor"),
                hip_roll: JointDevice::new("RHipRoll", "RHipRoll_sensor"),
                hip_pitch: JointDevice::new("RHipPitch", "RHipPitch_sensor"),
                knee_pitch: JointDevice::new("RKneePitch", "RKneePitch_sensor"),
                ankle_pitch: JointDevice::new("RAnklePitch", "RAnklePitch_sensor"),
                ankle_roll: JointDevice::new("RAnkleRoll", "RAnkleRoll_sensor"),
            },
        }
    }
}

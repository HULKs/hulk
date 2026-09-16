use serde::{Deserialize, Serialize};

use kinematics::joints::Joints;
use ros_z::Message;

#[derive(Serialize, Deserialize, Message, Clone)]
pub struct MotorCommand {
    pub position: f32,
    pub velocity: f32,
    pub torque: f32,
    pub kp: f32,
    pub kd: f32,
}

impl MotorCommand {
    pub fn zeros() -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            torque: 0.0,
            kp: 0.0,
            kd: 0.0,
        }
    }
}

pub type JointsCommand = Joints<MotorCommand>;

#[derive(Clone, Copy, Serialize, Deserialize, Message, PartialEq)]
pub enum DesiredMode {
    Damping,
    Prepare,
    Custom,
}

#[derive(Serialize, Deserialize, Message)]
pub struct MotionCommand {
    pub desired_mode: DesiredMode,
    pub joints_command: JointsCommand,
}

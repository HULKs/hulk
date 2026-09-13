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

pub type JointsCommand = Joints<MotorCommand>;

#[derive(Clone, Copy, Serialize, Deserialize, Message, PartialEq)]
pub enum DesiredMode {
    Damping,
    Prepare,
    Custom,
}

#[derive(Clone, Copy, Serialize, Deserialize, Message, PartialEq)]
pub enum MotionType {
    Stand,
    Damping,
    Walk,
}

#[derive(Serialize, Deserialize, Message)]
pub struct MotionCommand {
    pub motion_type: MotionType,
    pub joints_command: JointsCommand,
}

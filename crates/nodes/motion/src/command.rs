use kinematics::joints::Joints;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::motor_command::MotorCommand;

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

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

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Message)]
pub enum RobotCommand {
    Damping,
    Prepare,
    Custom { joints_command: JointsCommand },
}

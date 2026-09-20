use color_eyre::eyre::{Result, ensure};
use kinematics::joints::Joints;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{joint_limits::JointLimits, motor_command::MotorCommand};

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

impl RobotCommand {
    pub fn clamp(self, joint_limits: &JointLimits) -> Result<Self> {
        match self {
            Self::Damping | Self::Prepare => Ok(self),
            Self::Custom { mut joints_command } => {
                for (joint, [minimum, maximum]) in joint_limits.position.enumerate() {
                    let command = &mut joints_command[joint];
                    ensure!(
                        command.kp >= 0.0 && command.kd >= 0.0,
                        "negative motor gain for {joint:?}"
                    );
                    ensure!(
                        [
                            command.position,
                            command.velocity,
                            command.torque,
                            command.kp,
                            command.kd
                        ]
                        .into_iter()
                        .all(f32::is_finite),
                        "non-finite motor command for {joint:?}"
                    );
                    command.position = command.position.clamp(minimum, maximum);
                }
                Ok(Self::Custom { joints_command })
            }
        }
    }
}

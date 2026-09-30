//! Position commands with independent joint speed limits.

use booster::MotorState;
use color_eyre::{Result, eyre::ensure};
use kinematics::joints::head::{HeadJoint, HeadJoints};
use types::{joint_limits::JointLimits, motor_command::MotorCommand};

use crate::parameters::JointControlParameters;

#[derive(Clone, Copy)]
pub(crate) struct HeadObservation {
    pub(crate) positions: HeadJoints<f32>,
}

impl From<HeadJoints<MotorState>> for HeadObservation {
    fn from(head: HeadJoints<MotorState>) -> Self {
        Self {
            positions: HeadJoints {
                yaw: head.yaw.position,
                pitch: head.pitch.position,
            },
        }
    }
}

impl HeadObservation {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.positions.into_iter().all(f32::is_finite),
            "head observation contains non-finite positions"
        );
        Ok(())
    }
}

pub(crate) enum JointTarget {
    MoveTo {
        position: HeadJoints<f32>,
        travel_speed: HeadJoints<f32>,
    },
    MoveWithVelocity {
        velocity: HeadJoints<f32>,
    },
    Damping,
}

impl JointTarget {
    pub(crate) fn motor_commands(
        self,
        start_position: HeadJoints<f32>,
        elapsed: f32,
        parameters: &JointControlParameters,
        joint_limits: &JointLimits,
    ) -> HeadJoints<MotorCommand> {
        let mut commands = HeadJoints::fill(MotorCommand::zeros());
        for joint in [HeadJoint::Yaw, HeadJoint::Pitch] {
            let [minimum, maximum] = joint_limits.position.head[joint];
            let start = start_position[joint].clamp(minimum, maximum);
            let (position, kp, kd) = match self {
                Self::MoveTo {
                    position,
                    travel_speed,
                } => {
                    let goal = position[joint].clamp(minimum, maximum);
                    let step =
                        travel_speed[joint].min(parameters.maximum_velocity[joint]) * elapsed;
                    (
                        (start + (goal - start).clamp(-step, step)).clamp(minimum, maximum),
                        parameters.kp[joint],
                        parameters.kd[joint],
                    )
                }
                Self::MoveWithVelocity { velocity } => {
                    let maximum_velocity = parameters.maximum_velocity[joint];
                    let velocity = velocity[joint].clamp(-maximum_velocity, maximum_velocity);
                    (
                        (start + velocity * elapsed).clamp(minimum, maximum),
                        parameters.kp[joint],
                        parameters.kd[joint],
                    )
                }
                Self::Damping => (start, 0.0, parameters.damping_kd[joint]),
            };
            commands[joint] = MotorCommand {
                position,
                kp,
                kd,
                ..MotorCommand::zeros()
            };
        }
        commands
    }
}

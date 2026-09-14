//! K1 joint constraints, command continuity, and tracking/damping motor commands.

use std::time::Duration;

use kinematics::joints::head::HeadJoints;
use types::robot_command::MotorCommand;

use crate::parameters::Parameters;

#[derive(Debug, Clone, Copy)]
pub struct HeadObservation {
    pub positions: HeadJoints<f32>,
    pub velocities: HeadJoints<f32>,
}

#[derive(Debug, Clone, Copy)]
pub enum JointTarget {
    Position(HeadJoints<f32>),
    Damping,
}

#[derive(Debug, Clone, Copy)]
pub struct MotionProgress {
    pub effective_target: HeadJoints<f32>,
    pub target_reached: bool,
    pub constrained: bool,
}

pub struct JointControlOutput {
    pub commands: HeadJoints<MotorCommand>,
    /// Position-target progress; damping has no position target.
    pub progress: Option<MotionProgress>,
}

/// State storage will be added with the trajectory and motor-command implementation.
#[derive(Default)]
pub struct JointController;

impl JointController {
    pub fn update(
        &mut self,
        _target: JointTarget,
        _observation: &HeadObservation,
        _parameters: &Parameters,
        _now: Duration,
    ) -> JointControlOutput {
        todo!("constrain joint motion and generate tracking or damping motor commands")
    }

    pub fn reset(&mut self, _observation: &HeadObservation, _now: Duration) {
        todo!("seed command continuity from the current measured head state")
    }
}

//! Coordinates head requests, observations, patterns, geometry, and joint control.
//! Controller operations are scaffolding and are not called by the node yet.

use std::time::Duration;

use kinematics::joints::head::HeadJoints;
use types::{motion_command::HeadMotion, robot_command::MotorCommand};

use crate::{
    joint_control::{HeadObservation, JointController, MotionProgress},
    look_at::GazeGeometry,
    parameters::Parameters,
    patterns::{GlanceState, ScanState},
};

#[derive(Default)]
pub struct HeadController {
    observation: Option<HeadObservation>,
    observation_time: Option<Duration>,
    last_request: Option<HeadMotion>,
    progress: Option<MotionProgress>,
    scan: ScanState,
    glance: GlanceState,
    joint_control: JointController,
}

impl HeadController {
    /// Receives each head observation extracted from LowState.
    /// Times use a common robot-clock origin, including when running in simulation.
    pub fn observe(&mut self, _observation: HeadObservation, _now: Duration) {
        todo!("update measured head state from incoming observations")
    }

    /// Evaluates a request without restarting an unchanged scan or glance mode.
    /// Geometry is optional so joint-space motions do not depend on camera input.
    pub fn evaluate(
        &mut self,
        _request: &HeadMotion,
        _geometry: Option<&GazeGeometry<'_>>,
        _parameters: &Parameters,
        _now: Duration,
    ) -> HeadJoints<MotorCommand> {
        todo!("resolve the request, handle missing inputs, and coordinate head motion modules")
    }

    /// Reseeds controller state from measurements at startup or reactivation.
    pub fn reset(&mut self, _observation: HeadObservation, _now: Duration) {
        todo!("reset pattern and joint-control state")
    }
}

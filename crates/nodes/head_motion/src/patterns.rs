//! Stateful target sequences for scanning and glancing, without motor commands.

use coordinate_systems::Ground;
use kinematics::joints::head::HeadJoints;
use linear_algebra::Point3;
use ros_z::time::Time;

use crate::{joint_control::MotionProgress, parameters::Parameters};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanKind {
    LookAround,
    SearchForLostBall,
}

/// State storage will be added with the moving/dwelling scan implementation.
#[derive(Default)]
pub struct ScanState;

impl ScanState {
    /// Advances using arrival at the effective target, dwell time, and travel timeout.
    /// Repeating a scan kind preserves its phase.
    pub fn update(
        &mut self,
        _kind: ScanKind,
        _progress: Option<&MotionProgress>,
        _parameters: &Parameters,
        _now: Time,
    ) -> HeadJoints<f32> {
        todo!("advance scan phase and select its next joint target")
    }

    pub fn reset(&mut self) {
        todo!("reset scan state on leaving the mode or reactivation")
    }
}

/// State storage will be added with the alternating-target implementation.
#[derive(Default)]
pub struct GlanceState;

impl GlanceState {
    /// Returns a spatial target for look_at; target changes preserve glance phase.
    pub fn update(
        &mut self,
        _target: Point3<Ground>,
        _parameters: &Parameters,
        _now: Time,
    ) -> Point3<Ground> {
        todo!("advance glance phase and offset the spatial target")
    }

    pub fn reset(&mut self) {
        todo!("reset glance phase when the mode is left")
    }
}

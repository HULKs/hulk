//! Structured constraint reporting, independent of trajectory generation and logging.

use kinematics::joints::head::HeadJoint;

use super::{
    HeadObservation, KinematicState,
    trajectory::{AxisStep, Limits},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constraint {
    Position,
    Velocity,
    Acceleration,
    Jerk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintCause {
    TargetClipped,
    MeasuredOutsideBounds,
    ReferenceAtBound,
    /// Position bounds take priority over derivative continuity in recovery.
    PositionRecovery,
}

#[derive(Debug, Clone, Copy)]
pub struct ConstraintDiagnostic {
    pub joint: HeadJoint,
    pub constraint: Constraint,
    pub cause: ConstraintCause,
    pub value: f64,
    pub effective_value: f64,
    pub bounds: [f64; 2],
}

pub(super) fn record_measurement(
    diagnostics: &mut Vec<ConstraintDiagnostic>,
    joint: HeadJoint,
    observation: &HeadObservation,
    limits: Limits,
) {
    for (constraint, value, bounds) in [
        (
            Constraint::Position,
            f64::from(observation.positions[joint]),
            limits.position,
        ),
        (
            Constraint::Velocity,
            f64::from(observation.velocities[joint]),
            [-limits.velocity, limits.velocity],
        ),
    ] {
        if value < bounds[0] || value > bounds[1] {
            diagnostics.push(ConstraintDiagnostic {
                joint,
                constraint,
                cause: ConstraintCause::MeasuredOutsideBounds,
                value,
                effective_value: value.clamp(bounds[0], bounds[1]),
                bounds,
            });
        }
    }
}

pub(super) fn record_tracking(
    diagnostics: &mut Vec<ConstraintDiagnostic>,
    joint: HeadJoint,
    requested: f32,
    start: KinematicState,
    step: &AxisStep,
    limits: Limits,
) {
    if step.effective_target != requested {
        diagnostics.push(ConstraintDiagnostic {
            joint,
            constraint: Constraint::Position,
            cause: ConstraintCause::TargetClipped,
            value: requested.into(),
            effective_value: step.effective_target.into(),
            bounds: limits.position,
        });
    }
    if step.recovered {
        diagnostics.push(ConstraintDiagnostic {
            joint,
            constraint: Constraint::Position,
            cause: ConstraintCause::PositionRecovery,
            value: start.position,
            effective_value: step.reference.position,
            bounds: limits.position,
        });
    }
    record_reference(diagnostics, joint, step.reference, limits);
}

fn record_reference(
    diagnostics: &mut Vec<ConstraintDiagnostic>,
    joint: HeadJoint,
    state: KinematicState,
    limits: Limits,
) {
    for (constraint, value, bounds) in [
        (Constraint::Position, state.position, limits.position),
        (
            Constraint::Velocity,
            state.velocity,
            [-limits.velocity, limits.velocity],
        ),
        (
            Constraint::Acceleration,
            state.acceleration,
            [-limits.acceleration, limits.acceleration],
        ),
        (Constraint::Jerk, state.jerk, [-limits.jerk, limits.jerk]),
    ] {
        let epsilon = 1e-6 * (bounds[1] - bounds[0]).max(1.0);
        if value <= bounds[0] + epsilon || value >= bounds[1] - epsilon {
            diagnostics.push(ConstraintDiagnostic {
                joint,
                constraint,
                cause: ConstraintCause::ReferenceAtBound,
                value,
                effective_value: value,
                bounds,
            });
        }
    }
}

//! Joint trajectories, tracking/damping commands, and measured arrival diagnostics.

use std::time::Duration;

use color_eyre::{
    Result,
    eyre::{WrapErr, ensure, eyre},
};
use kinematics::joints::head::{HeadJoint, HeadJoints};
use ros_z::time::Time;
use types::{joint_limits::JointLimits, robot_command::MotorCommand};

use crate::parameters::JointControlParameters;
pub use diagnostics::{Constraint, ConstraintCause, ConstraintDiagnostic};
use diagnostics::{record_measurement, record_tracking};
pub use trajectory::KinematicState;
use trajectory::{AxisGenerator, Limits};

mod diagnostics;
mod trajectory;

const JOINTS: [HeadJoint; 2] = [HeadJoint::Yaw, HeadJoint::Pitch];

#[derive(Debug, Clone, Copy)]
pub struct HeadObservation {
    pub positions: HeadJoints<f32>,
    pub velocities: HeadJoints<f32>,
}

impl HeadObservation {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.positions
                .into_iter()
                .chain(self.velocities)
                .all(f32::is_finite),
            "head observation contains non-finite values"
        );
        Ok(())
    }

    fn reference(&self) -> HeadJoints<KinematicState> {
        let mut reference = HeadJoints::default();
        for joint in JOINTS {
            reference[joint] = KinematicState {
                position: self.positions[joint].into(),
                velocity: self.velocities[joint].into(),
                ..Default::default()
            };
        }
        reference
    }
}

#[derive(Debug, Clone, Copy)]
pub enum JointTarget {
    Position(HeadJoints<f32>),
    /// Travel toward a position at the requested positive joint speeds (rad/s),
    /// arriving at rest. Short segments may finish before reaching cruise speed.
    /// Lowering travel speed preserves the current derivatives while braking.
    MoveTo {
        position: HeadJoints<f32>,
        travel_speed: HeadJoints<f32>,
    },
    Damping,
}

#[derive(Debug, Clone, Copy)]
pub struct MotionProgress {
    /// Identifies the request that produced this feedback, before position clipping.
    pub requested_target: HeadJoints<f32>,
    /// Constrained final goal, not the intermediate trajectory reference.
    pub effective_target: HeadJoints<f32>,
    /// Measured position is within entry tolerances, independent of velocity.
    /// Glancing uses this to reverse while following a moving target.
    pub position_reached: bool,
    /// Measured position and velocity satisfy arrival tolerances, with hysteresis.
    pub target_reached: bool,
    /// Whether the requested position was clipped; speed clipping is a diagnostic.
    pub constrained: bool,
}

pub struct JointControlOutput {
    pub commands: HeadJoints<MotorCommand>,
    pub reference: HeadJoints<KinematicState>,
    /// Damping has no position target. Patterns own dwell timing.
    pub progress: Option<MotionProgress>,
    pub diagnostics: Vec<ConstraintDiagnostic>,
    /// A tracking reference was initialized; damping has no trajectory to reseed.
    pub reseeded: bool,
}

#[derive(Default)]
pub struct JointController {
    generators: HeadJoints<AxisGenerator>,
    reference: Option<HeadJoints<KinematicState>>,
    last_update: Option<Time>,
    previous_target: Option<HeadJoints<f32>>,
    arrived: bool,
}

impl JointController {
    /// Called only for requested outputs; observations themselves don't advance motion.
    /// Parameters and observations must be from a consistent snapshot for this request.
    pub fn update(
        &mut self,
        target: JointTarget,
        observation: &HeadObservation,
        parameters: &JointControlParameters,
        joints: &JointLimits,
        now: Time,
    ) -> Result<JointControlOutput> {
        observation.validate()?;
        parameters.validate().map_err(|reason| eyre!(reason))?;
        joints.validate().map_err(|reason| eyre!(reason))?;
        match target {
            JointTarget::Position(requested) => {
                self.track(requested, None, observation, parameters, joints, now)
            }
            JointTarget::MoveTo {
                position,
                travel_speed,
            } => {
                ensure!(
                    travel_speed
                        .into_iter()
                        .all(|speed| speed.is_finite() && speed > 0.0),
                    "head travel speeds must be finite and positive"
                );
                self.track(
                    position,
                    Some(travel_speed),
                    observation,
                    parameters,
                    joints,
                    now,
                )
            }
            JointTarget::Damping => Ok(self.damp(observation, parameters, joints)),
        }
    }

    fn track(
        &mut self,
        requested: HeadJoints<f32>,
        travel_speed: Option<HeadJoints<f32>>,
        observation: &HeadObservation,
        parameters: &JointControlParameters,
        joints: &JointLimits,
        now: Time,
    ) -> Result<JointControlOutput> {
        ensure!(
            requested.into_iter().all(f32::is_finite),
            "head target contains non-finite values"
        );
        let TrackingStart {
            mut reference,
            elapsed,
            reseeded,
        } = self.tracking_start(observation, now, parameters.reseed_after);
        let mut effective_target = HeadJoints::default();
        let mut diagnostics = Vec::new();

        for joint in JOINTS {
            let limits = limits_for(joint, parameters, joints);
            record_measurement(&mut diagnostics, joint, observation, limits);
            let planning_limits = planning_limits(
                joint,
                limits,
                travel_speed.map(|speed| speed[joint]),
                &mut diagnostics,
            );
            let step = self.generators[joint]
                .step(
                    reference[joint],
                    requested[joint],
                    observation.positions[joint],
                    planning_limits,
                    elapsed,
                )
                .wrap_err_with(|| {
                    format!(
                        "failed to plan head joint {joint:?}: start={:?}, requested_target={}, \
                         measured_position={}, limits={planning_limits:?}, elapsed_seconds={elapsed}",
                        reference[joint], requested[joint], observation.positions[joint],
                    )
                })?;
            record_tracking(
                &mut diagnostics,
                joint,
                requested[joint],
                reference[joint],
                &step,
                limits,
            );
            reference[joint] = step.reference;
            effective_target[joint] = step.effective_target;
        }

        // Commit controller state only after both joints produced valid trajectories.
        let target_reached =
            self.update_arrival(effective_target, observation, parameters, reseeded);
        self.reference = Some(reference);
        self.last_update = Some(now);
        self.previous_target = Some(effective_target);

        Ok(JointControlOutput {
            commands: motor_commands(reference, parameters.kp, parameters.kd),
            reference,
            progress: Some(MotionProgress {
                requested_target: requested,
                effective_target,
                position_reached: position_within_tolerance(
                    effective_target,
                    observation,
                    parameters,
                    1.0,
                ),
                target_reached,
                constrained: effective_target != requested,
            }),
            diagnostics,
            reseeded,
        })
    }

    fn damp(
        &mut self,
        observation: &HeadObservation,
        parameters: &JointControlParameters,
        joints: &JointLimits,
    ) -> JointControlOutput {
        let mut reference = HeadJoints::default();
        let mut diagnostics = Vec::new();
        for joint in JOINTS {
            let limits = limits_for(joint, parameters, joints);
            record_measurement(&mut diagnostics, joint, observation, limits);
            reference[joint] = KinematicState {
                position: f64::from(observation.positions[joint])
                    .clamp(limits.position[0], limits.position[1]),
                ..Default::default()
            };
        }

        self.reference = None;
        self.last_update = None;
        self.previous_target = None;
        self.arrived = false;
        JointControlOutput {
            commands: motor_commands(reference, HeadJoints::fill(0.0), parameters.damping_kd),
            reference,
            progress: None,
            diagnostics,
            reseeded: false,
        }
    }

    fn tracking_start(
        &self,
        observation: &HeadObservation,
        now: Time,
        reseed_after: Duration,
    ) -> TrackingStart {
        if let (Some(reference), Some(last_update)) = (self.reference, self.last_update)
            && now >= last_update
            && now.duration_since(last_update) <= reseed_after
        {
            return TrackingStart {
                reference,
                elapsed: now.duration_since(last_update).as_secs_f64(),
                reseeded: false,
            };
        }
        TrackingStart {
            reference: observation.reference(),
            elapsed: 0.0,
            reseeded: true,
        }
    }

    fn update_arrival(
        &mut self,
        target: HeadJoints<f32>,
        observation: &HeadObservation,
        parameters: &JointControlParameters,
        reseeded: bool,
    ) -> bool {
        let keep_arrival = !reseeded && self.arrived && self.previous_target == Some(target);
        let factor = if keep_arrival {
            parameters.arrival_exit_factor
        } else {
            1.0
        };
        self.arrived = position_within_tolerance(target, observation, parameters, factor)
            && JOINTS.into_iter().all(|joint| {
                observation.velocities[joint].abs() <= parameters.velocity_tolerance[joint] * factor
            });
        self.arrived
    }

    pub fn reset(&mut self, observation: &HeadObservation, now: Time) -> Result<()> {
        observation.validate()?;
        self.reference = Some(observation.reference());
        self.last_update = Some(now);
        self.previous_target = None;
        self.arrived = false;
        Ok(())
    }
}

fn position_within_tolerance(
    target: HeadJoints<f32>,
    observation: &HeadObservation,
    parameters: &JointControlParameters,
    factor: f32,
) -> bool {
    JOINTS.into_iter().all(|joint| {
        (observation.positions[joint] - target[joint]).abs()
            <= parameters.position_tolerance[joint] * factor
    })
}

fn planning_limits(
    joint: HeadJoint,
    safety_limits: Limits,
    travel_speed: Option<f32>,
    diagnostics: &mut Vec<ConstraintDiagnostic>,
) -> Limits {
    let Some(speed) = travel_speed.map(f64::from) else {
        return safety_limits;
    };
    if speed > safety_limits.velocity {
        diagnostics.push(ConstraintDiagnostic {
            joint,
            constraint: Constraint::Velocity,
            cause: ConstraintCause::TargetClipped,
            value: speed,
            effective_value: safety_limits.velocity,
            bounds: [0.0, safety_limits.velocity],
        });
    }
    Limits {
        // Ruckig's position-mode max_velocity sets cruise speed; target_velocity
        // is the arrival velocity and remains zero. Safety limits are independent.
        velocity: speed.min(safety_limits.velocity),
        ..safety_limits
    }
}

struct TrackingStart {
    reference: HeadJoints<KinematicState>,
    elapsed: f64,
    reseeded: bool,
}

fn limits_for(
    joint: HeadJoint,
    parameters: &JointControlParameters,
    joints: &JointLimits,
) -> Limits {
    Limits {
        position: joints.position.head[joint].map(f64::from),
        velocity: parameters.maximum_velocity[joint].into(),
        acceleration: parameters.maximum_acceleration[joint].into(),
        jerk: parameters.maximum_jerk[joint].into(),
    }
}

fn motor_commands(
    reference: HeadJoints<KinematicState>,
    kp: HeadJoints<f32>,
    kd: HeadJoints<f32>,
) -> HeadJoints<MotorCommand> {
    let command = |joint| MotorCommand {
        position: reference[joint].position as f32,
        velocity: reference[joint].velocity as f32,
        torque: 0.0,
        kp: kp[joint],
        kd: kd[joint],
    };
    HeadJoints {
        yaw: command(HeadJoint::Yaw),
        pitch: command(HeadJoint::Pitch),
    }
}

#[cfg(test)]
mod tests;

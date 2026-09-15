//! Constraint and pattern-deadline logging with independent warning throttling.

use std::{mem::take, time::Duration};

use color_eyre::Report;
use ros_z::time::Time;
use tracing::warn;
use types::motion_command::HeadMotion;

use crate::{
    head::{HeadOutput, HoldReason},
    joint_control::{ConstraintCause, ConstraintDiagnostic, HeadObservation, JointControlOutput},
    parameters::JointControlParameters,
    patterns::{GlanceTimeout, ScanTimeout},
};

#[derive(Debug, Clone, Copy)]
pub enum FailureKind {
    Observation,
    Request,
    Response,
}

/// Owns all runtime logging; independent failure categories cannot suppress each other.
#[derive(Default)]
pub struct NodeLogger {
    constraints: ConstraintLogger,
    patterns: PatternLogger,
    hold_reason: Option<HoldReason>,
    hold: WarningThrottle,
    failures: [WarningThrottle; 3],
}

impl NodeLogger {
    pub fn log_output(
        &mut self,
        request: &HeadMotion,
        output: &HeadOutput,
        parameters: &JointControlParameters,
        now: Time,
    ) {
        self.constraints.log(
            request,
            &output.observation,
            &output.joint_control,
            parameters,
            now,
        );
        self.patterns.log_scan(
            output.scan_timeout.as_ref(),
            &output.observation,
            parameters.warning_interval,
            now,
        );
        self.patterns.log_glance(
            output.glance_timeout.as_ref(),
            &output.observation,
            parameters.warning_interval,
            now,
        );
        if output.hold_reason != self.hold_reason {
            self.hold = WarningThrottle::default();
            self.hold_reason = output.hold_reason;
        }
        let suppressed = self.hold.warning(
            output.hold_reason.is_some(),
            parameters.warning_interval,
            now,
        );
        if let (Some(reason), Some(suppressed)) = (output.hold_reason, suppressed) {
            warn!(?request, ?reason, suppressed, injected = output.injected,
                measured_position_rad = ?output.observation.positions,
                measured_velocity_rad_s = ?output.observation.velocities,
                progress = ?output.joint_control.progress,
                action = "hold_captured_reference", "head gaze geometry unavailable");
        }
    }

    pub fn log_error(
        &mut self,
        kind: FailureKind,
        request: Option<&HeadMotion>,
        error: &Report,
        warning_interval: Duration,
        now: Time,
    ) {
        if let Some(suppressed) = self.failures[kind as usize].warning(true, warning_interval, now)
        {
            let action = match kind {
                FailureKind::Observation => "invalidate_measurement",
                FailureKind::Request => "omit_command_reply",
                FailureKind::Response => "continue_serving_requests",
            };
            warn!(?kind, ?request, error = %format_args!("{error:#}"), suppressed, action,
                "head motion service failure");
        }
    }
}

/// Logs constraint episodes from the pure joint controller. Call once for each evaluated
/// output, including unconstrained outputs so completed episodes are cleared.
#[derive(Default)]
pub struct ConstraintLogger {
    episodes: Vec<ConstraintEpisode>,
    last_log: Option<Time>,
}

struct ConstraintEpisode {
    diagnostic: ConstraintDiagnostic,
    started: Time,
    last_warning: Time,
    suppressed: usize,
    maximum_excess: f64,
}

impl ConstraintLogger {
    pub fn log(
        &mut self,
        request: &HeadMotion,
        observation: &HeadObservation,
        output: &JointControlOutput,
        parameters: &JointControlParameters,
        now: Time,
    ) {
        if self.last_log.is_some_and(|previous| now < previous) {
            self.episodes.clear();
        }
        self.last_log = Some(now);
        self.episodes.retain(|episode| {
            output
                .diagnostics
                .iter()
                .any(|diagnostic| same_constraint_episode(&episode.diagnostic, diagnostic))
        });
        for diagnostic in &output.diagnostics {
            let excess = (diagnostic.bounds[0] - diagnostic.value)
                .max(diagnostic.value - diagnostic.bounds[1])
                .max(0.0);
            let (duration, suppressed, maximum_excess) = match self
                .episodes
                .iter_mut()
                .find(|episode| same_constraint_episode(&episode.diagnostic, diagnostic))
            {
                Some(episode) => {
                    episode.maximum_excess = episode.maximum_excess.max(excess);
                    if now.duration_since(episode.last_warning) < parameters.warning_interval {
                        episode.suppressed += 1;
                        continue;
                    }
                    episode.last_warning = now;
                    let suppressed = take(&mut episode.suppressed);
                    (
                        now.duration_since(episode.started),
                        suppressed,
                        episode.maximum_excess,
                    )
                }
                None => {
                    self.episodes.push(ConstraintEpisode {
                        diagnostic: *diagnostic,
                        started: now,
                        last_warning: now,
                        suppressed: 0,
                        maximum_excess: excess,
                    });
                    (Duration::ZERO, 0, excess)
                }
            };
            let action = match diagnostic.cause {
                ConstraintCause::TargetClipped => "track_constrained_target",
                ConstraintCause::MeasuredOutsideBounds => "report_measured_violation",
                ConstraintCause::ReferenceAtBound => "follow_bounded_trajectory",
                ConstraintCause::PositionRecovery => {
                    "reseed_at_bounded_position_with_zero_derivatives"
                }
            };
            let joint = diagnostic.joint;
            warn!(
                ?request, ?joint, constraint = ?diagnostic.constraint, cause = ?diagnostic.cause,
                value = diagnostic.value, effective_value = diagnostic.effective_value,
                bounds = ?diagnostic.bounds, maximum_excess, action,
                measured_position_rad = observation.positions[joint],
                measured_velocity_rad_s = observation.velocities[joint],
                reference = ?output.reference[joint], progress = ?output.progress,
                reseeded = output.reseeded, episode_seconds = duration.as_secs_f64(), suppressed,
                "head joint constraint active (SI units: rad, rad/s, rad/s^2, rad/s^3)"
            );
        }
    }
}

fn same_constraint_episode(left: &ConstraintDiagnostic, right: &ConstraintDiagnostic) -> bool {
    left.joint == right.joint
        && left.constraint == right.constraint
        && left.cause == right.cause
        && left.bounds == right.bounds
        && (left.value < (left.bounds[0] + left.bounds[1]) / 2.0)
            == (right.value < (right.bounds[0] + right.bounds[1]) / 2.0)
}

/// Logs pattern deadlines. Call the corresponding method for every evaluated output.
#[derive(Default)]
pub struct PatternLogger {
    throttle: WarningThrottle,
}

#[derive(Default)]
struct WarningThrottle {
    last_update: Option<Time>,
    last_warning: Option<Time>,
    suppressed: usize,
}

impl PatternLogger {
    pub fn log_scan(
        &mut self,
        timeout: Option<&ScanTimeout>,
        observation: &HeadObservation,
        warning_interval: Duration,
        now: Time,
    ) {
        let suppressed = self
            .throttle
            .warning(timeout.is_some(), warning_interval, now);
        let (Some(timeout), Some(suppressed)) = (timeout, suppressed) else {
            return;
        };
        warn!(
            kind = ?timeout.kind, waypoint = ?timeout.waypoint,
            requested_target = ?timeout.requested_target, progress = ?timeout.progress,
            measured_position_rad = ?observation.positions,
            measured_velocity_rad_s = ?observation.velocities,
            elapsed_seconds = timeout.elapsed.as_secs_f64(),
            ever_reached = timeout.ever_reached, suppressed,
            action = "advance_to_next_waypoint", "head scan waypoint deadline reached"
        );
    }

    pub fn log_glance(
        &mut self,
        timeout: Option<&GlanceTimeout>,
        observation: &HeadObservation,
        warning_interval: Duration,
        now: Time,
    ) {
        let suppressed = self
            .throttle
            .warning(timeout.is_some(), warning_interval, now);
        let (Some(timeout), Some(suppressed)) = (timeout, suppressed) else {
            return;
        };
        warn!(
            side = ?timeout.side, ground_target = ?timeout.target, progress = ?timeout.progress,
            measured_position_rad = ?observation.positions,
            measured_velocity_rad_s = ?observation.velocities,
            tracking_seconds = timeout.elapsed.as_secs_f64(), suppressed,
            action = "switch_glance_side", "head glance phase deadline reached"
        );
    }
}

impl WarningThrottle {
    fn warning(&mut self, active: bool, warning_interval: Duration, now: Time) -> Option<usize> {
        if self.last_update.is_some_and(|previous| now < previous) {
            self.last_warning = None;
            self.suppressed = 0;
        }
        self.last_update = Some(now);
        if !active {
            return None;
        }
        if self
            .last_warning
            .is_some_and(|previous| now.duration_since(previous) < warning_interval)
        {
            self.suppressed += 1;
            return None;
        }
        self.last_warning = Some(now);
        Some(take(&mut self.suppressed))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use json5::from_str;
    use kinematics::joints::head::{HeadJoint, HeadJoints};
    use types::robot_command::MotorCommand;

    use super::*;
    use crate::joint_control::{
        Constraint, ConstraintCause, ConstraintDiagnostic, HeadObservation, JointControlOutput,
    };
    use crate::parameters::Parameters;

    fn fixture() -> (Parameters, HeadObservation, JointControlOutput) {
        let parameters = from_str(include_str!(
            "../../../../etc/parameters/base/head_motion.json5"
        ))
        .unwrap();
        let observation = HeadObservation {
            positions: HeadJoints::fill(0.0),
            velocities: HeadJoints::fill(0.0),
        };
        let output = JointControlOutput {
            commands: HeadJoints::fill(MotorCommand {
                position: 0.0,
                velocity: 0.0,
                torque: 0.0,
                kp: 0.0,
                kd: 0.0,
            }),
            reference: HeadJoints::default(),
            progress: None,
            diagnostics: vec![ConstraintDiagnostic {
                joint: HeadJoint::Yaw,
                constraint: Constraint::Position,
                cause: ConstraintCause::TargetClipped,
                value: 2.0,
                effective_value: 1.0,
                bounds: [-1.0, 1.0],
            }],
            reseeded: true,
        };
        (parameters, observation, output)
    }

    #[test]
    fn holding_keeps_throttling_across_reseeds_but_recovery_ends_the_episode() {
        let (parameters, observation, joint_control) = fixture();
        let mut output = HeadOutput {
            observation,
            joint_control,
            hold_reason: Some(HoldReason::MissingGeometry),
            scan_timeout: None,
            glance_timeout: None,
            injected: false,
        };
        let mut logger = NodeLogger::default();
        for millis in [0, 200, 400] {
            logger.log_output(
                &HeadMotion::ZeroAngles,
                &output,
                &parameters.joint_control,
                Time::zero() + Duration::from_millis(millis),
            );
        }
        assert_eq!(logger.hold.last_warning, Some(Time::zero()));
        assert_eq!(logger.hold.suppressed, 2);

        output.hold_reason = None;
        logger.log_output(
            &HeadMotion::ZeroAngles,
            &output,
            &parameters.joint_control,
            Time::zero() + Duration::from_millis(500),
        );
        output.hold_reason = Some(HoldReason::MissingGeometry);
        let restart = Time::zero() + Duration::from_millis(600);
        logger.log_output(
            &HeadMotion::ZeroAngles,
            &output,
            &parameters.joint_control,
            restart,
        );
        assert_eq!(logger.hold.last_warning, Some(restart));
        assert_eq!(logger.hold.suppressed, 0);
    }

    #[test]
    fn persistent_constraints_keep_throttling_and_statistics_across_reseeds() {
        let (mut parameters, observation, mut output) = fixture();
        parameters.joint_control.warning_interval = Duration::from_secs(1);
        let mut logger = ConstraintLogger::default();
        for millis in [0, 200, 400, 600, 800] {
            logger.log(
                &HeadMotion::ZeroAngles,
                &observation,
                &output,
                &parameters.joint_control,
                Time::zero() + Duration::from_millis(millis),
            );
            // A temporary larger violation belongs to the same episode.
            output.diagnostics[0].value = if millis == 200 { 3.0 } else { 2.0 };
        }
        assert_eq!(logger.episodes.len(), 1);
        let episode = &logger.episodes[0];
        assert_eq!(episode.last_warning, Time::zero());
        assert_eq!(episode.suppressed, 4);
        assert_eq!(episode.maximum_excess, 2.0);

        logger.log(
            &HeadMotion::ZeroAngles,
            &observation,
            &output,
            &parameters.joint_control,
            Time::zero() + Duration::from_secs(1),
        );
        let episode = &logger.episodes[0];
        assert_eq!(episode.started, Time::zero());
        assert_eq!(episode.last_warning, Time::zero() + Duration::from_secs(1));
        assert_eq!(episode.suppressed, 0);
        assert_eq!(episode.maximum_excess, 2.0);

        output.diagnostics.clear();
        logger.log(
            &HeadMotion::ZeroAngles,
            &observation,
            &output,
            &parameters.joint_control,
            Time::zero() + Duration::from_millis(1200),
        );
        assert!(logger.episodes.is_empty());
    }

    #[test]
    fn clock_rollback_after_suppressed_warnings_starts_a_new_episode() {
        let (mut parameters, observation, mut output) = fixture();
        parameters.joint_control.warning_interval = Duration::from_secs(1);
        output.reseeded = false;
        let mut logger = ConstraintLogger::default();
        // Rollback remains later than the last warning (0 ms), but precedes the last call.
        for millis in [0, 200, 400, 300] {
            logger.log(
                &HeadMotion::ZeroAngles,
                &observation,
                &output,
                &parameters.joint_control,
                Time::zero() + Duration::from_millis(millis),
            );
        }
        assert_eq!(logger.episodes.len(), 1);
        let episode = &logger.episodes[0];
        assert_eq!(episode.started, Time::zero() + Duration::from_millis(300));
        assert_eq!(
            episode.last_warning,
            Time::zero() + Duration::from_millis(300)
        );
        assert_eq!(episode.suppressed, 0);
    }
}

//! Throttled service failures and look-at holds.

use std::time::Duration;

use color_eyre::Report;
use ros_z::time::Time;
use tracing::warn;
use types::motion_command::HeadMotion;

use crate::head::{HeadOutput, HoldReason};

#[derive(Debug, Clone, Copy)]
pub(crate) enum FailureKind {
    Observation,
    Request,
    Response,
    JointLimits,
}

#[derive(Default)]
pub(crate) struct NodeLogger {
    hold_reason: Option<HoldReason>,
    hold: WarningThrottle,
    observation: WarningThrottle,
    request: WarningThrottle,
    response: WarningThrottle,
    joint_limits: WarningThrottle,
}

impl NodeLogger {
    pub(crate) fn log_output(
        &mut self,
        request: &HeadMotion,
        output: &HeadOutput,
        warning_interval: Duration,
        now: Time,
    ) {
        let Some(reason) = output.hold_reason else {
            return;
        };
        // Holds may alternate with solved requests every cycle, so only a new reason resets.
        if self.hold_reason != Some(reason) {
            self.hold = WarningThrottle::default();
            self.hold_reason = Some(reason);
        }
        if self.hold.ready(warning_interval, now) {
            warn!(
                ?request,
                ?reason,
                "head look-at target unavailable; holding position"
            );
        }
    }

    pub(crate) fn log_error(
        &mut self,
        kind: FailureKind,
        request: Option<&HeadMotion>,
        error: &Report,
        warning_interval: Duration,
        now: Time,
    ) {
        let throttle = match kind {
            FailureKind::Observation => &mut self.observation,
            FailureKind::Request => &mut self.request,
            FailureKind::Response => &mut self.response,
            FailureKind::JointLimits => &mut self.joint_limits,
        };
        if throttle.ready(warning_interval, now) {
            warn!(?kind, ?request, error = %format_args!("{error:#}"),
                "head motion service failure");
        }
    }
}

#[derive(Default)]
struct WarningThrottle {
    last_update: Option<Time>,
    last_warning: Option<Time>,
}

impl WarningThrottle {
    fn ready(&mut self, interval: Duration, now: Time) -> bool {
        if self.last_update.is_some_and(|last| now < last) {
            self.last_warning = None;
        }
        self.last_update = Some(now);
        if self
            .last_warning
            .is_some_and(|last| now.duration_since(last) < interval)
        {
            return false;
        }
        self.last_warning = Some(now);
        true
    }
}

//! Joint-target sequences for localization and lost-ball scanning.

use std::time::Duration;

use color_eyre::{Result, eyre::eyre};
use kinematics::joints::head::HeadJoints;
use ros_z::time::Time;
use types::support_foot::Side;

use crate::{
    joint_control::{JointTarget, MotionProgress},
    parameters::{Parameters, ScanParameters},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanKind {
    LookAround,
    SearchForLostBall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanWaypoint {
    Center { next_side: Side },
    Side(Side),
}

impl ScanWaypoint {
    fn position(self, parameters: &ScanParameters) -> HeadJoints<f32> {
        match self {
            Self::Center { .. } => parameters.center,
            Self::Side(Side::Left) => parameters.left,
            Self::Side(Side::Right) => parameters.right,
        }
    }

    fn next(self, kind: ScanKind) -> Self {
        match (self, kind) {
            (Self::Center { next_side }, _) => Self::Side(next_side),
            (Self::Side(side), ScanKind::LookAround) => Self::Center {
                next_side: side.opposite(),
            },
            (Self::Side(side), ScanKind::SearchForLostBall) => Self::Side(side.opposite()),
        }
    }
}

pub struct ScanOutput {
    pub target: JointTarget,
    /// Describes the waypoint just abandoned, not the newly returned target.
    pub timeout: Option<ScanTimeout>,
}

#[derive(Debug)]
pub struct ScanTimeout {
    pub kind: ScanKind,
    pub waypoint: ScanWaypoint,
    pub requested_target: HeadJoints<f32>,
    pub progress: Option<MotionProgress>,
    pub elapsed: Duration,
    pub ever_reached: bool,
}

enum ScanPhase {
    Moving,
    Dwelling { since: Time },
}

struct ActiveScan {
    kind: ScanKind,
    waypoint: ScanWaypoint,
    position: HeadJoints<f32>,
    started: Time,
    last_update: Time,
    phase: ScanPhase,
    effective_target: Option<HeadJoints<f32>>,
    ever_reached: bool,
}

#[derive(Default)]
pub struct ScanState {
    active: Option<ActiveScan>,
}

impl ScanState {
    /// Call only when a scan output is requested. Progress is from the preceding
    /// joint-control output; its requested target must match the active waypoint.
    /// Initial side is chosen by the coordinator from field side (left if unknown).
    /// It affects LookAround on entry only; lost-ball search starts at center.
    pub fn update(
        &mut self,
        kind: ScanKind,
        initial_side: Side,
        progress: Option<&MotionProgress>,
        parameters: &Parameters,
        now: Time,
    ) -> Result<ScanOutput> {
        let scan_parameters = match kind {
            ScanKind::LookAround => &parameters.look_around,
            ScanKind::SearchForLostBall => &parameters.search_for_lost_ball,
        };
        scan_parameters
            .validate()
            .map_err(|error| eyre!("{kind:?}: {error}"))?;
        let previous = self.active.take().filter(|scan| {
            scan.kind == kind
                && now >= scan.last_update
                && now.duration_since(scan.last_update) <= parameters.joint_control.reseed_after
        });
        let (scan, output) = match previous {
            Some(mut scan) => {
                let output = scan.update(progress, scan_parameters, now);
                (scan, output)
            }
            None => {
                let waypoint = match kind {
                    ScanKind::LookAround => ScanWaypoint::Side(initial_side),
                    ScanKind::SearchForLostBall => ScanWaypoint::Center {
                        next_side: Side::Left,
                    },
                };
                let scan = ActiveScan::new(kind, waypoint, scan_parameters, now);
                // Never reuse arrival from the previous mode or activity period.
                let output = scan.output(scan_parameters, None);
                (scan, output)
            }
        };
        self.active = Some(scan);
        Ok(output)
    }

    /// Call when leaving scanning, including damping or another head-motion mode.
    pub fn reset(&mut self) {
        self.active = None;
    }
}

impl ActiveScan {
    fn new(kind: ScanKind, waypoint: ScanWaypoint, parameters: &ScanParameters, now: Time) -> Self {
        Self {
            kind,
            waypoint,
            position: waypoint.position(parameters),
            started: now,
            last_update: now,
            phase: ScanPhase::Moving,
            effective_target: None,
            ever_reached: false,
        }
    }

    fn update(
        &mut self,
        progress: Option<&MotionProgress>,
        parameters: &ScanParameters,
        now: Time,
    ) -> ScanOutput {
        self.last_update = now;
        if self.position != self.waypoint.position(parameters) {
            // A live waypoint edit starts a new movement and invalidates old dwell.
            *self = Self::new(self.kind, self.waypoint, parameters, now);
            return self.output(parameters, None);
        }
        let progress = progress.filter(|progress| progress.requested_target == self.position);
        let complete = self.update_dwell(progress, parameters.dwell_duration, now);
        let expired = now.duration_since(self.started) >= parameters.maximum_waypoint_duration;
        let timeout = (expired && !complete).then(|| ScanTimeout {
            kind: self.kind,
            waypoint: self.waypoint,
            requested_target: self.position,
            progress: progress.copied(),
            elapsed: now.duration_since(self.started),
            ever_reached: self.ever_reached,
        });
        if complete || expired {
            // Advance at most once; this request has not commanded the next target yet.
            *self = Self::new(self.kind, self.waypoint.next(self.kind), parameters, now);
        }
        self.output(parameters, timeout)
    }

    fn update_dwell(
        &mut self,
        progress: Option<&MotionProgress>,
        dwell: Duration,
        now: Time,
    ) -> bool {
        let effective_target = progress.map(|progress| progress.effective_target);
        if self.effective_target != effective_target {
            self.phase = ScanPhase::Moving;
            self.effective_target = effective_target;
        }
        if !progress.is_some_and(|progress| progress.target_reached) {
            self.phase = ScanPhase::Moving;
            return false;
        }
        self.ever_reached = true;
        let since = match self.phase {
            ScanPhase::Moving => {
                self.phase = ScanPhase::Dwelling { since: now };
                now
            }
            ScanPhase::Dwelling { since } => since,
        };
        now.duration_since(since) >= dwell
    }

    fn output(&self, parameters: &ScanParameters, timeout: Option<ScanTimeout>) -> ScanOutput {
        ScanOutput {
            target: JointTarget::MoveTo {
                position: self.position,
                travel_speed: parameters.travel_speed,
            },
            timeout,
        }
    }
}

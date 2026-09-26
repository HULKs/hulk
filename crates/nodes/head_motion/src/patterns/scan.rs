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

#[cfg(test)]
mod tests {
    use json5::from_str;

    use super::*;

    fn configuration() -> Parameters {
        let mut parameters: Parameters = from_str(include_str!(
            "../../../../../etc/parameters/base/head_motion.json5"
        ))
        .unwrap();
        parameters.validate().unwrap();
        // Allow deliberately sparse requests when testing phase timing in isolation.
        parameters.joint_control.reseed_after = Duration::from_secs(1);
        for scan in [
            &mut parameters.look_around,
            &mut parameters.search_for_lost_ball,
        ] {
            scan.dwell_duration = Duration::from_millis(100);
            scan.maximum_waypoint_duration = Duration::from_millis(500);
        }
        parameters
    }

    fn at(millis: u64) -> Time {
        Time::zero() + Duration::from_millis(millis)
    }

    fn position(output: &ScanOutput) -> HeadJoints<f32> {
        let JointTarget::MoveTo { position, .. } = output.target else {
            panic!("scan must request travel speed")
        };
        position
    }

    fn arrived(target: HeadJoints<f32>) -> MotionProgress {
        MotionProgress {
            requested_target: target,
            effective_target: target,
            position_reached: true,
            target_reached: true,
            constrained: false,
        }
    }

    #[test]
    fn localization_visits_center_but_search_only_starts_there() {
        let parameters = configuration();
        let localization = &parameters.look_around;
        let search = &parameters.search_for_lost_ball;
        for (kind, sequence) in [
            (
                ScanKind::LookAround,
                [
                    localization.right,
                    localization.center,
                    localization.left,
                    localization.center,
                    localization.right,
                ],
            ),
            (
                ScanKind::SearchForLostBall,
                [
                    search.center,
                    search.left,
                    search.right,
                    search.left,
                    search.right,
                ],
            ),
        ] {
            let mut state = ScanState::default();
            let mut output = state
                .update(kind, Side::Right, None, &parameters, at(0))
                .unwrap();
            let mut millis = 0;
            for expected in sequence {
                assert_eq!(position(&output), expected);
                assert!(output.timeout.is_none());
                let feedback = arrived(expected);
                // Repeated requests (even with a different initial side) preserve phase.
                for elapsed in [10, 109] {
                    output = state
                        .update(
                            kind,
                            Side::Left,
                            Some(&feedback),
                            &parameters,
                            at(millis + elapsed),
                        )
                        .unwrap();
                    assert_eq!(position(&output), expected, "must dwell for a full 100 ms");
                }
                millis += 110;
                output = state
                    .update(kind, Side::Left, Some(&feedback), &parameters, at(millis))
                    .unwrap();
            }
        }
    }

    #[test]
    fn dwell_requires_matching_continuous_arrival_at_the_effective_target() {
        let parameters = configuration();
        let kind = ScanKind::LookAround;
        let mut state = ScanState::default();
        let first = state
            .update(kind, Side::Left, None, &parameters, at(0))
            .unwrap();
        let target = position(&first);
        let mut feedback = arrived(target);
        feedback.effective_target.yaw = 0.8;
        feedback.constrained = true;
        for millis in [10, 90] {
            state
                .update(kind, Side::Left, Some(&feedback), &parameters, at(millis))
                .unwrap();
        }
        feedback.target_reached = false;
        state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(100))
            .unwrap();
        feedback.target_reached = true;
        state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(110))
            .unwrap();
        // A live limit reduction invalidates dwell at the old constrained goal.
        feedback.effective_target.yaw = 0.7;
        for millis in [200, 299] {
            let output = state
                .update(kind, Side::Left, Some(&feedback), &parameters, at(millis))
                .unwrap();
            assert_eq!(position(&output), target);
        }
        let next = state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(300))
            .unwrap();
        assert_eq!(position(&next), parameters.look_around.center);
        // Stale arrival from the side cannot start the center's dwell.
        for millis in [310, 450] {
            let output = state
                .update(kind, Side::Left, Some(&feedback), &parameters, at(millis))
                .unwrap();
            assert_eq!(position(&output), parameters.look_around.center);
        }
    }

    #[test]
    fn arrival_losses_do_not_extend_the_deadline_or_skip_multiple_waypoints() {
        let parameters = configuration();
        let kind = ScanKind::SearchForLostBall;
        let mut state = ScanState::default();
        state
            .update(kind, Side::Left, None, &parameters, at(0))
            .unwrap();
        let mut feedback = arrived(parameters.search_for_lost_ball.center);
        for millis in (10..500).step_by(10) {
            feedback.target_reached = millis % 100 < 50;
            let output = state
                .update(kind, Side::Left, Some(&feedback), &parameters, at(millis))
                .unwrap();
            assert_eq!(position(&output), parameters.search_for_lost_ball.center);
            assert!(output.timeout.is_none());
        }
        let next = state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(600))
            .unwrap();
        assert_eq!(position(&next), parameters.search_for_lost_ball.left);
        let timeout = next.timeout.unwrap();
        assert!(timeout.ever_reached);
        assert_eq!(timeout.requested_target, feedback.requested_target);
        assert_eq!(timeout.elapsed, Duration::from_millis(600));
        let repeated = state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(600))
            .unwrap();
        assert!(repeated.timeout.is_none());
        assert_eq!(position(&repeated), parameters.search_for_lost_ball.left);
    }

    #[test]
    fn mode_changes_reactivation_clock_rollback_and_live_edits_reset_dwell() {
        let mut parameters = configuration();
        let kind = ScanKind::LookAround;
        let mut state = ScanState::default();
        state
            .update(kind, Side::Left, None, &parameters, at(0))
            .unwrap();
        let feedback = arrived(parameters.look_around.left);
        state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(10))
            .unwrap();
        // An inactivity gap starts again on the currently requested initial side.
        let resumed = state
            .update(kind, Side::Right, Some(&feedback), &parameters, at(1100))
            .unwrap();
        assert_eq!(position(&resumed), parameters.look_around.right);
        assert!(resumed.timeout.is_none());
        let rolled_back = state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(1000))
            .unwrap();
        assert_eq!(position(&rolled_back), parameters.look_around.left);
        state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(1010))
            .unwrap();
        parameters.look_around.left.pitch = 0.6;
        let edited = state
            .update(kind, Side::Left, Some(&feedback), &parameters, at(1110))
            .unwrap();
        assert_eq!(position(&edited), parameters.look_around.left);
        assert!(matches!(
            state.active.as_ref().unwrap().phase,
            ScanPhase::Moving
        ));
        let switched = state
            .update(
                ScanKind::SearchForLostBall,
                Side::Right,
                Some(&feedback),
                &parameters,
                at(1120),
            )
            .unwrap();
        assert_eq!(position(&switched), parameters.search_for_lost_ball.center);
        state.reset();
        let reentered = state
            .update(kind, Side::Right, None, &parameters, at(1130))
            .unwrap();
        assert_eq!(position(&reentered), parameters.look_around.right);
    }
}

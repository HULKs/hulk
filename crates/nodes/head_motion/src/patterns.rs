//! Timed scan waypoints. Phases advance independently of measured arrival.

use kinematics::joints::head::HeadJoints;
use ros_z::time::Time;
use types::side::Side;

use crate::parameters::ScanParameters;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanKind {
    LookAround,
    SearchForLostBall,
}

#[derive(Default)]
pub(crate) struct ScanState {
    waypoint: Option<Waypoint>,
    started: Option<Time>,
}

#[derive(Clone, Copy)]
enum Waypoint {
    Center { next_side: Side },
    Side(Side),
}

impl ScanState {
    /// `HeadController` resets this state when the mode changes or requests stop.
    pub(crate) fn update(
        &mut self,
        kind: ScanKind,
        initial_side: Side,
        parameters: &ScanParameters,
        now: Time,
    ) -> HeadJoints<f32> {
        let waypoint = self.waypoint.get_or_insert(match kind {
            ScanKind::LookAround => Waypoint::Side(initial_side),
            ScanKind::SearchForLostBall => Waypoint::Center {
                next_side: Side::Left,
            },
        });
        let started = self.started.get_or_insert(now);
        if now.duration_since(*started) >= parameters.waypoint_duration {
            *waypoint = match (*waypoint, kind) {
                (Waypoint::Center { next_side }, _) => Waypoint::Side(next_side),
                (Waypoint::Side(side), ScanKind::LookAround) => Waypoint::Center {
                    next_side: side.opposite(),
                },
                (Waypoint::Side(side), ScanKind::SearchForLostBall) => {
                    Waypoint::Side(side.opposite())
                }
            };
            *started = now;
        }
        match *waypoint {
            Waypoint::Center { .. } => parameters.center,
            Waypoint::Side(Side::Left) => parameters.left,
            Waypoint::Side(Side::Right) => parameters.right,
        }
    }
}

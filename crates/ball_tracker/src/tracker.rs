use std::time::Duration;

use coordinate_systems::Ground;
use linear_algebra::{Isometry2, Point2, Vector2};
use nalgebra::Matrix2;
use types::ball_tracking::{TrackStatus, TrackerDebugState};

use crate::{FieldBounds, config::TrackerParameters, measurement::Measurement};

pub type TrackId = u64;

#[derive(Clone, Debug)]
pub struct TrackSnapshot {
    pub id: TrackId,
    pub position: Point2<Ground>,
    pub velocity: Vector2<Ground>,
    pub covariance: Matrix2<f32>,
    pub existence_probability: f32,
    pub status: TrackStatus,
    pub last_seen_age: Duration,
}

#[derive(Clone, Debug)]
pub struct TrackerOutput {
    pub tracks: Vec<TrackSnapshot>,
    pub primary_track: Option<TrackSnapshot>,
    pub debug: TrackerDebugState,
}

#[derive(Clone, Debug)]
pub struct Tracker {
    #[expect(dead_code, reason = "reserved for future track allocation")]
    next_track_id: TrackId,
    previous_primary_track_id: Option<TrackId>,
}

impl Tracker {
    pub fn new() -> Self {
        Self {
            next_track_id: 1,
            previous_primary_track_id: None,
        }
    }

    pub fn update(
        &mut self,
        _dt: Duration,
        _odometry_delta: Isometry2<Ground, Ground>,
        _measurements: &[Measurement],
        _parameters: &TrackerParameters,
        _field_bounds: FieldBounds,
        _is_visible: impl Fn(Point2<Ground>) -> bool,
    ) -> TrackerOutput {
        self.output()
    }

    pub fn output(&self) -> TrackerOutput {
        TrackerOutput {
            tracks: Vec::new(),
            primary_track: None,
            debug: TrackerDebugState {
                global_hypothesis_count: 1,
                best_hypothesis_weight_log: Some(0.0),
                track_count: 0,
                primary_track_id: self.previous_primary_track_id,
                assignment_count: 0,
                pruned_hypothesis_count: 0,
                pruned_track_count: 0,
            },
        }
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_has_empty_output() {
        let tracker = Tracker::new();
        let output = tracker.output();

        assert!(output.tracks.is_empty());
        assert!(output.primary_track.is_none());
        assert_eq!(output.debug.global_hypothesis_count, 1);
        assert_eq!(output.debug.track_count, 0);
    }
}

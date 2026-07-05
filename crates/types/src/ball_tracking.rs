use coordinate_systems::Ground;
use linear_algebra::{Point2, Vector2};
use nalgebra::Matrix2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::time::Time;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
    ros_z::Message,
)]
pub enum TrackStatus {
    Tentative,
    Confirmed,
    Stale,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
    ros_z::Message,
)]
pub struct BallTrack<Frame = Ground> {
    pub id: u64,
    pub position: Point2<Frame>,
    pub velocity: Vector2<Frame>,
    #[path_serde(leaf)]
    pub covariance: Matrix2<f32>,
    pub existence_probability: f32,
    pub status: TrackStatus,
    #[path_serde(skip)]
    pub last_seen: Time,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
    ros_z::Message,
)]
pub struct TrackerDebugState {
    pub global_hypothesis_count: usize,
    pub best_hypothesis_weight_log: Option<f32>,
    pub track_count: usize,
    pub primary_track_id: Option<u64>,
    pub assignment_count: usize,
    pub pruned_hypothesis_count: usize,
    pub pruned_track_count: usize,
}

#[cfg(test)]
mod tests {
    use coordinate_systems::Ground;
    use linear_algebra::{point, vector};
    use nalgebra::Matrix2;
    use ros_z::time::Time;

    use super::{BallTrack, TrackStatus, TrackerDebugState};

    #[test]
    fn ball_track_carries_primary_tracking_fields() {
        let track = BallTrack::<Ground> {
            id: 7,
            position: point![1.0, 2.0],
            velocity: vector![0.3, -0.1],
            covariance: Matrix2::identity(),
            existence_probability: 0.75,
            status: TrackStatus::Confirmed,
            last_seen: Time::zero(),
        };

        assert_eq!(track.id, 7);
        assert_eq!(track.status, TrackStatus::Confirmed);
        assert_eq!(track.existence_probability, 0.75);
    }

    #[test]
    fn debug_state_allows_empty_tracker() {
        let debug = TrackerDebugState {
            global_hypothesis_count: 0,
            best_hypothesis_weight_log: None,
            track_count: 0,
            primary_track_id: None,
            assignment_count: 0,
            pruned_hypothesis_count: 0,
            pruned_track_count: 0,
        };

        assert_eq!(debug.best_hypothesis_weight_log, None);
        assert_eq!(debug.primary_track_id, None);
    }
}

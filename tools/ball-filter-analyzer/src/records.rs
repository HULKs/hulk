#[derive(Clone, Debug, PartialEq)]
pub struct TopicHealthRow {
    pub topic: String,
    pub message_count: i64,
    pub first_time_ns: Option<i64>,
    pub last_time_ns: Option<i64>,
    pub average_rate_hz: Option<f64>,
    pub max_gap_ms: Option<f64>,
    pub missing: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerceptRow {
    pub time_ns: i64,
    pub percept_index: i32,
    pub x: f32,
    pub y: f32,
    pub cov_xx: f32,
    pub cov_xy: f32,
    pub cov_yy: f32,
    pub image_x: f32,
    pub image_y: f32,
    pub image_radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackRow {
    pub time_ns: i64,
    pub track_id: i64,
    pub status: String,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub existence: f32,
    pub cov_xx: f32,
    pub cov_xy: f32,
    pub cov_yy: f32,
    pub last_seen_ns: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimaryRow {
    pub time_ns: i64,
    pub track_id: Option<i64>,
    pub status: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub vx: Option<f32>,
    pub vy: Option<f32>,
    pub existence: Option<f32>,
    pub last_seen_ns: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebugRow {
    pub time_ns: i64,
    pub global_hypothesis_count: i64,
    pub best_hypothesis_weight_log: Option<f32>,
    pub track_count: i64,
    pub primary_track_id: Option<i64>,
    pub assignment_count: i64,
    pub pruned_hypothesis_count: i64,
    pub pruned_track_count: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventRow {
    pub time_ns: i64,
    pub event_kind: String,
    pub severity: String,
    pub track_id: Option<i64>,
    pub details: String,
}

use coordinate_systems::Ground;
use types::{
    ball_detection::BallPercept,
    ball_tracking::{BallTrack, TrackStatus, TrackerDebugState},
};

pub fn status_name(status: TrackStatus) -> String {
    match status {
        TrackStatus::Tentative => "Tentative".to_string(),
        TrackStatus::Confirmed => "Confirmed".to_string(),
        TrackStatus::Stale => "Stale".to_string(),
    }
}

pub fn percept_rows(time_ns: i64, percepts: &[BallPercept]) -> Vec<PerceptRow> {
    percepts
        .iter()
        .enumerate()
        .map(|(index, percept)| PerceptRow {
            time_ns,
            percept_index: i32::try_from(index).unwrap_or(i32::MAX),
            x: percept.percept_in_ground.mean.x,
            y: percept.percept_in_ground.mean.y,
            cov_xx: percept.percept_in_ground.covariance[(0, 0)],
            cov_xy: percept.percept_in_ground.covariance[(0, 1)],
            cov_yy: percept.percept_in_ground.covariance[(1, 1)],
            image_x: percept.image_location.center.x(),
            image_y: percept.image_location.center.y(),
            image_radius: percept.image_location.radius,
        })
        .collect()
}

pub fn track_rows(time_ns: i64, tracks: &[BallTrack<Ground>]) -> Vec<TrackRow> {
    tracks
        .iter()
        .map(|track| TrackRow {
            time_ns,
            track_id: i64::try_from(track.id).unwrap_or(i64::MAX),
            status: status_name(track.status),
            x: track.position.x(),
            y: track.position.y(),
            vx: track.velocity.x(),
            vy: track.velocity.y(),
            existence: track.existence_probability,
            cov_xx: track.covariance[(0, 0)],
            cov_xy: track.covariance[(0, 1)],
            cov_yy: track.covariance[(1, 1)],
            last_seen_ns: i64::try_from(track.last_seen.as_nanos()).unwrap_or(i64::MAX),
        })
        .collect()
}

pub fn primary_row(time_ns: i64, primary: &Option<BallTrack<Ground>>) -> PrimaryRow {
    match primary {
        Some(track) => PrimaryRow {
            time_ns,
            track_id: Some(i64::try_from(track.id).unwrap_or(i64::MAX)),
            status: Some(status_name(track.status)),
            x: Some(track.position.x()),
            y: Some(track.position.y()),
            vx: Some(track.velocity.x()),
            vy: Some(track.velocity.y()),
            existence: Some(track.existence_probability),
            last_seen_ns: Some(i64::try_from(track.last_seen.as_nanos()).unwrap_or(i64::MAX)),
        },
        None => PrimaryRow {
            time_ns,
            track_id: None,
            status: None,
            x: None,
            y: None,
            vx: None,
            vy: None,
            existence: None,
            last_seen_ns: None,
        },
    }
}

pub fn debug_row(time_ns: i64, debug: &TrackerDebugState) -> DebugRow {
    DebugRow {
        time_ns,
        global_hypothesis_count: i64::try_from(debug.global_hypothesis_count).unwrap_or(i64::MAX),
        best_hypothesis_weight_log: debug.best_hypothesis_weight_log,
        track_count: i64::try_from(debug.track_count).unwrap_or(i64::MAX),
        primary_track_id: debug
            .primary_track_id
            .map(|id| i64::try_from(id).unwrap_or(i64::MAX)),
        assignment_count: i64::try_from(debug.assignment_count).unwrap_or(i64::MAX),
        pruned_hypothesis_count: i64::try_from(debug.pruned_hypothesis_count).unwrap_or(i64::MAX),
        pruned_track_count: i64::try_from(debug.pruned_track_count).unwrap_or(i64::MAX),
    }
}

#[cfg(test)]
mod tests {
    use coordinate_systems::Ground;
    use geometry::circle::Circle;
    use linear_algebra::{point, vector};
    use nalgebra::Matrix2;
    use ros_z::time::Time;
    use types::{
        ball_detection::BallPercept,
        ball_tracking::{BallTrack, TrackStatus, TrackerDebugState},
        multivariate_normal_distribution::MultivariateNormalDistribution,
    };

    use super::*;

    #[test]
    fn percept_rows_flatten_percept_position_covariance_and_image_circle() {
        let percept = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 2.0],
                covariance: Matrix2::new(0.1, 0.2, 0.2, 0.4),
            },
            image_location: Circle::new(point![10.0, 20.0], 3.0),
        };

        let rows = percept_rows(42, &[percept]);

        assert_eq!(rows[0].time_ns, 42);
        assert_eq!(rows[0].percept_index, 0);
        assert_eq!(rows[0].x, 1.0);
        assert_eq!(rows[0].cov_xy, 0.2);
        assert_eq!(rows[0].image_radius, 3.0);
    }

    #[test]
    fn track_rows_flatten_track_state() {
        let track = BallTrack::<Ground> {
            id: 7,
            position: point![1.0, 2.0],
            velocity: vector![0.3, -0.2],
            covariance: Matrix2::new(0.1, 0.2, 0.2, 0.4),
            existence_probability: 0.8,
            status: TrackStatus::Confirmed,
            last_seen: Time::from_nanos(900),
        };

        let rows = track_rows(1_000, &[track]);

        assert_eq!(rows[0].track_id, 7);
        assert_eq!(rows[0].status, "Confirmed");
        assert_eq!(rows[0].vx, 0.3);
        assert_eq!(rows[0].last_seen_ns, 900);
    }

    #[test]
    fn primary_row_uses_null_fields_when_primary_is_missing() {
        let row = primary_row(1_000, &None);

        assert_eq!(row.time_ns, 1_000);
        assert!(row.track_id.is_none());
        assert!(row.status.is_none());
        assert!(row.x.is_none());
    }

    #[test]
    fn debug_row_flattens_tracker_debug_state() {
        let row = debug_row(
            1_000,
            &TrackerDebugState {
                global_hypothesis_count: 16,
                best_hypothesis_weight_log: Some(-0.5),
                track_count: 3,
                primary_track_id: Some(7),
                assignment_count: 8,
                pruned_hypothesis_count: 2,
                pruned_track_count: 1,
            },
        );

        assert_eq!(row.global_hypothesis_count, 16);
        assert_eq!(row.primary_track_id, Some(7));
        assert_eq!(row.assignment_count, 8);
    }
}

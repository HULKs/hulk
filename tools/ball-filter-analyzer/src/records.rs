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

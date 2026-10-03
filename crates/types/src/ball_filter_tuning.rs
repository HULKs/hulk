//! Monitoring messages shared by the offline optimizer and Twix.
use serde::{Deserialize, Serialize};

use crate::parameters::BallFilterParameters;

pub const ROUTER: &str = "tcp/127.0.0.1:7448";
pub const NAMESPACE: &str = "/ball_tuning";
pub const PROGRESS_TOPIC: &str = "tuning/progress";
pub const OPEN_VIEWER_TOPIC: &str = "/ball_tuning/tuning/open_viewer";
pub const OPPONENTS_TOPIC: &str = "/ball_tuning/tuning/opponents";
pub const WALKING_SPEED_TOPIC: &str = "/ball_tuning/tuning/walking_speed_scale";

pub const fn default_walking_speed_scale() -> f32 {
    1.0
}

pub fn walking_speed_scale_is_valid(scale: f32) -> bool {
    scale.is_finite() && (0.1..=3.0).contains(&scale)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct OpponentParameters {
    pub count: u32,
    /// Cylinder diameter in metres, shared by physics, occlusion and rendering.
    pub width: f32,
}

impl Default for OpponentParameters {
    fn default() -> Self {
        Self {
            count: 2,
            width: 0.44,
        }
    }
}

impl OpponentParameters {
    pub fn is_valid(self) -> bool {
        self.count <= 8 && self.width.is_finite() && (0.1..=1.2).contains(&self.width)
    }
}
pub const TUNED_PARAMETER_POINTERS: &[&str] = &[
    "/noise/detection_noise",
    "/noise/process_noise_resting",
    "/noise/process_noise_moving",
    "/maximum_matching_cost",
    "/velocity_decay_factor",
    "/hidden_validity_decay_rate",
    "/visible_missed_validity_decay_rate",
    "/competing_hypothesis_validity_decay_rate",
];

#[derive(Clone, Debug, Default, Serialize, Deserialize, ros_z::Message)]
pub struct Metrics {
    pub loss: f64,
    pub position_rmse_metres: Option<f64>,
    pub missing_seconds: f64,
    pub missing_runs: u64,
    pub longest_missing_seconds: f64,
    pub close_range_position_rmse_metres: Option<f64>,
    pub close_range_present_seconds: f64,
    pub close_range_missing_seconds: f64,
    pub along_motion_error_metres: Option<f64>,
    pub motion_lag_seconds: Option<f64>,
    pub moving_reference_seconds: f64,
    pub false_track_seconds: f64,
    pub missing_transform_seconds: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ros_z::Message)]
pub struct SearchProgress {
    pub reference_frame: String,
    pub trial: u64,
    pub trials: u64,
    pub best_trial: u64,
    pub baseline: Metrics,
    pub best: Metrics,
    pub best_parameters: BallFilterParameters,
    pub validation_baseline: Option<Metrics>,
    pub validation_best: Option<Metrics>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ros_z::Message)]
pub struct RemoteWorker {
    pub run: String,
    pub worker: u64,
    pub round: u64,
    pub status: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ros_z::Message)]
pub struct RemoteProgress {
    pub host: String,
    pub workers: Vec<RemoteWorker>,
    pub completed_trials: u64,
    pub best_candidate: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ros_z::Message)]
pub struct Progress {
    pub status: String,
    pub recording: String,
    pub recording_index: u64,
    pub recordings: u64,
    pub phase: String,
    pub elapsed_seconds: f64,
    pub duration_seconds: f64,
    pub search: Option<SearchProgress>,
    pub output_directory: String,
    pub error: Option<String>,
    pub viewer_status: Option<String>,
    pub live_trial: Option<u64>,
    pub live_status: Option<String>,
    #[serde(default)]
    pub remote: Option<RemoteProgress>,
    /// Last successful remote status poll, independent of the local heartbeat.
    #[serde(default)]
    pub remote_updated_unix_seconds: Option<f64>,
    #[serde(default)]
    pub opponents: OpponentParameters,
    #[serde(default)]
    pub active_opponents: Option<OpponentParameters>,
    /// Human-selected multiplier for behavior walking speeds, never an optimizer variable.
    #[serde(default = "default_walking_speed_scale")]
    pub walking_speed_scale: f32,
    #[serde(default)]
    pub active_walking_speed_scale: Option<f32>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            status: String::new(),
            recording: String::new(),
            recording_index: 0,
            recordings: 0,
            phase: String::new(),
            elapsed_seconds: 0.0,
            duration_seconds: 0.0,
            search: None,
            output_directory: String::new(),
            error: None,
            viewer_status: None,
            live_trial: None,
            live_status: None,
            remote: None,
            remote_updated_unix_seconds: None,
            opponents: OpponentParameters::default(),
            active_opponents: None,
            walking_speed_scale: default_walking_speed_scale(),
            active_walking_speed_scale: None,
        }
    }
}

use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct ViewerConfig {
    pub namespace: String,
    pub source_expression: String,
    pub storage_path: Option<PathBuf>,
    pub poll_interval: Duration,
    pub discovery_reconcile_interval: Duration,
    pub scrub_window_radius: Duration,
    pub scrub_prefetch_radius: Duration,
    pub scrub_debounce: Duration,
    pub max_timeline_points: usize,
    pub max_samples_per_lane: usize,
    pub max_retained_lanes: usize,
    pub live_event_batch_max: usize,
    pub live_event_batch_delay: Duration,
    pub worker_command_channel_capacity: usize,
    pub worker_event_channel_capacity: usize,
    pub worker_internal_event_channel_capacity: usize,
    pub discovery_event_channel_capacity: usize,
    pub max_events_per_frame: usize,
    pub max_event_bytes_per_frame: usize,
    pub max_event_ingest_time_per_frame: Duration,
    pub repaint_delay_on_activity: Duration,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            namespace: "demo".to_string(),
            source_expression: "odometry".to_string(),
            storage_path: None,
            poll_interval: Duration::from_millis(100),
            discovery_reconcile_interval: Duration::from_secs(30),
            scrub_window_radius: Duration::from_secs(5),
            scrub_prefetch_radius: Duration::from_secs(10),
            scrub_debounce: Duration::from_millis(200),
            max_timeline_points: 200_000,
            max_samples_per_lane: 50_000,
            max_retained_lanes: 512,
            live_event_batch_max: 32,
            live_event_batch_delay: Duration::from_millis(40),
            worker_command_channel_capacity: 256,
            worker_event_channel_capacity: 512,
            worker_internal_event_channel_capacity: 512,
            discovery_event_channel_capacity: 512,
            max_events_per_frame: 256,
            max_event_bytes_per_frame: 1_500_000,
            max_event_ingest_time_per_frame: Duration::from_millis(6),
            repaint_delay_on_activity: Duration::from_millis(10),
        }
    }
}

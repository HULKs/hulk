use std::time::{Duration, Instant};

use hulkz_stream::{BackendStats, SourceStats};

#[derive(Debug, Clone)]
pub struct ViewerConfig {
    pub namespace: &'static str,
    pub source_path: &'static str,
    pub poll_interval: Duration,
    pub scrub_window_radius: Duration,
    pub scrub_prefetch_radius: Duration,
    pub scrub_debounce: Duration,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            namespace: "demo",
            source_path: "odometry",
            poll_interval: Duration::from_millis(100),
            scrub_window_radius: Duration::from_secs(5),
            scrub_prefetch_radius: Duration::from_secs(10),
            scrub_debounce: Duration::from_millis(200),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordRow {
    pub timestamp_nanos: u64,
    pub effective_namespace: Option<String>,
    pub json_pretty: Option<String>,
    pub raw_fallback: Option<String>,
}

#[derive(Debug, Clone)]
pub enum WorkerCommand {
    SetIngestEnabled(bool),
    SetFollowLive(bool),
    SetScrubAnchor(u64),
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    RecordsAppended(Vec<RecordRow>),
    Stats {
        source: Box<SourceStats>,
        backend: Box<BackendStats>,
    },
    Error(String),
    Ready,
}

#[derive(Debug, Default)]
pub struct ViewerState {
    pub records: Vec<RecordRow>,
    pub selected_index: Option<usize>,
    pub follow_live: bool,
}

impl ViewerState {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            selected_index: None,
            follow_live: true,
        }
    }

    pub fn append_records(&mut self, records: Vec<RecordRow>) {
        if records.is_empty() {
            return;
        }

        self.records.extend(records);

        if self.follow_live || self.selected_index.is_none() {
            self.selected_index = Some(self.records.len().saturating_sub(1));
        }
    }

    pub fn step_prev(&mut self) {
        let next = self
            .selected_index
            .unwrap_or_else(|| self.records.len().saturating_sub(1))
            .saturating_sub(1);
        if !self.records.is_empty() {
            self.selected_index = Some(next);
        }
    }

    pub fn step_next(&mut self) {
        if self.records.is_empty() {
            return;
        }

        let max_index = self.records.len().saturating_sub(1);
        let next = self
            .selected_index
            .unwrap_or(max_index)
            .saturating_add(1)
            .min(max_index);
        self.selected_index = Some(next);
    }

    pub fn jump_latest(&mut self) {
        if !self.records.is_empty() {
            self.selected_index = Some(self.records.len().saturating_sub(1));
        }
    }

    pub fn selected(&self) -> Option<&RecordRow> {
        self.selected_index.and_then(|idx| self.records.get(idx))
    }
}

pub fn should_emit_scrub_command(last_emitted: Instant, now: Instant, debounce: Duration) -> bool {
    now.saturating_duration_since(last_emitted) >= debounce
}

#[cfg(test)]
mod tests {
    use super::{should_emit_scrub_command, RecordRow, ViewerState};
    use std::time::{Duration, Instant};

    fn row(ts: u64) -> RecordRow {
        RecordRow {
            timestamp_nanos: ts,
            effective_namespace: Some("demo".to_string()),
            json_pretty: Some(format!("{{\n  \"ts\": {ts}\n}}")),
            raw_fallback: None,
        }
    }

    #[test]
    fn append_with_follow_live_selects_latest() {
        let mut state = ViewerState::new();
        state.follow_live = true;

        state.append_records(vec![row(1), row(2), row(3)]);

        assert_eq!(state.selected_index, Some(2));
    }

    #[test]
    fn append_without_follow_live_keeps_selection() {
        let mut state = ViewerState::new();
        state.follow_live = false;
        state.append_records(vec![row(1), row(2), row(3)]);
        state.selected_index = Some(1);

        state.append_records(vec![row(4), row(5)]);

        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn step_controls_clamp_indices() {
        let mut state = ViewerState::new();
        state.follow_live = false;
        state.append_records(vec![row(10), row(20), row(30)]);
        state.selected_index = Some(0);

        state.step_prev();
        assert_eq!(state.selected_index, Some(0));

        state.step_next();
        state.step_next();
        state.step_next();
        assert_eq!(state.selected_index, Some(2));
    }

    #[test]
    fn scrub_debounce_blocks_rapid_updates() {
        let now = Instant::now();
        let debounce = Duration::from_millis(200);

        assert!(!should_emit_scrub_command(
            now,
            now + Duration::from_millis(100),
            debounce
        ));
        assert!(should_emit_scrub_command(
            now,
            now + Duration::from_millis(220),
            debounce
        ));
    }
}

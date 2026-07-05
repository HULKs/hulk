use std::collections::{BTreeMap, BTreeSet};

use crate::{mcap_input::AnalysisInput, records::EventRow};

#[derive(Clone, Debug)]
pub struct AnalysisConfig {
    pub match_window_ns: i64,
    pub duplicate_distance_m: f32,
    pub hypothesis_cap: i64,
    pub assignment_pressure_threshold: i64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            match_window_ns: 100_000_000,
            duplicate_distance_m: 0.3,
            hypothesis_cap: 16,
            assignment_pressure_threshold: 8,
        }
    }
}

pub fn detect_events(input: &AnalysisInput, config: &AnalysisConfig) -> Vec<EventRow> {
    let mut events = Vec::new();
    detect_primary_events(input, &mut events);
    detect_duplicate_tracks(input, config, &mut events);
    detect_debug_pressure(input, config, &mut events);
    detect_track_disappearances(input, &mut events);
    detect_mcap_stream_warnings(input, &mut events);
    events.sort_by_key(|event| event.time_ns);
    events
}

fn detect_primary_events(input: &AnalysisInput, events: &mut Vec<EventRow>) {
    let mut previous = None;
    for row in &input.primary {
        match (previous, row.track_id) {
            (Some(previous_id), Some(current_id)) if previous_id != current_id => {
                events.push(EventRow {
                    time_ns: row.time_ns,
                    event_kind: "primary_switch".to_string(),
                    severity: "warning".to_string(),
                    track_id: Some(current_id),
                    details: format!("primary switched {previous_id} -> {current_id}"),
                });
            }
            (_, None) => events.push(EventRow {
                time_ns: row.time_ns,
                event_kind: "primary_missing".to_string(),
                severity: "info".to_string(),
                track_id: None,
                details: "primary topic reported no track".to_string(),
            }),
            _ => {}
        }
        previous = row.track_id;
    }
}

fn detect_duplicate_tracks(
    input: &AnalysisInput,
    config: &AnalysisConfig,
    events: &mut Vec<EventRow>,
) {
    let mut tracks = input.tracks.iter().collect::<Vec<_>>();
    tracks.sort_by_key(|track| track.time_ns);

    for left_index in 0..tracks.len() {
        let left = tracks[left_index];
        for right in tracks.iter().skip(left_index + 1) {
            let time_delta = right.time_ns.saturating_sub(left.time_ns);
            if time_delta > config.match_window_ns {
                break;
            }
            if left.track_id == right.track_id {
                continue;
            }

            let distance = ((left.x - right.x).powi(2) + (left.y - right.y).powi(2)).sqrt();
            if distance <= config.duplicate_distance_m {
                events.push(EventRow {
                    time_ns: right.time_ns,
                    event_kind: "duplicate_tracks".to_string(),
                    severity: "warning".to_string(),
                    track_id: Some(left.track_id),
                    details: format!(
                        "tracks {} and {} were {distance:.3}m apart",
                        left.track_id, right.track_id
                    ),
                });
            }
        }
    }
}

fn detect_debug_pressure(
    input: &AnalysisInput,
    config: &AnalysisConfig,
    events: &mut Vec<EventRow>,
) {
    for debug in &input.debug {
        if debug.global_hypothesis_count >= config.hypothesis_cap {
            events.push(EventRow {
                time_ns: debug.time_ns,
                event_kind: "hypothesis_cap".to_string(),
                severity: "warning".to_string(),
                track_id: debug.primary_track_id,
                details: format!(
                    "global_hypothesis_count reached {}",
                    debug.global_hypothesis_count
                ),
            });
        }
        if debug.assignment_count >= config.assignment_pressure_threshold {
            events.push(EventRow {
                time_ns: debug.time_ns,
                event_kind: "assignment_pressure".to_string(),
                severity: "info".to_string(),
                track_id: debug.primary_track_id,
                details: format!("assignment_count was {}", debug.assignment_count),
            });
        }
    }
}

fn detect_track_disappearances(input: &AnalysisInput, events: &mut Vec<EventRow>) {
    let mut previous_ids = BTreeSet::<i64>::new();
    let mut by_time = BTreeMap::<i64, BTreeSet<i64>>::new();
    for track in &input.tracks {
        by_time
            .entry(track.time_ns)
            .or_default()
            .insert(track.track_id);
    }
    for (time_ns, ids) in by_time {
        for disappeared in previous_ids.difference(&ids) {
            events.push(EventRow {
                time_ns,
                event_kind: "track_disappeared".to_string(),
                severity: "info".to_string(),
                track_id: Some(*disappeared),
                details: format!("track {disappeared} vanished from public output"),
            });
        }
        previous_ids = ids;
    }
}

fn detect_mcap_stream_warnings(input: &AnalysisInput, events: &mut Vec<EventRow>) {
    for error in &input.decode_errors {
        if error.topic == "<mcap>" && error.time_ns == -1 {
            events.push(EventRow {
                time_ns: error.time_ns,
                event_kind: "mcap_stream_warning".to_string(),
                severity: "warning".to_string(),
                track_id: None,
                details: error.message.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        mcap_input::{AnalysisInput, DecodeErrorRow},
        records::{PrimaryRow, TrackRow},
    };

    use super::{AnalysisConfig, detect_events};

    #[test]
    fn detect_events_reports_primary_switch() {
        let input = AnalysisInput {
            primary: vec![
                PrimaryRow {
                    time_ns: 1_000,
                    track_id: Some(1),
                    status: Some("Confirmed".to_string()),
                    x: Some(0.0),
                    y: Some(0.0),
                    vx: Some(0.0),
                    vy: Some(0.0),
                    existence: Some(0.8),
                    last_seen_ns: Some(1_000),
                },
                PrimaryRow {
                    time_ns: 2_000,
                    track_id: Some(2),
                    status: Some("Confirmed".to_string()),
                    x: Some(0.0),
                    y: Some(0.0),
                    vx: Some(0.0),
                    vy: Some(0.0),
                    existence: Some(0.8),
                    last_seen_ns: Some(2_000),
                },
            ],
            ..Default::default()
        };

        let events = detect_events(&input, &AnalysisConfig::default());

        assert!(
            events
                .iter()
                .any(|event| event.event_kind == "primary_switch")
        );
    }

    #[test]
    fn detect_events_reports_duplicate_tracks_at_same_timestamp() {
        let input = AnalysisInput {
            tracks: vec![track_row(1_000, 1, 0.0, 0.0), track_row(1_000, 2, 0.1, 0.0)],
            ..Default::default()
        };

        let events = detect_events(&input, &AnalysisConfig::default());

        assert!(
            events
                .iter()
                .any(|event| event.event_kind == "duplicate_tracks")
        );
    }

    #[test]
    fn detect_events_uses_match_window_for_duplicate_tracks() {
        let input = AnalysisInput {
            tracks: vec![track_row(1_000, 1, 0.0, 0.0), track_row(1_050, 2, 0.1, 0.0)],
            ..Default::default()
        };
        let config = AnalysisConfig {
            match_window_ns: 100,
            ..Default::default()
        };

        let events = detect_events(&input, &config);

        assert!(events.iter().any(|event| {
            event.event_kind == "duplicate_tracks"
                && event.time_ns == 1_050
                && event.details.contains("tracks 1 and 2")
        }));
    }

    #[test]
    fn detect_events_reports_recoverable_mcap_stream_warning() {
        let input = AnalysisInput {
            decode_errors: vec![DecodeErrorRow {
                topic: "<mcap>".to_string(),
                time_ns: -1,
                message: "trailing chunk is truncated".to_string(),
            }],
            ..Default::default()
        };

        let events = detect_events(&input, &AnalysisConfig::default());

        assert!(events.iter().any(|event| {
            event.event_kind == "mcap_stream_warning"
                && event.time_ns == -1
                && event.details.contains("trailing chunk is truncated")
        }));
    }

    fn track_row(time_ns: i64, track_id: i64, x: f32, y: f32) -> TrackRow {
        TrackRow {
            time_ns,
            track_id,
            status: "Confirmed".to_string(),
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            existence: 0.8,
            cov_xx: 0.1,
            cov_xy: 0.0,
            cov_yy: 0.1,
            last_seen_ns: time_ns,
        }
    }
}

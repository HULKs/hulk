use std::sync::Arc;

use color_eyre::Result;
use eframe::{
    egui::Align2,
    epaint::{Color32, Stroke},
};

use coordinate_systems::Ground;
use linear_algebra::{Point2, point, vector};
use ros_z_debug::{SampleRecord, TopicObservation};
use types::{
    ball_detection::BallPercept,
    ball_tracking::{BallTrack, TrackStatus, TrackerDebugState},
    field_dimensions::FieldDimensions,
};

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct BallFilter {
    tracks: TopicObservation<Vec<BallTrack<Ground>>>,
    primary_ball: TopicObservation<Option<BallTrack<Ground>>>,
    debug_state: TopicObservation<TrackerDebugState>,
    ball_percepts: TopicObservation<Vec<BallPercept>>,
}

impl Layer<Ground> for BallFilter {
    const NAME: &'static str = "Ball Filter";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let tracks = backend
            .observer()
            .observe_typed("ball_filter/tracks")
            .expect("failed to construct ball tracks observer")
            .spawn();
        let primary_ball = backend
            .observer()
            .observe_typed("ball_filter/primary_ball")
            .expect("failed to construct primary ball observer")
            .spawn();
        let debug_state = backend
            .observer()
            .observe_typed("ball_filter/debug_state")
            .expect("failed to construct ball tracker debug observer")
            .spawn();
        let ball_percepts = backend
            .observer()
            .observe_typed("ball_filter/ball_percepts")
            .expect("failed to construct ball percepts observer")
            .spawn();

        Self {
            tracks,
            primary_ball,
            debug_state,
            ball_percepts,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        self.paint_percepts(painter);
        self.paint_tracks(painter);
        self.paint_primary_ball(painter);
        self.paint_debug_summary(painter);

        Ok(())
    }
}

impl BallFilter {
    fn paint_percepts(&self, painter: &TwixPainter<Ground>) {
        let latest_sample = self.ball_percepts.latest();
        let Some(SampleRecord {
            value: ball_percepts,
            ..
        }) = latest_sample.as_deref()
        else {
            return;
        };

        for percept in ball_percepts {
            let position = Point2::from(percept.percept_in_ground.mean);
            let covariance = percept.percept_in_ground.covariance;
            painter.covariance(
                position,
                covariance,
                Stroke::new(0.005, Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
                Color32::from_rgba_unmultiplied(255, 255, 255, 20),
            );
            painter.circle_filled(
                position,
                0.025,
                Color32::from_rgba_unmultiplied(255, 255, 255, 120),
            );
        }
    }

    fn paint_tracks(&self, painter: &TwixPainter<Ground>) {
        let latest_sample = self.tracks.latest();
        let Some(SampleRecord { value: tracks, .. }) = latest_sample.as_deref() else {
            return;
        };

        for track in tracks {
            let track_color = track_color(track.status);
            let stroke = Stroke::new(0.01, Color32::BLACK);
            painter.covariance(
                track.position,
                track.covariance,
                stroke,
                track_fill_color(track.status),
            );
            painter.target(track.position, 0.025, stroke, track_color);
            painter.line_segment(
                track.position,
                track.position + track.velocity,
                Stroke::new(0.015, track_color),
            );
            painter.floating_text(
                track.position + vector![0.05, 0.05],
                Align2::LEFT_BOTTOM,
                track_label(track),
                Default::default(),
                Color32::WHITE,
            );
        }
    }

    fn paint_primary_ball(&self, painter: &TwixPainter<Ground>) {
        let latest_sample = self.primary_ball.latest();
        let Some(SampleRecord {
            value: Some(primary_ball),
            ..
        }) = latest_sample.as_deref()
        else {
            return;
        };

        painter.circle_stroke(
            primary_ball.position,
            0.08,
            Stroke::new(0.025, Color32::WHITE),
        );
        painter.line_segment(
            primary_ball.position,
            primary_ball.position + primary_ball.velocity,
            Stroke::new(0.025, Color32::WHITE),
        );
    }

    fn paint_debug_summary(&self, painter: &TwixPainter<Ground>) {
        let latest_sample = self.debug_state.latest();
        let Some(SampleRecord { value: debug, .. }) = latest_sample.as_deref() else {
            return;
        };

        painter.floating_text(
            point![-0.95, 0.95],
            Align2::LEFT_TOP,
            debug_summary(debug),
            Default::default(),
            Color32::WHITE,
        );
    }
}

fn track_color(status: TrackStatus) -> Color32 {
    match status {
        TrackStatus::Tentative => Color32::from_rgba_unmultiplied(255, 180, 0, 180),
        TrackStatus::Confirmed => Color32::from_rgba_unmultiplied(0, 255, 0, 190),
        TrackStatus::Stale => Color32::from_rgba_unmultiplied(255, 102, 204, 180),
    }
}

fn track_fill_color(status: TrackStatus) -> Color32 {
    match status {
        TrackStatus::Tentative => Color32::from_rgba_unmultiplied(255, 180, 0, 70),
        TrackStatus::Confirmed => Color32::from_rgba_unmultiplied(0, 255, 0, 80),
        TrackStatus::Stale => Color32::from_rgba_unmultiplied(255, 102, 204, 70),
    }
}

fn track_label(track: &BallTrack<Ground>) -> String {
    format!(
        "#{} {} p={:.2}",
        track.id,
        status_label(track.status),
        track.existence_probability,
    )
}

fn status_label(status: TrackStatus) -> &'static str {
    match status {
        TrackStatus::Tentative => "tentative",
        TrackStatus::Confirmed => "confirmed",
        TrackStatus::Stale => "stale",
    }
}

fn debug_summary(debug: &TrackerDebugState) -> String {
    let primary_track = debug
        .primary_track_id
        .map(|id| format!("#{id}"))
        .unwrap_or_else(|| "-".to_string());
    let best_weight = debug
        .best_hypothesis_weight_log
        .map(|weight| format!("{weight:.2}"))
        .unwrap_or_else(|| "-".to_string());

    format!(
        "tracks: {}\nprimary: {}\nhypotheses: {}\nassignments: {}\npruned: h={} t={}\nbest logw: {}",
        debug.track_count,
        primary_track,
        debug.global_hypothesis_count,
        debug.assignment_count,
        debug.pruned_hypothesis_count,
        debug.pruned_track_count,
        best_weight,
    )
}

#[cfg(test)]
mod tests {
    use linear_algebra::{point, vector};
    use nalgebra::Matrix2;
    use ros_z::time::Time;
    use types::ball_tracking::{BallTrack, TrackStatus, TrackerDebugState};

    use super::{debug_summary, track_label};

    #[test]
    fn track_label_contains_track_id_status_and_existence_probability() {
        let track = BallTrack {
            id: 7,
            position: point![1.0, 2.0],
            velocity: vector![0.3, -0.1],
            covariance: Matrix2::identity(),
            existence_probability: 0.934,
            status: TrackStatus::Confirmed,
            last_seen: Time::zero(),
        };

        assert_eq!(track_label(&track), "#7 confirmed p=0.93");
    }

    #[test]
    fn debug_summary_formats_missing_optional_values_as_dash() {
        let debug = TrackerDebugState {
            global_hypothesis_count: 3,
            best_hypothesis_weight_log: None,
            track_count: 2,
            primary_track_id: None,
            assignment_count: 5,
            pruned_hypothesis_count: 1,
            pruned_track_count: 4,
        };

        assert_eq!(
            debug_summary(&debug),
            "tracks: 2\nprimary: -\nhypotheses: 3\nassignments: 5\npruned: h=1 t=4\nbest logw: -",
        );
    }

    #[test]
    fn debug_summary_formats_primary_track_and_weight() {
        let debug = TrackerDebugState {
            global_hypothesis_count: 4,
            best_hypothesis_weight_log: Some(-3.421),
            track_count: 3,
            primary_track_id: Some(7),
            assignment_count: 12,
            pruned_hypothesis_count: 1,
            pruned_track_count: 0,
        };

        assert_eq!(
            debug_summary(&debug),
            "tracks: 3\nprimary: #7\nhypotheses: 4\nassignments: 12\npruned: h=1 t=0\nbest logw: -3.42",
        );
    }
}

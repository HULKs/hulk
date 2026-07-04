use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use ros_z_debug::TopicObservation;
use types::{
    ball_tracking::{BallTrack, TrackStatus},
    field_dimensions::FieldDimensions,
};

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct BallFilter {
    tracks: TopicObservation<Vec<BallTrack<Ground>>>,
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

        Self { tracks }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(ros_z_debug::SampleRecord { value: tracks, .. }) =
            self.tracks.latest().as_deref()
        {
            for track in tracks {
                let stroke = Stroke::new(0.01_f32, Color32::BLACK);
                let color = match track.status {
                    TrackStatus::Tentative => Color32::from_rgba_unmultiplied(255, 255, 0, 100),
                    TrackStatus::Confirmed => Color32::from_rgba_unmultiplied(0, 255, 0, 120),
                    TrackStatus::Stale => Color32::from_rgba_unmultiplied(255, 102, 204, 100),
                };
                painter.covariance(track.position, track.covariance, stroke, color);
                painter.target(track.position, 0.02, stroke, color);
                painter.line_segment(track.position, track.position + track.velocity, stroke);
            }
        }

        Ok(())
    }
}

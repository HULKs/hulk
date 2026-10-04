use color_eyre::Report;
use coordinate_systems::{Field, Ground, Pixel};
use eframe::egui::Color32;
use ros_z::time::Time;
use twix_visualization::twix_painter::TwixPainter;
use types::ball_position::BallPosition;

use super::{
    super::image_overlay::{ImageOverlay, OverlayObservation},
    ball_projection::{ALIGNMENT_TOLERANCE, BallProjection},
};
use crate::{panels::ball_visualization::TEAM_BALL_COLOR, repaint::ObservationContext};

pub(in crate::panels::image) struct BallPositionOverlay {
    selected: OverlayObservation<Option<BallPosition<Ground>>>,
    team: OverlayObservation<Option<BallPosition<Field>>>,
    projection: BallProjection,
}

impl ImageOverlay for BallPositionOverlay {
    const NAME: &'static str = "Ball Filter";
    const STORAGE_KEY: &'static str = "selected_ball_filter";

    fn new<C: ObservationContext>(context: &C) -> Result<Self, Report> {
        Ok(Self {
            selected: OverlayObservation::new(context, "ball_filter/ball_position")?,
            team: OverlayObservation::new(context, "team_ball")?,
            projection: BallProjection::new(context)?,
        })
    }

    fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time) {
        let Some(frame) = self.projection.at_time(image_time) else {
            return;
        };
        if let Some(sample) = self
            .team
            .nearest_source_time(image_time, ALIGNMENT_TOLERANCE)
            && let Some(ball) = sample.value
        {
            frame.paint_field(painter, ball.position, TEAM_BALL_COLOR);
        }
        if let Some(sample) = self
            .selected
            .nearest_source_time(image_time, ALIGNMENT_TOLERANCE)
            && let Some(ball) = sample.value
        {
            let transform = self.projection.ground_transform(&frame, sample.source_time);
            frame.paint_ground(painter, transform * ball.position, Color32::BLUE);
        }
    }
}

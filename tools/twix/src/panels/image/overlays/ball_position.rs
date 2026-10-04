use std::sync::Arc;

use color_eyre::Report;
use coordinate_systems::{Field, Ground, Pixel};
use eframe::egui::Color32;
use ros_z::time::Time;
use ros_z_debug::SampleRecord;
use twix_visualization::twix_painter::TwixPainter;
use types::ball_position::BallPosition;

use super::{
    super::{
        image_overlay::ImageOverlay,
        inputs::{OverlayInput, OverlayInputs},
    },
    ball_projection::{BallProjection, aligned_window, nearest},
};
use crate::panels::ball_visualization::TEAM_BALL_COLOR;

#[derive(Default)]
pub(in crate::panels::image) struct BallPositionOverlay {
    selected: Option<Arc<SampleRecord<Option<BallPosition<Ground>>>>>,
    team: Option<Arc<SampleRecord<Option<BallPosition<Field>>>>>,
    projection: BallProjection,
}

impl ImageOverlay for BallPositionOverlay {
    fn inputs(&self) -> Vec<OverlayInput> {
        let mut inputs = vec![
            OverlayInput::source_time::<Option<BallPosition<Ground>>>(
                "ball_filter/ball_position",
                aligned_window(),
            ),
            OverlayInput::source_time::<Option<BallPosition<Field>>>("team_ball", aligned_window()),
        ];
        inputs.extend(BallProjection::inputs());
        inputs
    }

    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.selected = nearest(&inputs.records(0).unwrap_or_default(), time).cloned();
        self.team = nearest(&inputs.records(1).unwrap_or_default(), time).cloned();
        self.projection.prepare(time, inputs, 2)
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let Some(frame) = self.projection.frame() else {
            return;
        };
        if let Some(sample) = &self.team
            && let Some(ball) = sample.value
        {
            frame.paint_field(painter, ball.position, TEAM_BALL_COLOR);
        }
        if let Some(sample) = &self.selected
            && let Some(ball) = sample.value
        {
            let transform = self.projection.ground_transform(frame, sample.source_time);
            frame.paint_ground(painter, transform * ball.position, Color32::BLUE);
        }
    }
}

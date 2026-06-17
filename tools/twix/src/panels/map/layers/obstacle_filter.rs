use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::{field_dimensions::FieldDimensions, obstacle_filter::Hypothesis};

use crate::{
    backend::TwixBackend,
    panels::map::layer::Layer,
    twix_painter::TwixPainter,
    value_buffer::{BufferHandle, BufferHistory},
};

pub struct ObstacleFilter {
    hypotheses: BufferHandle<Vec<Hypothesis>>,
}

impl Layer<Ground> for ObstacleFilter {
    const NAME: &'static str = "Obstacle Filter";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let hypotheses = backend.subscribe_buffered_value_with_queue_depth(
            "obstacle_filter_hypotheses",
            BufferHistory::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        Self { hypotheses }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(hypotheses) = self.hypotheses.get_last_value()? {
            for hypothesis in hypotheses.iter() {
                let position = Point2::from(hypothesis.state.mean);
                let covariance = hypothesis.state.covariance;
                let stroke = Stroke::new(0.01_f32, Color32::BLACK);
                let fill_color = Color32::from_rgba_unmultiplied(255, 255, 0, 20);
                painter.covariance(position, covariance, stroke, fill_color);
            }
        }

        Ok(())
    }
}

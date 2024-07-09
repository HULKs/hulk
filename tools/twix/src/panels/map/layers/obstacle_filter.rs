use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::{field_dimensions::FieldDimensions, obstacle_filter::Hypothesis};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct ObstacleFilter {
    hypotheses: BufferHandle<Option<Vec<Hypothesis>>>,
}

impl Layer<Ground> for ObstacleFilter {
    const NAME: &'static str = "Obstacle Filter";

    fn new(nao: Arc<Nao>) -> Self {
        let hypotheses =
            nao.subscribe_value("Control.additional_outputs.obstacle_filter_hypotheses");
        Self { hypotheses }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(hypotheses) = self.hypotheses.get_last_value()?.flatten() {
            for hypothesis in hypotheses.iter() {
                let position = Point2::from(hypothesis.state.mean);
                let covariance = hypothesis.state.covariance;
                let stroke = Stroke::new(0.01, Color32::BLACK);
                let fill_color = Color32::from_rgba_unmultiplied(255, 255, 0, 20);
                painter.covariance(position, covariance, stroke, fill_color);
            }
        }

        Ok(())
    }
}

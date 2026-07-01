use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use ros_z_debug::TopicObservation;
use types::{field_dimensions::FieldDimensions, obstacle_filter::Hypothesis};

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct ObstacleFilter {
    hypotheses: TopicObservation<Vec<Hypothesis>>,
}

impl Layer<Ground> for ObstacleFilter {
    const NAME: &'static str = "Obstacle Filter";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let hypotheses = backend
            .observer()
            .observe_typed("obstacle_filter_hypotheses")
            .expect("failed to construct obstacle filter hypotheses observer")
            .spawn();

        Self { hypotheses }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(ros_z_debug::SampleRecord {
            value: hypotheses, ..
        }) = self.hypotheses.latest().as_deref()
        {
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

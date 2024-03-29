use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use types::{field_dimensions::FieldDimensions, obstacle_filter::Hypothesis};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct ObstacleFilter {
    ground_to_field: ValueBuffer,
    hypotheses: ValueBuffer,
}

impl Layer for ObstacleFilter {
    const NAME: &'static str = "Obstacle Filter";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Main {
                path: "ground_to_field".to_string(),
            },
        });
        let hypotheses = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Additional {
                path: "obstacle_filter_hypotheses".to_string(),
            },
        });
        Self {
            ground_to_field,
            hypotheses,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_field: Option<Isometry2<Ground, Field>> =
            self.ground_to_field.parse_latest()?;
        let hypotheses: Vec<Hypothesis> = self.hypotheses.parse_latest()?;

        for hypothesis in hypotheses.iter() {
            let position =
                ground_to_field.unwrap_or_default() * Point2::from(hypothesis.state.mean);
            let covariance = hypothesis.state.covariance;
            let stroke = Stroke::new(0.01, Color32::BLACK);
            let fill_color = Color32::from_rgba_unmultiplied(255, 255, 0, 20);
            painter.covariance(position, covariance, stroke, fill_color);
        }

        Ok(())
    }
}

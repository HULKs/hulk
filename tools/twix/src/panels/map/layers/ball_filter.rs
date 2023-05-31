use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::epaint::{Color32, Stroke};
use nalgebra::{Isometry2, Point2};
use types::{ball_filter::Hypothesis, FieldDimensions};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallFilter {
    robot_to_field: ValueBuffer,
    ball_hypotheses: ValueBuffer,
}

impl Layer for BallFilter {
    const NAME: &'static str = "Ball Filter";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Main {
                path: "robot_to_field".to_string(),
            },
        });
        let ball_hypotheses = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Additional {
                path: "ball_filter_hypotheses".to_string(),
            },
        });
        Self {
            robot_to_field,
            ball_hypotheses,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Option<Isometry2<f32>> = self.robot_to_field.parse_latest()?;
        let ball_hypotheses: Vec<Hypothesis> = self.ball_hypotheses.parse_latest()?;

        for hypothesis in ball_hypotheses.iter() {
            let position =
                robot_to_field.unwrap_or_default() * Point2::from(hypothesis.state.mean.xy());
            let covariance = hypothesis
                .state
                .covariance
                .fixed_view::<2, 2>(0, 0)
                .into_owned();
            let stroke = Stroke::new(0.01, Color32::BLACK);
            let fill_color = Color32::from_rgba_unmultiplied(255, 255, 0, 100);
            painter.covariance(position, covariance, stroke, fill_color);
        }

        Ok(())
    }
}

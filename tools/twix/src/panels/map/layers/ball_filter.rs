use std::sync::Arc;

use ball_filter::BallHypothesis;
use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Ground;
use linear_algebra::IntoFramed;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallFilter {
    ball_state: ValueBuffer,
}

impl Layer<Ground> for BallFilter {
    const NAME: &'static str = "Ball Filter";

    fn new(nao: Arc<Nao>) -> Self {
        let ball_state = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Additional {
                path: "best_ball_hypothesis".to_string(),
            },
        });
        Self { ball_state }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ball_state: BallHypothesis = self.ball_state.require_latest()?;

        let stroke = Stroke::new(0.01, Color32::BLACK);
        let resting_color = Color32::GRAY.gamma_multiply(0.5);
        let moving_color = Color32::YELLOW.gamma_multiply(0.5);

        let resting = ball_state.resting;
        painter.covariance(
            resting.mean.framed::<Ground>().as_point(),
            resting.covariance,
            stroke,
            resting_color,
        );

        let moving = ball_state.moving;
        painter.covariance(
            moving.mean.xy().framed::<Ground>().as_point(),
            moving.covariance.fixed_view::<2, 2>(0, 0).into_owned(),
            stroke,
            moving_color,
        );

        Ok(())
    }
}

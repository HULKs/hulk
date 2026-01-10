use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::{ball_detection::BallPercept, field_dimensions::FieldDimensions};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct BallMeasurement {
    balls: BufferHandle<Option<Vec<BallPercept>>>,
}

impl Layer<Ground> for BallMeasurement {
    const NAME: &'static str = "Ball Measurements";

    fn new(nao: Arc<Nao>) -> Self {
        let balls = nao.subscribe_value("WorldState.main_outputs.balls");
        Self { balls }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let balls = self.balls.get_last_value()?.flatten();

        for ball in balls.iter().flatten() {
            let position = Point2::from(ball.percept_in_ground.mean);
            let covariance = ball.percept_in_ground.covariance;

            let stroke = Stroke::new(0.01, Color32::BLACK);
            painter.covariance(
                position,
                covariance,
                stroke,
                Color32::YELLOW.gamma_multiply(0.5),
            );
            painter.ball(position, 0.07, Color32::WHITE);
        }

        Ok(())
    }
}

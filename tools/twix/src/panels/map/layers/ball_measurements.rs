use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::{ball::BallPercept, field_dimensions::FieldDimensions};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallMeasurement {
    detected_balls_top: ValueBuffer,
    detected_balls_bottom: ValueBuffer,
}

impl Layer<Ground> for BallMeasurement {
    const NAME: &'static str = "Ball Measurements";

    fn new(nao: Arc<Nao>) -> Self {
        let detected_balls_top = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::VisionTop,
            output: Output::Main {
                path: "balls".to_string(),
            },
        });
        let detected_balls_bottom = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::VisionBottom,
            output: Output::Main {
                path: "balls".to_string(),
            },
        });
        Self {
            detected_balls_bottom,
            detected_balls_top,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let balls_top: Vec<BallPercept> = self.detected_balls_top.parse_latest()?;
        let balls_bottom: Vec<BallPercept> = self.detected_balls_bottom.parse_latest()?;

        for ball in balls_top.iter().chain(balls_bottom.iter()) {
            let position = Point2::from(ball.percept_in_ground.mean);
            let covariance = ball.percept_in_ground.covariance;

            let stroke = Stroke::new(0.01, Color32::BLACK);
            painter.covariance(
                position,
                covariance,
                stroke,
                Color32::YELLOW.gamma_multiply(0.5),
            );
            painter.ball(position, 0.07);
        }

        Ok(())
    }
}

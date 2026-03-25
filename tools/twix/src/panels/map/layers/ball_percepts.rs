use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::{ball_detection::BallPercept, field_dimensions::FieldDimensions};

use crate::{
    panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct BallPercepts {
    ball_percepts: BufferHandle<Option<Vec<BallPercept>>>,
}

impl Layer<Ground> for BallPercepts {
    const NAME: &'static str = "Ball Percepts";

    fn new(robot: Arc<Robot>) -> Self {
        let ball_percepts = robot.subscribe_value("WorldState.additional_outputs.ball_percepts");
        Self { ball_percepts }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(ball_percepts) = self.ball_percepts.get_last_value()?.flatten() else {
            return Ok(());
        };

        for percept in ball_percepts {
            let position = Point2::from(percept.percept_in_ground.mean);
            let covariance = percept.percept_in_ground.covariance;

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

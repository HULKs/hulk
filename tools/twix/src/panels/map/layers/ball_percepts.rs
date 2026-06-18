use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use ros_z_debug::RetentionPolicy;
use types::{ball_detection::BallPercept, field_dimensions::FieldDimensions};

use crate::{
    backend::{TwixBackend, retained_subscription::TypedSubscription},
    panels::map::{latest_value, layer::Layer},
    twix_painter::TwixPainter,
};

pub struct BallPercepts {
    ball_percepts: TypedSubscription<Vec<BallPercept>>,
}

impl Layer<Ground> for BallPercepts {
    const NAME: &'static str = "Ball Percepts";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let ball_percepts = backend.subscribe_typed_retained(
            "ball_filter/ball_percepts",
            RetentionPolicy::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        Self { ball_percepts }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(ball_percepts) = latest_value(&self.ball_percepts) else {
            return Ok(());
        };

        for percept in ball_percepts {
            let position = Point2::from(percept.percept_in_ground.mean);
            let covariance = percept.percept_in_ground.covariance;

            let stroke = Stroke::new(0.01_f32, Color32::BLACK);
            painter.covariance(
                position,
                covariance,
                stroke,
                Color32::YELLOW.gamma_multiply(0.5),
            );
            painter.ball(position, 0.07_f32, Color32::WHITE);
        }

        Ok(())
    }
}

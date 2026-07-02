use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use ros_z_debug::{SampleRecord, TopicObservation};
use types::{ball_detection::BallPercept, field_dimensions::FieldDimensions};

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct BallPercepts {
    ball_percepts: TopicObservation<Vec<BallPercept>>,
}

impl Layer<Ground> for BallPercepts {
    const NAME: &'static str = "Ball Percepts";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let ball_percepts = backend
            .observer()
            .observe_typed("ball_filter/ball_percepts")
            .expect("failed to construct ball_percepts observer")
            .spawn();

        Self { ball_percepts }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let latest_sample = self.ball_percepts.latest();

        let Some(SampleRecord {
            value: ball_percepts,
            ..
        }) = latest_sample.as_deref()
        else {
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

use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use eframe::epaint::Color32;

use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use linear_algebra::Point2;
use ros_z_debug::{RetentionPolicy, SampleRecord, TopicObservation};
use types::{ball_detection::BallPercept, field_dimensions::FieldDimensions};

use crate::{backend::RobotBackend, panels::map::layer::Layer};
use twix_visualization::twix_painter::TwixPainter;

pub struct BallPercepts {
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
    ball_percepts: TopicObservation<Vec<BallPercept>>,
}

impl Layer<Ground> for BallPercepts {
    const NAME: &'static str = "Ball Percepts";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();
        let ground_to_field = backend
            .observer()
            .observe_typed("ground_to_field")
            .expect("failed to construct ground_to_field observer")
            .retention(RetentionPolicy::time_window(Duration::from_secs(2)).unwrap())
            .spawn();

        let ball_percepts = backend
            .observer()
            .observe_typed("ball_filter/ball_percepts")
            .expect("failed to construct ball_percepts observer")
            .spawn();

        Self {
            ball_percepts,
            ground_to_field,
        }
    }

    fn repaint_on_updates(&self, context: &impl ObservationContext) -> Vec<ObservationRepaint> {
        vec![
            self.ball_percepts.repaint_on_updates(context),
            self.ground_to_field.repaint_on_updates(context),
        ]
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let latest_sample = self.ball_percepts.latest();

        let Some(SampleRecord {
            value: ball_percepts,
            source_time,
            ..
        }) = latest_sample.as_deref()
        else {
            return Ok(());
        };

        let Some(current) = self.ground_to_field.latest() else {
            return Ok(());
        };
        let Some(captured) = self.ground_to_field.get_nearest(*source_time) else {
            return Ok(());
        };
        let transform = current.value.inverse() * captured.value;
        for percept in ball_percepts {
            let position = transform * Point2::from(percept.percept_in_ground.mean);
            painter.ball(position, field_dimensions.ball_radius, Color32::GREEN);
        }

        Ok(())
    }
}

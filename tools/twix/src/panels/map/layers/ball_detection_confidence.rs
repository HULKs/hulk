use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use eframe::egui::{Align2, Color32, FontId, vec2};
use linear_algebra::Isometry2;
use linear_algebra::Point2;
use ros_z_debug::{RetentionPolicy, TopicObservation};
use types::{
    ball_detection::BallPercept,
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

use crate::{backend::RobotBackend, panels::map::layer::Layer};
use twix_visualization::twix_painter::TwixPainter;

pub struct BallDetectionConfidence {
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
    percepts: TopicObservation<Vec<BallPercept>>,
    detections: TopicObservation<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>,
}

impl Layer<Ground> for BallDetectionConfidence {
    const NAME: &'static str = "Ball Percept Confidence";
    const STORAGE_KEY: Option<&'static str> = Some("ball_percept_confidence");

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();
        let ground_to_field = backend
            .observer()
            .observe_typed("ground_to_field")
            .expect("failed to construct ground_to_field observer")
            .retention(RetentionPolicy::time_window(Duration::from_secs(2)).unwrap())
            .spawn();
        let percepts = backend
            .observer()
            .observe_typed("ball_filter/ball_percepts")
            .expect("failed to construct ball percepts observer")
            .spawn();
        let detections = backend
            .observer()
            .observe_typed("detected_objects")
            .expect("failed to construct detected objects observer")
            .retention(RetentionPolicy::time_window(Duration::from_secs(2)).unwrap())
            .spawn();
        Self {
            ground_to_field,
            percepts,
            detections,
        }
    }

    fn repaint_on_updates(&self, context: &impl ObservationContext) -> Vec<ObservationRepaint> {
        vec![
            self.percepts.repaint_on_updates(context),
            self.detections.repaint_on_updates(context),
            self.ground_to_field.repaint_on_updates(context),
        ]
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(percepts) = self.percepts.latest() else {
            return Ok(());
        };
        let Some(current) = self.ground_to_field.latest() else {
            return Ok(());
        };
        let Some(captured) = self.ground_to_field.get_nearest(percepts.source_time) else {
            return Ok(());
        };
        let transform = current.value.inverse() * captured.value;
        let detections = self.detections.get_all();
        for percept in &percepts.value {
            // Match frame time and image circle to recover the detector score.
            let detection = detections
                .iter()
                .rev()
                .filter(|sample| sample.value.time == percepts.source_time)
                .find_map(|sample| {
                    crate::panels::ball_visualization::detection_for_percept(
                        percept,
                        &sample.value.inner,
                    )
                });
            let Some(detection) = detection else {
                continue;
            };
            let position = transform * Point2::from(percept.percept_in_ground.mean);
            let screen_position = painter.transform_world_to_pixel(position)
                + vec2(field_dimensions.ball_radius * painter.scaling() + 4.0, 0.0);
            painter.floating_text(
                painter.transform_pixel_to_world(screen_position),
                Align2::LEFT_CENTER,
                format!("{:.2}", detection.bounding_box.confidence),
                FontId::proportional(12.0),
                Color32::GREEN,
            );
        }
        Ok(())
    }
}

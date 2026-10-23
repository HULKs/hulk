use std::sync::Arc;

use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, FontId, vec2};
use ros_z::time::Time;
use ros_z_debug::{SampleRecord, query::plan::Selection};
use twix_visualization::twix_painter::TwixPainter;
use types::{
    ball_percept::BallPercept,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

use super::super::{
    image_overlay::ImageOverlay,
    inputs::{OverlayInput, OverlayInputs},
};

#[derive(Default)]
pub(in crate::panels::image) struct BallPerceptsOverlay {
    percepts: Option<Arc<SampleRecord<Vec<BallPercept>>>>,
}

impl ImageOverlay for BallPerceptsOverlay {
    fn inputs(&self) -> Vec<OverlayInput> {
        vec![
            OverlayInput::source_time::<Vec<BallPercept>>(
                "ball_filter/ball_percepts",
                Selection::Exact,
            )
            .prefer_frame(),
        ]
    }

    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.percepts = None;
        self.percepts = inputs
            .decode(0, time)?
            .map(|records| Arc::clone(records.last()));
        Ok(())
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let Some(percepts) = &self.percepts else {
            return;
        };
        for percept in &percepts.value {
            painter.ball(
                percept.image_location.center,
                percept.image_location.radius,
                Color32::GREEN,
            );
        }
    }
}

type Detections = TimeWrapper<Vec<Object<RobocupObjectLabel>>>;

#[derive(Default)]
pub(in crate::panels::image) struct BallDetectionConfidenceOverlay {
    percepts: Option<Arc<SampleRecord<Vec<BallPercept>>>>,
    detections: Option<Arc<SampleRecord<Detections>>>,
}

impl ImageOverlay for BallDetectionConfidenceOverlay {
    fn inputs(&self) -> Vec<OverlayInput> {
        vec![
            OverlayInput::source_time::<Vec<BallPercept>>(
                "ball_filter/ball_percepts",
                Selection::Exact,
            )
            .prefer_frame(),
            OverlayInput::new::<Detections>("detected_objects", Selection::Exact).prefer_frame(),
        ]
    }

    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.percepts = None;
        self.detections = None;
        self.percepts = inputs
            .decode(0, time)?
            .map(|records| Arc::clone(records.last()));
        self.detections = inputs
            .decode(1, time)?
            .map(|records| Arc::clone(records.last()));
        Ok(())
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let (Some(percepts), Some(detections)) = (&self.percepts, &self.detections) else {
            return;
        };
        for percept in &percepts.value {
            let Some(detection) = crate::panels::ball_visualization::detection_for_percept(
                percept,
                &detections.value.inner,
            ) else {
                continue;
            };
            let area = detection.bounding_box.area;
            let radius = (area.max.x() - area.min.x()).min(area.max.y() - area.min.y()) / 2.0;
            let screen_position = painter.transform_world_to_pixel(area.center())
                + vec2(radius * painter.scaling() + 4.0, 0.0);
            painter.floating_text(
                painter.transform_pixel_to_world(screen_position),
                Align2::LEFT_CENTER,
                format!("{:.2}", detection.bounding_box.confidence),
                FontId::proportional(12.0),
                Color32::GREEN,
            );
        }
    }
}

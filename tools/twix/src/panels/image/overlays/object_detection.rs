use std::sync::Arc;

use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{DragValue, Ui};
use ros_z::time::Time;
use ros_z_debug::{SampleRecord, query::plan::Selection};
use twix_visualization::twix_painter::TwixPainter;
use types::{
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

use super::super::{
    image_overlay::ImageOverlay,
    inputs::{OverlayInput, OverlayInputs},
};

use super::prediction_colors;

pub(in crate::panels::image) struct ObjectDetectionOverlay {
    confidence_threshold: f32,
    sample: Option<Arc<SampleRecord<Detections>>>,
}

impl Default for ObjectDetectionOverlay {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.5,
            sample: None,
        }
    }
}

type Detections = TimeWrapper<Vec<Object<RobocupObjectLabel>>>;

impl ImageOverlay for ObjectDetectionOverlay {
    fn inputs(&self) -> Vec<OverlayInput> {
        vec![OverlayInput::new::<Detections>("detected_objects", Selection::Exact).prefer_frame()]
    }

    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.sample = None;
        self.sample = inputs
            .decode(0, time)?
            .map(|records| Arc::clone(records.last()));
        Ok(())
    }

    fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Confidence");
            ui.add(
                DragValue::new(&mut self.confidence_threshold)
                    .range(0.0..=1.0)
                    .speed(0.01)
                    .fixed_decimals(2),
            );
        });
    }

    fn reset_settings(&mut self) {
        *self = Self::default();
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let Some(object_detections) = &self.sample else {
            return;
        };
        paint_bounding_boxes(
            painter,
            &object_detections.value.inner,
            self.confidence_threshold,
        );
    }
}

fn paint_bounding_boxes(
    painter: &TwixPainter<Pixel>,
    detections: &[Object<RobocupObjectLabel>],
    confidence_threshold: f32,
) {
    for detection in detections {
        let bounding_box = detection.bounding_box;
        if bounding_box.confidence < confidence_threshold {
            continue;
        }
        painter.detection_box(
            bounding_box,
            detection.label.into(),
            prediction_colors::robocup_object(detection.label),
        );
    }
}

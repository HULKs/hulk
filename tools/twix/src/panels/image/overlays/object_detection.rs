use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{DragValue, Ui};
use ros_z::time::Time;
use twix_visualization::twix_painter::TwixPainter;
use types::{
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

use crate::repaint::ObservationContext;

use super::super::image_overlay::{ImageOverlay, OverlayObservation};
use super::prediction_colors;

pub(in crate::panels::image) struct ObjectDetectionOverlay {
    confidence_threshold: f32,
    object_detections: OverlayObservation<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>,
}

impl ImageOverlay for ObjectDetectionOverlay {
    const NAME: &'static str = "Object Detection";
    const STORAGE_KEY: &'static str = "object_detection";

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            confidence_threshold: 0.5,
            object_detections: OverlayObservation::new(context, "detected_objects")?,
        })
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

    fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time) {
        let Some(object_detections) = self.object_detections.at_time(image_time) else {
            return;
        };
        paint_bounding_boxes(
            painter,
            &object_detections.value.inner,
            self.confidence_threshold,
        );
    }

    fn latest_time(&self) -> Option<Time> {
        self.object_detections.latest_time()
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

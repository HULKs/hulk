use color_eyre::Report;
use eframe::egui::{Align2, Color32, Stroke};
use types::object_detection::{Object, RobocupObjectLabel};

use crate::repaint::ObservationContext;

use super::super::image_overlay::{ImageOverlay, ImageOverlayPainter, OverlayObservation};

pub(in crate::panels::image) struct ObjectDetectionOverlay {
    object_detections: OverlayObservation<Vec<Object<RobocupObjectLabel>>>,
}

impl ImageOverlay for ObjectDetectionOverlay {
    const NAME: &'static str = "Object Detection";
    const STORAGE_KEY: &'static str = "object_detection";

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            object_detections: OverlayObservation::new(context, "detected_objects")?,
        })
    }

    fn paint(&self, painter: &ImageOverlayPainter) {
        let Some(object_detections) = self.object_detections.latest() else {
            return;
        };
        paint_bounding_boxes(painter, &object_detections.value, Color32::LIGHT_RED);
    }
}

fn paint_bounding_boxes(
    painter: &ImageOverlayPainter,
    detections: &[Object<RobocupObjectLabel>],
    line_color: Color32,
) {
    for detection in detections {
        let bounding_box = detection.bounding_box;
        painter.rect_stroke(
            bounding_box.area.min,
            bounding_box.area.max,
            Stroke::new(1.0, line_color),
        );
        painter.floating_text(
            bounding_box.area.min,
            Align2::RIGHT_BOTTOM,
            format!("{:.2}", bounding_box.confidence),
            Color32::WHITE,
        );
        painter.floating_text(
            bounding_box.area.max,
            Align2::RIGHT_TOP,
            detection.label.into(),
            Color32::WHITE,
        );
    }
}

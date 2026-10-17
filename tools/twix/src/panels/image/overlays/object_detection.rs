use std::sync::Arc;

use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, Stroke};
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

#[derive(Default)]
pub(in crate::panels::image) struct ObjectDetectionOverlay {
    sample: Option<Arc<SampleRecord<Detections>>>,
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

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let Some(object_detections) = &self.sample else {
            return;
        };
        paint_bounding_boxes(painter, &object_detections.value.inner, Color32::LIGHT_RED);
    }
}

fn paint_bounding_boxes(
    painter: &TwixPainter<Pixel>,
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
            eframe::egui::FontId::default(),
            Color32::WHITE,
        );
        painter.floating_text(
            bounding_box.area.max,
            Align2::RIGHT_TOP,
            detection.label.into(),
            eframe::egui::FontId::default(),
            Color32::WHITE,
        );
    }
}

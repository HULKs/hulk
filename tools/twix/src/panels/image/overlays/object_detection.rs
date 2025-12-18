use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, FontId, Stroke};
use types::object_detection::Detection;

use crate::{nao::Nao, panels::image::overlay::Overlay, value_buffer::BufferHandle};

pub struct ObjectDetection {
    object_detections: BufferHandle<Vec<Detection>>,
}

impl Overlay for ObjectDetection {
    const NAME: &'static str = "Object Detection";

    fn new(nao: Arc<Nao>) -> Self {
        let object_detections =
            nao.subscribe_value("ObjectDetection.main_outputs.object_detections");
        Self { object_detections }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let Some(object_detections) = self.object_detections.get_last_value()? else {
            return Ok(());
        };

        paint_bounding_boxes(painter, object_detections, Color32::LIGHT_RED)?;

        Ok(())
    }

    fn config_ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
        });
    }
}

fn paint_bounding_boxes(
    painter: &crate::twix_painter::TwixPainter<Pixel>,
    detections: Vec<Detection>,
    line_color: Color32,
) -> Result<()> {
    for detection in detections {
        // draw bounding box
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
            FontId::default(),
            Color32::WHITE,
        );
        painter.floating_text(
            bounding_box.area.max,
            Align2::RIGHT_TOP,
            detection.label.to_string(),
            FontId::default(),
            Color32::WHITE,
        );
    }
    Ok(())
}

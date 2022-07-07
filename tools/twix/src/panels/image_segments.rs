use std::{str::FromStr, sync::Arc};

use communication::CyclerOutput;
use eframe::{
    egui::{ComboBox, Response, Ui, Widget},
    epaint::{Color32, Stroke},
    Storage,
};
use nalgebra::{point, vector, Similarity2};
use serde_json::from_value;
use types::{CameraPosition, ImageSegments, Rgb};

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};

use crate::twix_paint::TwixPainter;

pub struct ImageSegmentsPanel {
    nao: Arc<Nao>,
    value_buffer: ValueBuffer,
    camera_position: CameraPosition,
}

impl Panel for ImageSegmentsPanel {
    const NAME: &'static str = "Image Segments";

    fn new(nao: Arc<Nao>, _storage: Option<&dyn Storage>) -> Self {
        let value_buffer =
            nao.subscribe_output(CyclerOutput::from_str("vision_top.main.image_segments").unwrap());
        Self {
            nao,
            value_buffer,
            camera_position: CameraPosition::Top,
        }
    }
}

impl Widget for &mut ImageSegmentsPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut changed = false;
        let _camera_selector = ComboBox::from_label("Camera")
            .selected_text(format!("{:?}", self.camera_position))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut self.camera_position, CameraPosition::Top, "Top")
                    .clicked()
                {
                    changed = true;
                };
                if ui
                    .selectable_value(&mut self.camera_position, CameraPosition::Bottom, "Bottom")
                    .changed()
                {
                    changed = true;
                };
            });
        if changed {
            let output = match self.camera_position {
                CameraPosition::Top => {
                    CyclerOutput::from_str("vision_top.main.image_segments").unwrap()
                }
                CameraPosition::Bottom => {
                    CyclerOutput::from_str("vision_bottom.main.image_segments").unwrap()
                }
            };
            self.value_buffer = self.nao.subscribe_output(output);
        }
        let value = match self.value_buffer.get_latest() {
            Ok(value) => value,
            Err(error) => return ui.label(format!("{:?}", error)),
        };
        let image_segments: ImageSegments = from_value(value).unwrap();

        let painter = TwixPainter::new(ui, vector![640.0, 480.0], Similarity2::identity(), 1.0);

        for scanline in image_segments.scan_grid.vertical_scan_lines {
            let x = scanline.position as f32;
            for segment in scanline.segments {
                let color = Rgb::from(segment.color);
                let start = point![x * 2.0, segment.start as f32];
                let end = point![x * 2.0, segment.end as f32];
                painter.line_segment(
                    start,
                    end,
                    Stroke::new(4.0, Color32::from_rgb(color.r, color.g, color.b)),
                );
                painter.line_segment(
                    start - vector![1.0, 0.0],
                    start + vector![1.0, 0.0],
                    Stroke::new(1.0, Color32::from_rgb(0, 0, 255)),
                );
            }
        }
        painter.response
    }
}

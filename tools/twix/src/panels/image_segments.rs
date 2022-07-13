use std::{str::FromStr, sync::Arc};

use communication::CyclerOutput;
use eframe::{
    egui::{ComboBox, Response, Ui, Widget},
    epaint::{Color32, Stroke},
    Storage,
};
use nalgebra::{point, vector, Similarity2};
use types::{CameraPosition, ImageSegments, Rgb, RgbChannel};

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};

use crate::twix_paint::TwixPainter;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ColorMode {
    Original,
    FieldColor,
    Y,
    Cb,
    Cr,
    Red,
    Green,
    Blue,
    RedChromaticity,
    GreenChromaticity,
    BlueChromaticity,
}

pub struct ImageSegmentsPanel {
    nao: Arc<Nao>,
    value_buffer: ValueBuffer,
    camera_position: CameraPosition,
    color_mode: ColorMode,
    use_filtered_segments: bool,
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
            color_mode: ColorMode::Original,
            use_filtered_segments: false,
        }
    }
}

impl Widget for &mut ImageSegmentsPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            let mut camera_selection_changed = false;
            let _camera_selector = ComboBox::from_label("Camera")
                .selected_text(format!("{:?}", self.camera_position))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut self.camera_position, CameraPosition::Top, "Top")
                        .clicked()
                    {
                        camera_selection_changed = true;
                    };
                    if ui
                        .selectable_value(
                            &mut self.camera_position,
                            CameraPosition::Bottom,
                            "Bottom",
                        )
                        .changed()
                    {
                        camera_selection_changed = true;
                    };
                });
            let filtered_segments_checkbox =
                ui.checkbox(&mut self.use_filtered_segments, "Filtered Segments");
            if camera_selection_changed || filtered_segments_checkbox.changed() {
                let output = match (self.camera_position, self.use_filtered_segments) {
                    (CameraPosition::Top, false) => {
                        CyclerOutput::from_str("vision_top.main.image_segments").unwrap()
                    }
                    (CameraPosition::Top, true) => {
                        CyclerOutput::from_str("vision_top.main.filtered_segments").unwrap()
                    }
                    (CameraPosition::Bottom, false) => {
                        CyclerOutput::from_str("vision_bottom.main.image_segments").unwrap()
                    }
                    (CameraPosition::Bottom, true) => {
                        CyclerOutput::from_str("vision_bottom.main.filtered_segments").unwrap()
                    }
                };
                self.value_buffer = self.nao.subscribe_output(output);
            }
            ComboBox::from_label("ColorMode")
                .selected_text(format!("{:?}", self.color_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.color_mode, ColorMode::Original, "Original");
                    ui.selectable_value(&mut self.color_mode, ColorMode::FieldColor, "FieldColor");
                    ui.selectable_value(&mut self.color_mode, ColorMode::Y, "Y");
                    ui.selectable_value(&mut self.color_mode, ColorMode::Cb, "Cb");
                    ui.selectable_value(&mut self.color_mode, ColorMode::Cr, "Cr");
                    ui.selectable_value(&mut self.color_mode, ColorMode::Red, "Red");
                    ui.selectable_value(&mut self.color_mode, ColorMode::Green, "Green");
                    ui.selectable_value(&mut self.color_mode, ColorMode::Blue, "Blue");
                    ui.selectable_value(
                        &mut self.color_mode,
                        ColorMode::RedChromaticity,
                        "RedChromaticity",
                    );
                    ui.selectable_value(
                        &mut self.color_mode,
                        ColorMode::GreenChromaticity,
                        "GreenChromaticity",
                    );
                    ui.selectable_value(
                        &mut self.color_mode,
                        ColorMode::BlueChromaticity,
                        "BlueChromaticity",
                    );
                });
        });
        let image_segments: ImageSegments = match self.value_buffer.require_latest() {
            Ok(value) => value,
            Err(error) => return ui.label(format!("{:?}", error)),
        };

        let mut painter = TwixPainter::new(ui, vector![640.0, 480.0], Similarity2::identity(), 1.0);
        if let Some(hover_pos) = painter.response.hover_pos() {
            let image_coords = painter.inverse_transform_pos(hover_pos);
            let x = image_coords.x.round() as u16;
            let y = image_coords.y.round() as u16;
            if let Some(scanline) = image_segments
                .scan_grid
                .vertical_scan_lines
                .iter()
                .find(|scanline| scanline.position * 2 >= x)
            {
                if let Some(segment) = scanline.segments.iter().find(|segment| segment.end >= y) {
                    let start = segment.start;
                    let end = segment.end;
                    let ycbcr_color = segment.color;
                    let y = ycbcr_color.y;
                    let cb = ycbcr_color.cb;
                    let cr = ycbcr_color.cr;
                    let rgb_color = Rgb::from(ycbcr_color);
                    let r = rgb_color.r;
                    let g = rgb_color.g;
                    let b = rgb_color.b;
                    let red_chromaticity = rgb_color.get_chromaticity(RgbChannel::Red);
                    let green_chromaticity = rgb_color.get_chromaticity(RgbChannel::Green);
                    let blue_chromaticity = rgb_color.get_chromaticity(RgbChannel::Blue);
                    painter.response = painter
                .response
                .on_hover_text_at_pointer(format!("x: {x}, start: {start}, end: {end}\nY: {y:3}, Cb: {cb:3}, Cr: {cr:3}\nR: {r:3}, G: {g:3}, B: {b:3}\nr: {red_chromaticity:.2}, g: {green_chromaticity:.2}, b: {blue_chromaticity:.2}"));
                }
            }
        }

        for scanline in image_segments.scan_grid.vertical_scan_lines {
            let x = scanline.position as f32;
            for segment in scanline.segments {
                let ycbcr_color = segment.color;
                let rgb_color = Rgb::from(ycbcr_color);
                let start = point![x * 2.0, segment.start as f32];
                let end = point![x * 2.0, segment.end as f32];
                let original_color = Color32::from_rgb(rgb_color.r, rgb_color.g, rgb_color.b);
                let medium_color = Color32::LIGHT_YELLOW;
                let high_color = Color32::YELLOW;
                let visualized_color = match self.color_mode {
                    ColorMode::Original => original_color,
                    ColorMode::FieldColor => match segment.field_color {
                        types::Intensity::Low => original_color,
                        types::Intensity::Medium => medium_color,
                        types::Intensity::High => high_color,
                    },
                    ColorMode::Y => Color32::from_gray(ycbcr_color.y),
                    ColorMode::Cb => Color32::from_gray(ycbcr_color.cb),
                    ColorMode::Cr => Color32::from_gray(ycbcr_color.cr),
                    ColorMode::Red => Color32::from_gray(rgb_color.r),
                    ColorMode::Green => Color32::from_gray(rgb_color.g),
                    ColorMode::Blue => Color32::from_gray(rgb_color.b),
                    ColorMode::RedChromaticity => Color32::from_gray(
                        (rgb_color.get_chromaticity(RgbChannel::Red) * 255.0) as u8,
                    ),
                    ColorMode::GreenChromaticity => Color32::from_gray(
                        (rgb_color.get_chromaticity(RgbChannel::Green) * 255.0) as u8,
                    ),
                    ColorMode::BlueChromaticity => Color32::from_gray(
                        (rgb_color.get_chromaticity(RgbChannel::Blue) * 255.0) as u8,
                    ),
                };
                painter.line_segment(start, end, Stroke::new(4.0, visualized_color));
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

use std::sync::Arc;

use eframe::{
    egui::{ComboBox, Response, Ui, Widget},
    epaint::{Color32, Stroke},
};
use linear_algebra::{point, vector};
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use serde_json::{json, Value};
use types::{
    camera_position::CameraPosition,
    color::{Hsv, Rgb},
    image_segments::{Direction, ImageSegments, Segment},
};

use crate::{
    nao::Nao,
    panel::Panel,
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
    zoom_and_pan::ZoomAndPanTransform,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    buffer: BufferHandle<ImageSegments>,
    camera_position: CameraPosition,
    direction: Direction,
    color_mode: ColorMode,
    use_filtered_segments: bool,
    zoom_and_pan: ZoomAndPanTransform,
}

impl Panel for ImageSegmentsPanel {
    const NAME: &'static str = "Image Segments";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let camera_position = match value.and_then(|value| value.get("camera_position")) {
            Some(Value::String(string)) if string == "Bottom" => CameraPosition::Bottom,
            _ => CameraPosition::Top,
        };
        let value_buffer = nao.subscribe_value(format!(
            "Vision{camera_position:?}.main_outputs.image_segments"
        ));
        let color_mode = match value.and_then(|value| value.get("color_mode")) {
            Some(Value::String(string)) => serde_json::from_str(&format!("\"{string}\"")).unwrap(),
            _ => ColorMode::Original,
        };
        let use_filtered_segments = value
            .and_then(|value| value.get("use_filtered_segments"))
            .and_then(|value| value.as_bool())
            .unwrap_or_default();
        Self {
            nao,
            buffer: value_buffer,
            camera_position,
            direction: Direction::Vertical,
            color_mode,
            use_filtered_segments,
            zoom_and_pan: ZoomAndPanTransform::default(),
        }
    }

    fn save(&self) -> Value {
        json!({
            "camera_position": self.camera_position.clone(),
            "color_mode": self.color_mode.clone(),
            "use_filtered_segments": self.use_filtered_segments.clone()
        })
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
            ComboBox::from_label("Direction")
                .selected_text(format!("{:?}", self.direction))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.direction, Direction::Vertical, "Vertical");
                    ui.selectable_value(&mut self.direction, Direction::Horizontal, "Horizontal");
                });
            let filtered_segments_checkbox =
                ui.checkbox(&mut self.use_filtered_segments, "Filtered Segments");
            if camera_selection_changed || filtered_segments_checkbox.changed() {
                let output = match (self.camera_position, self.use_filtered_segments) {
                    (CameraPosition::Top, false) => {
                        "VisionTop.main_outputs.image_segments".to_string()
                    }
                    (CameraPosition::Top, true) => {
                        "VisionTop.main_outputs.filtered_segments".to_string()
                    }
                    (CameraPosition::Bottom, false) => {
                        "VisionBottom.main_outputs.image_segments".to_string()
                    }
                    (CameraPosition::Bottom, true) => {
                        "VisionBottom.main_outputs.filtered_segments".to_string()
                    }
                };
                self.buffer = self.nao.subscribe_value(output);
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
        let image_segments: ImageSegments = match self.buffer.get_last_value() {
            Ok(Some(value)) => value,
            Ok(None) => return ui.label("No data"),
            Err(error) => return ui.label(format!("{error:#}")),
        };

        let (mut response, mut painter) = TwixPainter::<Pixel>::allocate(
            ui,
            vector![640.0, 480.0],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );
        self.zoom_and_pan.apply(ui, &mut painter, &response);

        if let Some(hover_pos) = response.hover_pos() {
            let image_coords = painter.transform_pixel_to_world(hover_pos);
            let x = image_coords.x().round() as u16;
            let y = image_coords.y().round() as u16;
            if let Some(scanline) = image_segments
                .scan_grid
                .vertical_scan_lines
                .iter()
                .find(|scanline| scanline.position >= x)
            {
                if let Some(segment) = scanline.segments.iter().find(|segment| segment.end >= y) {
                    let start = segment.start;
                    let end = segment.end;
                    let ycbcr_color = segment.color;
                    let y = ycbcr_color.y;
                    let cb = ycbcr_color.cb;
                    let cr = ycbcr_color.cr;
                    let rgb_color = Rgb::from(ycbcr_color);
                    let r = rgb_color.red;
                    let g = rgb_color.green;
                    let b = rgb_color.blue;
                    let chromaticity = rgb_color.convert_to_rgchromaticity();
                    let red_chromaticity = chromaticity.red;
                    let green_chromaticity = chromaticity.green;
                    let blue_chromaticity = 1.0 - chromaticity.red - chromaticity.green;
                    let hsv: Hsv = rgb_color.into();
                    let h = hsv.h;
                    let s = hsv.s;
                    let v = hsv.v;
                    response = response
                .on_hover_text_at_pointer(format!("x: {x}, start: {start}, end: {end}\nY: {y:3}, Cb: {cb:3}, Cr: {cr:3}\nR: {r:3}, G: {g:3}, B: {b:3}\nr: {red_chromaticity:.2}, g: {green_chromaticity:.2}, b: {blue_chromaticity:.2}\nH: {h}, S: {s}, V: {v}"));
                }
            }
        }

        let scan_lines = match self.direction {
            Direction::Horizontal => image_segments.scan_grid.horizontal_scan_lines,
            Direction::Vertical => image_segments.scan_grid.vertical_scan_lines,
        };

        for scanline in scan_lines {
            for segment in scanline.segments {
                self.draw_segment(scanline.position as f32, self.direction, segment, &painter);
            }
        }

        response
    }
}

impl ImageSegmentsPanel {
    fn draw_segment(
        &self,
        position: f32,
        direction: Direction,
        segment: Segment,
        painter: &TwixPainter<Pixel>,
    ) {
        let ycbcr_color = segment.color;
        let rgb_color = Rgb::from(ycbcr_color);
        let (start, end) = match direction {
            Direction::Horizontal => (
                point![segment.start as f32, position],
                point![segment.end as f32, position],
            ),
            Direction::Vertical => (
                point![position, segment.start as f32],
                point![position, segment.end as f32],
            ),
        };
        let original_color = Color32::from_rgb(rgb_color.red, rgb_color.green, rgb_color.blue);
        let high_color = Color32::YELLOW;
        let chromaticity = rgb_color.convert_to_rgchromaticity();
        let visualized_color = match self.color_mode {
            ColorMode::Original => original_color,
            ColorMode::FieldColor => match segment.field_color {
                types::color::Intensity::Low => original_color,
                types::color::Intensity::High => high_color,
            },
            ColorMode::Y => Color32::from_gray(ycbcr_color.y),
            ColorMode::Cb => Color32::from_gray(ycbcr_color.cb),
            ColorMode::Cr => Color32::from_gray(ycbcr_color.cr),
            ColorMode::Red => Color32::from_gray(rgb_color.red),
            ColorMode::Green => Color32::from_gray(rgb_color.green),
            ColorMode::Blue => Color32::from_gray(rgb_color.blue),
            ColorMode::RedChromaticity => Color32::from_gray((chromaticity.red * 255.0) as u8),
            ColorMode::GreenChromaticity => Color32::from_gray((chromaticity.green * 255.0) as u8),
            ColorMode::BlueChromaticity => {
                Color32::from_gray(((1.0 - chromaticity.red - chromaticity.green) * 255.0) as u8)
            }
        };
        painter.line_segment(start, end, Stroke::new(4.0, visualized_color));
        painter.line_segment(
            start - vector![1.0, 0.0],
            start + vector![1.0, 0.0],
            Stroke::new(1.0, Color32::from_rgb(0, 0, 255)),
        );
    }
}

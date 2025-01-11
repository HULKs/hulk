use std::sync::Arc;

use eframe::{
    egui::{ComboBox, Response, Ui, Widget},
    epaint::{Color32, Stroke},
};
use linear_algebra::{point, vector, Vector2};
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use serde_json::{json, Value};
use types::{
    camera_position::CameraPosition,
    color::{Hsv, RgChromaticity, Rgb},
    image_segments::{Direction, EdgeType, ImageSegments, Segment},
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
            Ok(None) => return ui.label("no data available"),
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
                    let rgb = Rgb::from(ycbcr_color);
                    let r = rgb.red;
                    let g = rgb.green;
                    let b = rgb.blue;
                    let chromaticity = RgChromaticity::from(rgb);
                    let red_chromaticity = chromaticity.red;
                    let green_chromaticity = chromaticity.green;
                    let blue_chromaticity = 1.0 - chromaticity.red - chromaticity.green;
                    let hsv = Hsv::from(rgb);
                    let h = hsv.hue;
                    let s = hsv.saturation;
                    let v = hsv.value;
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
        let rgb = Rgb::from(ycbcr_color);
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
        let original_color = Color32::from_rgb(rgb.red, rgb.green, rgb.blue);
        let high_color = Color32::YELLOW;
        let chromaticity = RgChromaticity::from(rgb);
        let visualized_color = match self.color_mode {
            ColorMode::Original => original_color,
            ColorMode::FieldColor => match segment.field_color {
                types::color::Intensity::Low => original_color,
                types::color::Intensity::High => high_color,
            },
            ColorMode::Y => Color32::from_gray(ycbcr_color.y),
            ColorMode::Cb => Color32::from_gray(ycbcr_color.cb),
            ColorMode::Cr => Color32::from_gray(ycbcr_color.cr),
            ColorMode::Red => Color32::from_gray(rgb.red),
            ColorMode::Green => Color32::from_gray(rgb.green),
            ColorMode::Blue => Color32::from_gray(rgb.blue),
            ColorMode::RedChromaticity => Color32::from_gray((chromaticity.red * 255.0) as u8),
            ColorMode::GreenChromaticity => Color32::from_gray((chromaticity.green * 255.0) as u8),
            ColorMode::BlueChromaticity => {
                Color32::from_gray(((1.0 - chromaticity.red - chromaticity.green) * 255.0) as u8)
            }
        };

        const SEGMENT_WIDTH: f32 = 4.0;
        const END_MARKER_WIDTH: f32 = 0.25;

        let (main_axis, other_axis) = match direction {
            Direction::Horizontal => (Vector2::x_axis(), Vector2::y_axis()),
            Direction::Vertical => (Vector2::y_axis(), Vector2::x_axis()),
        };

        let main_axis_offset = main_axis * END_MARKER_WIDTH / 2.0;
        let other_axis_offset = other_axis * SEGMENT_WIDTH / 3.0;

        painter.line_segment(start, end, Stroke::new(SEGMENT_WIDTH, visualized_color));

        painter.line_segment(
            start + main_axis_offset + other_axis_offset,
            start + main_axis_offset - other_axis_offset,
            Stroke::new(
                END_MARKER_WIDTH,
                edge_type_to_color(segment.start_edge_type),
            ),
        );
        painter.line_segment(
            end - main_axis_offset + other_axis_offset,
            end - main_axis_offset - other_axis_offset,
            Stroke::new(END_MARKER_WIDTH, edge_type_to_color(segment.end_edge_type)),
        );
    }
}

pub fn edge_type_to_color(edge_type: EdgeType) -> Color32 {
    match edge_type {
        EdgeType::Rising => Color32::RED,
        EdgeType::Falling => Color32::BLUE,
        EdgeType::ImageBorder => Color32::GOLD,
        EdgeType::LimbBorder => Color32::BLACK,
    }
}

use std::sync::Arc;

// use communication::messages::TextOrBinary;
// use eframe::egui::{Response, Slider, Ui, Widget, WidgetText};
use eframe::egui::{
    emath,
    epaint::{self, CubicBezierShape, PathShape, QuadraticBezierShape},
    pos2, Color32, Context, Frame, Grid, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, Widget, Window,
};
// use log::error;
use nalgebra::Vector3;
// use parameters::directory::Scope;
use serde_json::Value;

use crate::{log_error::LogError, nao::Nao, panel::Panel, value_buffer::BufferHandle};

pub struct ScribbleCalibrationPanel {
    nao: Arc<Nao>,
    top_camera: BufferHandle<Vector3<f32>>,
    bottom_camera: BufferHandle<Vector3<f32>>,

    line_1_goal: bool,
    line_2_penalty_horizontal: bool,
    line_3_penalty_left: bool,
    line_4_penalty_right: bool,

    /// The control points. The [`Self::degree`] first of them are used.
    lines: [(Pos2, Pos2); 4],

    /// Stroke for Bézier curve.
    stroke: Stroke,
}

impl Panel for ScribbleCalibrationPanel {
    const NAME: &'static str = "Scribble Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let top_camera = nao.subscribe_value(
            "parameters.camera_matrix_parameters.vision_top.extrinsic_rotations".to_string(),
        );
        let bottom_camera = nao.subscribe_value(
            "parameters.camera_matrix_parameters.vision_bottom.extrinsic_rotations".to_string(),
        );

        Self {
            nao,
            top_camera,
            bottom_camera,

            line_1_goal: true,
            line_2_penalty_horizontal: true,
            line_3_penalty_left: true,
            line_4_penalty_right: true,

            lines: [
                (pos2(50.0, 50.0), pos2(250.0, 50.0)),
                (pos2(50.0, 200.0), pos2(250.0, 200.0)),
                (pos2(110.0, 100.0), pos2(100.0, 150.0)),
                (pos2(190.0, 100.0), pos2(200.0, 150.0)),
            ],
            stroke: Stroke::new(1.0, Color32::from_rgb(25, 200, 100)),
        }
    }
}

impl Widget for &mut ScribbleCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.collapsing("Lines", |ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.line_1_goal, "Line 1: Goal line");
                    ui.checkbox(
                        &mut self.line_2_penalty_horizontal,
                        "Line 2: Penalty horizontal",
                    );
                    ui.checkbox(&mut self.line_3_penalty_left, "Line 3: Penalty left");
                    ui.checkbox(&mut self.line_4_penalty_right, "Line 4: Penalty right");
                });
            });

            ui.separator();

            self.ui_content(ui);
        })
        .response
    }
}

impl ScribbleCalibrationPanel {
    fn int_to_bool(&self, number: usize) -> bool {
        match number {
            0 => self.line_1_goal,
            1 => self.line_2_penalty_horizontal,
            2 => self.line_3_penalty_left,
            3 => self.line_4_penalty_right,
            _ => false,
        }
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> Response {
        let (response, painter) =
            ui.allocate_painter(Vec2::new(ui.available_width(), 300.0), Sense::hover());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );

        let line_states: Vec<bool> = (0..self.lines.len()).map(|i| self.int_to_bool(i)).collect();

        self.lines
            .iter_mut()
            .enumerate()
            .for_each(|(i, (start, end))| {
                if line_states[i] {
                    let start_screen = to_screen * *start;
                    let end_screen = to_screen * *end;

                    let radius = 5.0;
                    let start_response = ui.interact(
                        Rect::from_center_size(start_screen, Vec2::splat(radius * 2.0)),
                        response.id.with(format!("start_{}", i)),
                        Sense::drag(),
                    );
                    if start_response.dragged() {
                        *start += start_response.drag_delta() / to_screen.scale();
                        *start = to_screen.from().clamp(*start);
                    }
                    painter.add(Shape::circle_filled(start_screen, radius, Color32::WHITE));

                    let end_response = ui.interact(
                        Rect::from_center_size(end_screen, Vec2::splat(radius * 2.0)),
                        response.id.with(format!("end_{}", i)),
                        Sense::drag(),
                    );
                    if end_response.dragged() {
                        *end += end_response.drag_delta() / to_screen.scale();
                        *end = to_screen.from().clamp(*end);
                    }
                    painter.add(Shape::circle_filled(end_screen, radius, Color32::WHITE));

                    // Update the line with the new positions
                    let updated_start_screen = to_screen * *start;
                    let updated_end_screen = to_screen * *end;
                    painter.add(PathShape::line(
                        vec![updated_start_screen, updated_end_screen],
                        self.stroke,
                    ));
                }
            });

        response
    }
}

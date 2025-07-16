use std::sync::Arc;

use calibration::goal_and_penalty_box::LineType;
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use coordinate_systems::Pixel;
use eframe::egui::{
    popup_below_widget, vec2, Align2, Button, Color32, Key, PopupCloseBehavior, Rect, Response,
    Sense, Shape, SizeHint, Stroke, TextureOptions, Ui, UiBuilder, Widget,
};
use geometry::{line_segment::LineSegment, rectangle::Rectangle};
use linear_algebra::{distance, point, vector, Point2};
use projection::camera_matrix::CameraMatrix;
use serde_json::Value;
use types::{camera_position::CameraPosition, jpeg::JpegImage};

use crate::{
    nao::Nao,
    panel::Panel,
    panels::camera_calibration::optimization::{
        DrawnLine, SavedMeasurement, SemiAutomaticCalibrationContext,
    },
    panels::image::cycler_selector::{VisionCycler, VisionCyclerSelector},
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
    zoom_and_pan::ZoomAndPanTransform,
};

const KEYPOINT_RADIUS: f32 = 5.0;
const KEYPOINT_COLOR: Color32 = Color32::from_rgb(155, 0, 0);
const LINE_STROKE: Stroke = Stroke {
    width: 3.0,
    color: Color32::from_rgb(55, 80, 250),
};

#[derive(Clone, Copy)]
enum UserState {
    Idle,
    DrawingLine {
        start: Point2<Pixel>,
        line_type: Option<LineType>,
    },
    AnnotatingLine {
        line: LineSegment<Pixel>,
    },
}

pub struct SemiAutomaticCameraCalibrationPanel {
    nao: Arc<Nao>,
    top_camera: BufferHandle<CameraMatrix>,
    bottom_camera: BufferHandle<CameraMatrix>,

    image_buffer: BufferHandle<JpegImage>,
    zoom_and_pan: ZoomAndPanTransform,
    cycler: VisionCycler,

    user_state: UserState,
    drawn_lines: Vec<DrawnLine>,
    saved_measurements: Vec<SavedMeasurement>,
    optimization: SemiAutomaticCalibrationContext,
}

impl Panel for SemiAutomaticCameraCalibrationPanel {
    const NAME: &'static str = "Semi-Automatic Camera Calibration";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let top_camera =
            nao.subscribe_value("Control.main_outputs.uncalibrated_camera_matrices.top");
        let bottom_camera =
            nao.subscribe_value("Control.main_outputs.uncalibrated_camera_matrices.bottom");

        let cycler = value
            .and_then(|value| {
                let string = value.get("cycler")?.as_str()?;
                VisionCycler::try_from(string).ok()
            })
            .unwrap_or(VisionCycler::Top);
        let cycler_path = cycler.as_path();

        let image_buffer = {
            let path = format!("{cycler_path}.main_outputs.image.jpeg");
            nao.subscribe_value(path)
        };

        Self {
            nao: nao.clone(),
            top_camera,
            bottom_camera,
            image_buffer,
            zoom_and_pan: ZoomAndPanTransform::default(),
            cycler,
            user_state: UserState::Idle,
            drawn_lines: Vec::new(),
            saved_measurements: Vec::new(),
            optimization: SemiAutomaticCalibrationContext::new(nao.clone()),
        }
    }
}

impl Widget for &mut SemiAutomaticCameraCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut cycler_selector = VisionCyclerSelector::new(&mut self.cycler);
        if cycler_selector.ui(ui).changed() {
            self.resubscribe();
        }

        ui.horizontal(|ui| {
            if ui.button("Next (and save lines)").clicked() {
                let result = self.save_measurement();
                if let Err(error) = result {
                    println!("Error: {error}");
                }
                let result = self
                    .optimization
                    .run_optimization(self.saved_measurements.clone());

                if let Err(error) = result {
                    println!("Error: {error}");
                }
                self.drawn_lines.clear();
            }

            if ui.button("Clear Drawings").clicked() {
                self.drawn_lines.clear();
            }
            if ui.button("Reset Calibration").clicked() {
                self.saved_measurements.clear();
                let result = self.optimization.reset();
                if let Err(error) = result {
                    println!("Error: {error}");
                }
            }

            let save_to_head = ui.add_enabled(
                self.optimization.is_converged(),
                Button::new("Save to head"),
            );
            if save_to_head.clicked() {
                let result = self.optimization.save_to_head();
                if let Err(error) = result {
                    println!("Error: {error}");
                }
                self.saved_measurements.clear();
                let result = self.optimization.reset();
                if let Err(error) = result {
                    println!("Error: {error}");
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(format!("# drawn lines: {}", self.drawn_lines.len()));
            ui.label(format!("# measurements: {}", self.saved_measurements.len()));
        });
        ui.separator();

        ui.vertical(|ui| {
            if let Some(report) = self.optimization.optimization_report() {
                ui.horizontal(|ui| {
                    ui.label(format!("Residual: {}", report.objective_function));
                    ui.label(format!("Termination Reason: {:?}", report.termination));
                    ui.label(format!("Iterations: {}", report.number_of_evaluations));
                });
            }
            self.ui_content(ui);
        })
        .response
    }
}

impl SemiAutomaticCameraCalibrationPanel {
    fn resubscribe(&mut self) {
        let cycler_path = self.cycler.as_path();
        self.image_buffer = self
            .nao
            .subscribe_value(format!("{cycler_path}.main_outputs.image.jpeg"));
    }

    fn show_image(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let context = painter.context();

        let image_identifier = format!("bytes://image-{:?}", self.cycler);
        let image = {
            let jpeg = self
                .image_buffer
                .get_last_value()?
                .ok_or_else(|| eyre!("no image available"))?;
            context.forget_image(&image_identifier);
            context.include_bytes(image_identifier.clone(), jpeg.data);
            context
                .try_load_texture(
                    &image_identifier,
                    TextureOptions::NEAREST,
                    SizeHint::Size(640, 480),
                )?
                .texture_id()
                .unwrap()
        };

        painter.image(
            image,
            Rectangle {
                min: point!(0.0, 0.0),
                max: point!(640.0, 480.0),
            },
        );
        Ok(())
    }

    pub fn save_measurement(&mut self) -> Result<()> {
        if self.cycler == VisionCycler::Top {
            self.saved_measurements.push(SavedMeasurement {
                camera_position: CameraPosition::Top,
                camera_matrix: self
                    .top_camera
                    .get_last_value()?
                    .wrap_err("no camera_matrix found")?,
                drawn_lines: self.drawn_lines.clone(),
            });
        } else {
            self.saved_measurements.push(SavedMeasurement {
                camera_position: CameraPosition::Bottom,
                camera_matrix: self
                    .bottom_camera
                    .get_last_value()?
                    .wrap_err("no camera_matrix found")?,
                drawn_lines: self.drawn_lines.clone(),
            });
        }
        Ok(())
    }

    fn line_type_ui(&mut self, ui: &mut Ui) -> Option<LineType> {
        let line_types = [
            LineType::Goal,
            LineType::LeftPenaltyArea,
            LineType::RightPenaltyArea,
            LineType::FrontPenaltyArea,
            LineType::LeftGoalArea,
            LineType::RightGoalArea,
            LineType::FrontGoalArea,
        ];
        for line_type in &line_types {
            if ui
                .selectable_label(false, format!("{line_type:?}"))
                .clicked()
            {
                return Some(*line_type);
            }
        }
        None
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> Response {
        let (response, mut painter) = TwixPainter::allocate(
            ui,
            vector![640.0, 480.0],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );
        self.zoom_and_pan.apply(ui, &mut painter, &response);

        if let Err(error) = self.show_image(&painter) {
            ui.scope_builder(UiBuilder::new().max_rect(response.rect), |ui| {
                ui.label(format!("{error}"))
            });
        };

        for line in &self.drawn_lines {
            painter.line_segment(line.line_segment.0, line.line_segment.1, LINE_STROKE);
            painter.add(Shape::circle_filled(
                painter.transform_world_to_pixel(line.line_segment.0),
                KEYPOINT_RADIUS,
                KEYPOINT_COLOR,
            ));
            painter.add(Shape::circle_filled(
                painter.transform_world_to_pixel(line.line_segment.1),
                KEYPOINT_RADIUS,
                KEYPOINT_COLOR,
            ));
            painter.floating_text(
                line.line_segment.center(),
                Align2::CENTER_CENTER,
                format!("{:?}", line.line_type),
                Default::default(),
                Color32::BLACK,
            );
        }

        let primary_clicked = ui.input(|reader| reader.pointer.primary_clicked());
        let escape_pressed = ui.input(|reader| reader.key_pressed(Key::Escape));
        let pointer_position = response
            .hover_pos()
            .map(|pos| painter.transform_pixel_to_world(pos));

        let id = ui.auto_id_with("line-segment-selector");
        let popup_id = id.with("line-segment-selector");

        self.user_state = match (self.user_state, pointer_position) {
            _ if escape_pressed => UserState::Idle,
            (UserState::Idle, Some(pointer)) if primary_clicked => {
                let closest_line_index = self.drawn_lines.iter().position(|line| {
                    distance(line.line_segment.0, pointer) < KEYPOINT_RADIUS
                        || distance(line.line_segment.1, pointer) < KEYPOINT_RADIUS
                });
                if let Some(line_index) = closest_line_index {
                    let modified_line = self.drawn_lines.remove(line_index);
                    let start = if distance(modified_line.line_segment.0, pointer) < KEYPOINT_RADIUS
                    {
                        modified_line.line_segment.1
                    } else {
                        modified_line.line_segment.0
                    };
                    UserState::DrawingLine {
                        start,
                        line_type: Some(modified_line.line_type),
                    }
                } else {
                    UserState::DrawingLine {
                        start: pointer,
                        line_type: None,
                    }
                }
            }
            (UserState::DrawingLine { start, line_type }, Some(end)) if primary_clicked => {
                painter.line_segment(start, end, LINE_STROKE);
                painter.add(Shape::circle_filled(
                    painter.transform_world_to_pixel(start),
                    KEYPOINT_RADIUS,
                    KEYPOINT_COLOR,
                ));
                if let Some(line_type) = line_type {
                    self.drawn_lines.push(DrawnLine {
                        line_segment: LineSegment(start, end),
                        line_type,
                    });
                    UserState::Idle
                } else {
                    ui.memory_mut(|memory| {
                        memory.open_popup(popup_id);
                    });
                    UserState::AnnotatingLine {
                        line: LineSegment(start, end),
                    }
                }
            }
            (UserState::DrawingLine { start, line_type }, Some(end)) if !primary_clicked => {
                painter.line_segment(start, end, LINE_STROKE);
                painter.add(Shape::circle_filled(
                    painter.transform_world_to_pixel(start),
                    KEYPOINT_RADIUS,
                    KEYPOINT_COLOR,
                ));
                UserState::DrawingLine { start, line_type }
            }
            (UserState::AnnotatingLine { line }, _) => {
                painter.line_segment(line.0, line.1, LINE_STROKE);
                let popup_position = painter.transform_world_to_pixel(line.1);

                let local_response = ui.interact(
                    Rect::from_min_size(popup_position, vec2(200.0, 0.0)),
                    id,
                    Sense::CLICK,
                );

                let response = popup_below_widget(
                    ui,
                    popup_id,
                    &local_response,
                    PopupCloseBehavior::CloseOnClickOutside,
                    |ui| self.line_type_ui(ui),
                );

                match response {
                    Some(Some(line_type)) => {
                        self.drawn_lines.push(DrawnLine {
                            line_segment: line,
                            line_type,
                        });
                        UserState::Idle
                    }
                    Some(None) => UserState::AnnotatingLine { line },
                    None => UserState::Idle,
                }
            }
            (previous_state, _) => previous_state,
        };
        response
    }
}

use std::{collections::HashMap, sync::Arc};

use calibration::goal_and_penalty_box::LineType;
use color_eyre::{eyre::{eyre, ContextCompat}, Result};
use coordinate_systems::Pixel;
use eframe::egui::{
    Align, Align2, Color32, ColorImage, ComboBox, Response, Shape, SizeHint, Stroke, TextureOptions, Ui, UiBuilder, Widget
};
use geometry::{line_segment::LineSegment, rectangle::Rectangle};
use image::RgbImage;
use linear_algebra::{distance, point, vector, Point2};
use projection::camera_matrix::CameraMatrix;
use serde_json::Value;

use types::{jpeg::JpegImage, ycbcr422_image::YCbCr422Image};

use crate::{
    nao::Nao,
    panel::Panel,
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
};

use crate::panels::image::cycler_selector::{VisionCycler, VisionCyclerSelector};

use super::optimization::{DrawnLine, RobotLookState, SavedMeasurement, SavedMeasurements, SemiAutomaticCalibrationContext};

const KEYPOINT_RADIUS: f32 = 5.0;
const KEYPOINT_COLOR: Color32 = Color32::from_rgb(155, 0, 0);

enum RawOrJpeg {
    Raw(BufferHandle<YCbCr422Image>),
    Jpeg(BufferHandle<JpegImage>),
}

#[derive(Clone, Copy)]
enum UserState {
    Idle,
    DrawingLine { start: Point2<Pixel> },
}

pub struct SemiAutomaticCameraCalibrationPanel {
    nao: Arc<Nao>,
    top_camera: BufferHandle<CameraMatrix>,
    bottom_camera: BufferHandle<CameraMatrix>,

    image_buffer: RawOrJpeg,
    cycler: VisionCycler,

    user_state: UserState,
    head_pos: RobotLookState,
    drawn_lines: Vec<DrawnLine>,
    line_type: LineType,
    saved_measurements: HashMap<RobotLookState, SavedMeasurement>,
    optimization: SemiAutomaticCalibrationContext,
    stroke: Stroke,
}

impl Panel for SemiAutomaticCameraCalibrationPanel {
    const NAME: &'static str = "semi-automatic camera calibration";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let top_camera = nao.subscribe_value("Control.main_outputs.uncalibrated_camera_matrices.top");
        let bottom_camera = nao.subscribe_value("Control.main_outputs.uncalibrated_camera_matrices.bottom");


        let cycler = value
            .and_then(|value| {
                let string = value.get("cycler")?.as_str()?;
                VisionCycler::try_from(string).ok()
            })
            .unwrap_or(VisionCycler::Top);
        let cycler_path = cycler.as_path();

        let is_jpeg = value
            .and_then(|value| value.get("is_jpeg"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        let image_buffer = if is_jpeg {
            let path = format!("{cycler_path}.main_outputs.image.jpeg");
            RawOrJpeg::Jpeg(nao.subscribe_value(path))
        } else {
            let path = format!("{cycler_path}.main_outputs.image");
            RawOrJpeg::Raw(nao.subscribe_value(path))
        };

        Self {
            nao: nao.clone(),
            top_camera,
            bottom_camera,

            image_buffer,
            cycler,

            user_state: UserState::Idle,
            head_pos: RobotLookState::CenterCameraTop,
            drawn_lines: Vec::new(),
            line_type: LineType::Goal,
            saved_measurements: HashMap::new(),
            optimization: SemiAutomaticCalibrationContext::new(nao.clone()),
            stroke: Stroke::new(3.0, Color32::from_rgb(55, 80, 250)),
        }
    }
}

impl Widget for &mut SemiAutomaticCameraCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let jpeg = matches!(self.image_buffer, RawOrJpeg::Jpeg(_));
        let mut cycler_selector = VisionCyclerSelector::new(&mut self.cycler);
        if cycler_selector.ui(ui).changed() {
            self.resubscribe(jpeg);
        }
        ui.horizontal(|ui| {
            if ui.button("save measurements").clicked() {
                let result = self.save_measurement();
                if let Err(error) = result {
                    println!("Error: {}", error.to_string());
                }
                self.drawn_lines = Vec::new();
            }
            if ui.button("optimize / save to Head").clicked() {
                let result = self.optimization.run_optimization(SavedMeasurements{ measurements: self.saved_measurements.clone()});
                if let Err(error) = result {
                    println!("Error: {}", error.to_string());
                }
                println!("{:?}", self.saved_measurements)
            }
        
            if ui.button("Clear Drawings").clicked() {
                self.drawn_lines = Vec::new();
            }
            if ui.button("Clear Measurements").clicked() {
                self.saved_measurements = HashMap::new();
            }
        });
        ui.horizontal(|ui| {
            ui.label(format!("# drwan lines: {}", self.drawn_lines.len()));
            ui.label(format!("# measurements: {}", self.saved_measurements.len()));
        });
        ui.separator();
        ui.vertical(|ui| {
            ComboBox::from_label("Head position:")
                .selected_text(format!("{:?}",self.head_pos))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.head_pos,
                        RobotLookState::CenterCameraTop,
                        "Top-Center",
                    );
                    ui.selectable_value(
                        &mut self.head_pos,
                        RobotLookState::LeftCameraTop,
                        "Top-Left",
                    );
                    ui.selectable_value(
                        &mut self.head_pos,
                        RobotLookState::RightCameraTop,
                        "Top-Right",
                    );
                    ui.selectable_value(
                        &mut self.head_pos,
                        RobotLookState::CenterCameraBottom,
                        "Bottom-Center",
                    );
                    ui.selectable_value(
                        &mut self.head_pos,
                        RobotLookState::LeftCameraBottom,
                        "Bottom-Left",
                    );
                    ui.selectable_value(
                        &mut self.head_pos,
                        RobotLookState::RightCameraBottom,
                        "Bottom-Right",
                    );
                });
            ComboBox::from_label("Line type:")
                .selected_text(format!("{:?}",self.line_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.line_type,
                        LineType::Goal,
                        "Goal",
                    );
                    ui.selectable_value(
                        &mut self.line_type,
                        LineType::LeftPenaltyArea,
                        "Penalty-Left",
                    );
                    ui.selectable_value(
                        &mut self.line_type,
                        LineType::RightPenaltyArea,
                        "Penalty-Right",
                    );
                    ui.selectable_value(
                        &mut self.line_type,
                        LineType::FrontPenaltyArea,
                        "Penalty-Front",
                    );
                    ui.selectable_value(
                        &mut self.line_type,
                        LineType::LeftGoalArea,
                        "Goal-Area-Left",
                    );
                    ui.selectable_value(
                        &mut self.line_type,
                        LineType::RightGoalArea,
                        "Goal-Area-Right",
                    );
                    ui.selectable_value(
                        &mut self.line_type,
                        LineType::FrontGoalArea,
                        "Goal-Area-Front",
                    );
                });
        });
        self.ui_content(ui)

    }
}

impl SemiAutomaticCameraCalibrationPanel {
    // TODO: implement this only once
    fn resubscribe(&mut self, jpeg: bool) {
        let cycler_path = self.cycler.as_path();
        self.image_buffer = if jpeg {
            RawOrJpeg::Jpeg(
                self.nao
                    .subscribe_value(format!("{cycler_path}.main_outputs.image.jpeg")),
            )
        } else {
            RawOrJpeg::Raw(
                self.nao
                    .subscribe_value(format!("{cycler_path}.main_outputs.image")),
            )
        };
    }

    fn show_image(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let context = painter.context();

        let image_identifier = format!("bytes://image-{:?}", self.cycler);
        let image = match &self.image_buffer {
            RawOrJpeg::Raw(buffer) => {
                let ycbcr = buffer
                    .get_last_value()?
                    .ok_or_else(|| eyre!("no image available"))?;
                let image = ColorImage::from_rgb(
                    [ycbcr.width() as usize, ycbcr.height() as usize],
                    RgbImage::from(ycbcr).as_raw(),
                );
                context
                    .load_texture(&image_identifier, image, TextureOptions::NEAREST)
                    .id()
            }
            RawOrJpeg::Jpeg(buffer) => {
                let jpeg = buffer
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
            }
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

    pub fn save_measurement(&mut self) -> Result<()>{
        if self.cycler == VisionCycler::Top {
            self.saved_measurements.insert(self.head_pos, SavedMeasurement{
                camera_matrix: self.top_camera.get_last_value()?.wrap_err("no camera_matrix found")?,
                drawn_lines: self.drawn_lines.clone(),
            });
        } else {
            
            self.saved_measurements.insert(self.head_pos, SavedMeasurement{
                camera_matrix: self.bottom_camera.get_last_value()?.wrap_err("no camera_matrix found")?,
                drawn_lines: self.drawn_lines.clone(),
            });
        }
        Ok(())
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> Response {
        let (response, mut painter) = TwixPainter::allocate(
            ui,
            vector![640.0, 480.0],
            point![0.0, 0.0],
            Orientation::LeftHanded,
        );

        if let Err(error) = self.show_image(&painter) {
            ui.scope_builder(UiBuilder::new().max_rect(response.rect), |ui| {
                ui.label(format!("{error}"))
            });
        };

        for line in &self.drawn_lines {
            painter.line_segment(line.line_segment.0, line.line_segment.1, self.stroke);
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
            painter.floating_text(line.line_segment.center(), Align2::CENTER_CENTER, format!("{:?}", line.line_type), Default::default(), Color32::BLACK);
        }

        let primary_clicked = ui.input(|reader| reader.pointer.primary_pressed());
        let pointer_position = response
            .hover_pos()
            .map(|pos| painter.transform_pixel_to_world(pos));

        self.user_state = match (self.user_state, pointer_position) {
            (UserState::Idle, _) if !primary_clicked => UserState::Idle,
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
                    UserState::DrawingLine { start }
                } else {
                    UserState::DrawingLine { start: pointer }
                }
            }
            (UserState::DrawingLine { start }, Some(end)) if primary_clicked => {
                self.drawn_lines.push(DrawnLine {
                    line_segment: LineSegment(start, end),
                    line_type: self.line_type,
                });
                UserState::Idle
            }
            (UserState::DrawingLine { start }, Some(end)) if !primary_clicked => {
                painter.line_segment(start, end, self.stroke);
                painter.add(Shape::circle_filled(
                    painter.transform_world_to_pixel(start),
                    KEYPOINT_RADIUS,
                    KEYPOINT_COLOR,
                ));
                UserState::DrawingLine { start }
            }
            (_, _) => self.user_state,
        };
        response
    }
}

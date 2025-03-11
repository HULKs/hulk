use std::sync::Arc;

use color_eyre::{eyre::eyre, Result};
use coordinate_systems::Pixel;
use eframe::egui::{
    Color32, ColorImage, Response, Shape, SizeHint, Stroke, TextureOptions, Ui, UiBuilder, Widget,
};
use geometry::{line_segment::LineSegment, rectangle::Rectangle};
use image::RgbImage;
use linear_algebra::{distance, point, vector, Point2};
use nalgebra::Vector3;
use serde_json::Value;

use types::{jpeg::JpegImage, ycbcr422_image::YCbCr422Image};

use crate::{
    nao::Nao,
    panel::Panel,
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
};

use super::image::cycler_selector::{VisionCycler, VisionCyclerSelector};

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

pub struct ScribbleCalibrationPanel {
    nao: Arc<Nao>,
    _top_camera: BufferHandle<Vector3<f32>>,
    _bottom_camera: BufferHandle<Vector3<f32>>,

    image_buffer: RawOrJpeg,
    cycler: VisionCycler,

    state: UserState,
    lines: Vec<LineSegment<Pixel>>,
    stroke: Stroke,
}

impl Panel for ScribbleCalibrationPanel {
    const NAME: &'static str = "Scribble Calibration";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let _top_camera = nao.subscribe_value(
            "parameters.camera_matrix_parameters.vision_top.extrinsic_rotations".to_string(),
        );
        let _bottom_camera = nao.subscribe_value(
            "parameters.camera_matrix_parameters.vision_bottom.extrinsic_rotations".to_string(),
        );

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
            nao,
            _top_camera,
            _bottom_camera,

            image_buffer,
            cycler,

            state: UserState::Idle,
            lines: Vec::new(),
            stroke: Stroke::new(3.0, Color32::from_rgb(55, 80, 250)),
        }
    }
}

impl Widget for &mut ScribbleCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let jpeg = matches!(self.image_buffer, RawOrJpeg::Jpeg(_));
        let mut cycler_selector = VisionCyclerSelector::new(&mut self.cycler);
        if cycler_selector.ui(ui).changed() {
            self.resubscribe(jpeg);
        }

        self.ui_content(ui)
    }
}

impl ScribbleCalibrationPanel {
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

        for line in &self.lines {
            painter.line_segment(line.0, line.1, self.stroke);
            painter.add(Shape::circle_filled(
                painter.transform_world_to_pixel(line.0),
                KEYPOINT_RADIUS,
                KEYPOINT_COLOR,
            ));
            painter.add(Shape::circle_filled(
                painter.transform_world_to_pixel(line.1),
                KEYPOINT_RADIUS,
                KEYPOINT_COLOR,
            ));
        }

        let primary_clicked = ui.input(|reader| reader.pointer.primary_pressed());
        let pointer_position = response
            .hover_pos()
            .map(|pos| painter.transform_pixel_to_world(pos));

        self.state = match (self.state, pointer_position) {
            (UserState::Idle, _) if !primary_clicked => UserState::Idle,
            (UserState::Idle, Some(pointer)) if primary_clicked => {
                let closest_line_index = self.lines.iter().position(|line| {
                    distance(line.0, pointer) < KEYPOINT_RADIUS
                        || distance(line.1, pointer) < KEYPOINT_RADIUS
                });
                if let Some(line_index) = closest_line_index {
                    let modified_line = self.lines.remove(line_index);
                    let start = if distance(modified_line.0, pointer) < KEYPOINT_RADIUS {
                        modified_line.1
                    } else {
                        modified_line.0
                    };
                    UserState::DrawingLine { start }
                } else {
                    UserState::DrawingLine { start: pointer }
                }
            }
            (UserState::DrawingLine { start }, Some(end)) if primary_clicked => {
                self.lines.push(LineSegment::new(start, end));
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
            (_, _) => self.state,
        };
        response
    }
}

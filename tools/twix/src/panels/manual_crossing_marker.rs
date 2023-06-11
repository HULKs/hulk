use std::sync::Arc;

use color_eyre::{eyre::eyre, Result};
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::{
    egui::{Key, Modifiers, Response, TextureOptions, Ui, Widget},
    epaint::{Color32, Rect, Stroke},
};
use egui_extras::RetainedImage;
use nalgebra::{point, vector, Point2, Similarity2};

use crate::{
    image_buffer::ImageBuffer,
    nao::Nao,
    panel::Panel,
    twix_painter::{CoordinateSystem, TwixPainter},
};

#[derive(Default)]
struct Crossings {
    left_field_corner: Option<Point2<f32>>,
    left_penalty_field: Option<Point2<f32>>,
    left_penalty_corner: Option<Point2<f32>>,
    left_goal_field: Option<Point2<f32>>,
    left_goal_corner: Option<Point2<f32>>,
    right_goal_corner: Option<Point2<f32>>,
    right_goal_field: Option<Point2<f32>>,
    right_penalty_corner: Option<Point2<f32>>,
    right_penalty_field: Option<Point2<f32>>,
    right_field_corner: Option<Point2<f32>>,
}

pub struct ManualCrossingMarker {
    _nao: Arc<Nao>,
    image_buffer: ImageBuffer,
    crossings: Crossings,
}

impl Panel for ManualCrossingMarker {
    const NAME: &'static str = "Manual Crossing Marker";

    fn new(nao: Arc<Nao>, _value: Option<&serde_json::Value>) -> Self {
        let output = CyclerOutput {
            cycler: Cycler::VisionTop,
            output: Output::Main {
                path: "image.jpeg".to_string(),
            },
        };
        let image_buffer = nao.subscribe_image(output);
        Self {
            _nao: nao,
            image_buffer,
            crossings: Default::default(),
        }
    }
}

impl Widget for &mut ManualCrossingMarker {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.show_image(ui) {
            Ok(response) => response,
            Err(error) => ui.label(format!("{error:#?}")),
        }
    }
}

impl ManualCrossingMarker {
    fn show_image(&mut self, ui: &mut Ui) -> Result<Response> {
        let image_data = self
            .image_buffer
            .get_latest()
            .map_err(|error| eyre!("{error}"))?;
        let image_raw = bincode::deserialize::<Vec<u8>>(&image_data)?;
        let image = RetainedImage::from_image_bytes("image", &image_raw)
            .map_err(|error| eyre!("{error}"))?
            .with_options(TextureOptions::NEAREST);
        let image_size = image.size_vec2();
        let width_scale = ui.available_width() / image_size.x;
        let height_scale = ui.available_height() / image_size.y;
        let scale = width_scale.min(height_scale);
        let image_response = image.show_scaled(ui, scale);
        let displayed_image_size = image_size * scale;
        let image_rect = Rect::from_min_size(image_response.rect.left_top(), displayed_image_size);
        let painter = TwixPainter::paint_at(ui, image_rect).with_camera(
            vector![640.0, 480.0],
            Similarity2::identity(),
            CoordinateSystem::LeftHand,
        );

        if let Some(position) = self.crossings.left_field_corner {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.left_penalty_field {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.left_penalty_corner {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.left_goal_field {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.left_goal_corner {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.right_goal_corner {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.right_goal_field {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.right_penalty_corner {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.right_penalty_field {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.right_field_corner {
            self.show_cross(&painter, position);
        }

        if let (Some(start), Some(end)) = (
            self.crossings.left_field_corner,
            self.crossings.left_penalty_field,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.left_penalty_field,
            self.crossings.left_penalty_corner,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.left_penalty_field,
            self.crossings.left_goal_field,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.left_goal_field,
            self.crossings.left_goal_corner,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.left_goal_field,
            self.crossings.right_goal_field,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.left_penalty_corner,
            self.crossings.right_penalty_corner,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.left_goal_corner,
            self.crossings.right_goal_corner,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.right_goal_corner,
            self.crossings.right_goal_field,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.right_penalty_corner,
            self.crossings.right_penalty_field,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.right_goal_field,
            self.crossings.right_penalty_field,
        ) {
            self.show_line_segment(&painter, start, end);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.right_penalty_field,
            self.crossings.right_field_corner,
        ) {
            self.show_line_segment(&painter, start, end);
        }

        if let Some(position) = image_response.hover_pos() {
            let position = painter.transform_pixel_to_world(position);
            painter.line_segment(
                point![position.x, 0.0],
                point![position.x, 480.0],
                Stroke::new(1.0, Color32::WHITE),
            );
            painter.line_segment(
                point![0.0, position.y],
                point![640.0, position.y],
                Stroke::new(1.0, Color32::WHITE),
            );
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Q)) {
                self.crossings.left_field_corner = match self.crossings.left_field_corner {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::W)) {
                self.crossings.left_penalty_field = match self.crossings.left_penalty_field {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::E)) {
                self.crossings.left_penalty_corner = match self.crossings.left_penalty_corner {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::R)) {
                self.crossings.left_goal_field = match self.crossings.left_goal_field {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::T)) {
                self.crossings.left_goal_corner = match self.crossings.left_goal_corner {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Z)) {
                self.crossings.right_goal_corner = match self.crossings.right_goal_corner {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::U)) {
                self.crossings.right_goal_field = match self.crossings.right_goal_field {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::I)) {
                self.crossings.right_penalty_corner = match self.crossings.right_penalty_corner {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::O)) {
                self.crossings.right_penalty_field = match self.crossings.right_penalty_field {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::P)) {
                self.crossings.right_field_corner = match self.crossings.right_field_corner {
                    Some(_) => None,
                    None => Some(position),
                };
            }
        }
        Ok(image_response)
    }

    fn show_cross(&self, painter: &TwixPainter, position: Point2<f32>) {
        painter.line_segment(
            point![position.x, position.y - 3.0],
            point![position.x, position.y + 3.0],
            Stroke::new(0.5, Color32::RED),
        );
        painter.line_segment(
            point![position.x - 3.0, position.y],
            point![position.x + 3.0, position.y],
            Stroke::new(0.5, Color32::RED),
        );
    }

    fn show_line_segment(&self, painter: &TwixPainter, start: Point2<f32>, end: Point2<f32>) {
        painter.line_segment(
            point![start.x, start.y],
            point![end.x, end.y],
            Stroke::new(0.5, Color32::BROWN),
        );
    }
}

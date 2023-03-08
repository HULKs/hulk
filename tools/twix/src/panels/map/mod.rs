use std::sync::Arc;

use eframe::egui::{Response, Ui, Widget};
use nalgebra::{vector, Similarity2, Translation2};
use serde_json::{from_value, json, Value};
use types::{self, FieldDimensions};

use crate::{nao::Nao, panel::Panel, twix_painter::TwixPainter, value_buffer::ValueBuffer};

use self::layer::EnabledLayer;

mod layer;
mod layers;

pub struct MapPanel {
    field_dimensions: ValueBuffer,
    transformation: Similarity2<f32>,

    field: EnabledLayer<layers::Field>,
    image_segments: EnabledLayer<layers::ImageSegments>,
    lines: EnabledLayer<layers::Lines>,
    path_obstacles: EnabledLayer<layers::PathObstacles>,
    obstacles: EnabledLayer<layers::Obstacles>,
    path: EnabledLayer<layers::Path>,
    robot_pose: EnabledLayer<layers::RobotPose>,
    ball_position: EnabledLayer<layers::BallPosition>,
    kick_decisions: EnabledLayer<layers::KickDecisions>,
}

impl Panel for MapPanel {
    const NAME: &'static str = "Map";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let field = EnabledLayer::new(nao.clone(), value, true);
        let image_segments = EnabledLayer::new(nao.clone(), value, false);
        let lines = EnabledLayer::new(nao.clone(), value, true);
        let path_obstacles = EnabledLayer::new(nao.clone(), value, false);
        let obstacles = EnabledLayer::new(nao.clone(), value, false);
        let path = EnabledLayer::new(nao.clone(), value, false);
        let robot_pose = EnabledLayer::new(nao.clone(), value, true);
        let ball_position = EnabledLayer::new(nao.clone(), value, false);
        let kick_decisions = EnabledLayer::new(nao.clone(), value, false);

        let field_dimensions = nao.subscribe_parameter("field_dimensions");
        let transformation = Similarity2::identity();
        Self {
            field_dimensions,
            transformation,

            field,
            image_segments,
            lines,
            path_obstacles,
            obstacles,
            path,
            robot_pose,
            ball_position,
            kick_decisions,
        }
    }

    fn save(&self) -> Value {
        json!({
            "field": self.field.save(),
            "image_segments": self.image_segments.save(),
            "lines": self.lines.save(),
            "path_obstacles": self.path_obstacles.save(),
            "obstacles": self.obstacles.save(),
            "path": self.path.save(),
            "robot_pose": self.robot_pose.save(),
            "ball_position": self.ball_position.save(),
            "kick_decisions": self.kick_decisions.save(),
        })
    }
}

impl Widget for &mut MapPanel {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        ui.menu_button("Overlays", |ui| {
            self.ball_position.checkbox(ui);
            self.field.checkbox(ui);
            self.image_segments.checkbox(ui);
            self.kick_decisions.checkbox(ui);
            self.lines.checkbox(ui);
            self.obstacles.checkbox(ui);
            self.path.checkbox(ui);
            self.path_obstacles.checkbox(ui);
            self.robot_pose.checkbox(ui);
        });

        let field_dimensions: FieldDimensions = match self.field_dimensions.get_latest() {
            Ok(value) => from_value(value).unwrap(),
            Err(error) => return ui.label(format!("{error:?}")),
        };
        let (response, painter) = TwixPainter::allocate_new(ui);
        let mut painter = painter.with_map_transforms(&field_dimensions);
        painter.append_transform(self.transformation);

        // draw largest layers first so they don't obscure smaller ones
        let _ = self.field.paint(&painter, &field_dimensions);
        let _ = self.image_segments.paint(&painter, &field_dimensions);
        let _ = self.lines.paint(&painter, &field_dimensions);
        let _ = self.path_obstacles.paint(&painter, &field_dimensions);
        let _ = self.obstacles.paint(&painter, &field_dimensions);
        let _ = self.path.paint(&painter, &field_dimensions);
        let _ = self.robot_pose.paint(&painter, &field_dimensions);
        let _ = self.ball_position.paint(&painter, &field_dimensions);
        let _ = self.kick_decisions.paint(&painter, &field_dimensions);

        self.apply_zoom_and_pan(ui, &mut painter, &response);
        if response.double_clicked() {
            self.transformation = Similarity2::identity();
        }

        response
    }
}

impl MapPanel {
    fn apply_zoom_and_pan(
        &mut self,
        ui: &mut Ui,
        painter: &mut TwixPainter,
        response: &Response,
    ) {
        let pointer_position = match ui.input().pointer.interact_pos() {
            Some(position) if response.rect.contains(position) => position,
            _ => return,
        };

        let pointer_in_world_before_zoom = painter.transform_pixel_to_world(pointer_position);
        let zoom_factor = 1.01_f32.powf(ui.input().scroll_delta.y);
        let zoom_transform = Similarity2::from_scaling(zoom_factor);
        painter.append_transform(zoom_transform);
        let pointer_in_pixel_after_zoom =
            painter.transform_world_to_pixel(pointer_in_world_before_zoom);
        let shift_from_zoom = pointer_position - pointer_in_pixel_after_zoom;
        let pixel_drag = vector![response.drag_delta().x, response.drag_delta().y];
        self.transformation.append_scaling_mut(zoom_factor);
        self.transformation
            .append_translation_mut(&Translation2::from(
                pixel_drag + vector![shift_from_zoom.x, shift_from_zoom.y],
            ));
    }
}

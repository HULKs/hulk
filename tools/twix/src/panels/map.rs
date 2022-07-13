use std::sync::Arc;

use eframe::{
    egui::{ComboBox, Ui, Widget},
    Storage,
};
use nalgebra::{vector, Similarity2, Translation2};
use serde_json::from_value;
use types::{self, FieldDimensions};

use crate::{nao::Nao, panel::Panel, twix_paint::TwixPainter, value_buffer::ValueBuffer};

use super::{layers, EnabledLayer};

pub struct MapPanel {
    field_dimensions: ValueBuffer,
    field: EnabledLayer<layers::Field>,
    robot_pose: EnabledLayer<layers::RobotPose>,
    ball_position: EnabledLayer<layers::BallPosition>,
    obstacles: EnabledLayer<layers::Obstacles>,
    path_obstacles: EnabledLayer<layers::PathObstacles>,
    path: EnabledLayer<layers::Path>,
    kick_decisions: EnabledLayer<layers::KickDecisions>,
    transformation: Similarity2<f32>,
}

impl Panel for MapPanel {
    const NAME: &'static str = "Map";

    fn new(nao: Arc<Nao>, storage: Option<&dyn Storage>) -> Self {
        let field = EnabledLayer::new(nao.clone(), storage, true);
        let robot_pose = EnabledLayer::new(nao.clone(), storage, true);
        let ball_position = EnabledLayer::new(nao.clone(), storage, false);
        let obstacles = EnabledLayer::new(nao.clone(), storage, false);
        let path_obstacles = EnabledLayer::new(nao.clone(), storage, false);
        let path = EnabledLayer::new(nao.clone(), storage, false);
        let kick_decisions = EnabledLayer::new(nao.clone(), storage, false);

        let field_dimensions = nao.subscribe_parameter("field_dimensions");
        let transformation = Similarity2::identity();
        Self {
            field_dimensions,
            field,
            robot_pose,
            ball_position,
            obstacles,
            path_obstacles,
            path,
            kick_decisions,
            transformation,
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        self.field.save(storage);
        self.robot_pose.save(storage);
        self.ball_position.save(storage);
        self.obstacles.save(storage);
        self.path_obstacles.save(storage);
        self.path.save(storage);
        self.kick_decisions.save(storage);
    }
}

impl Widget for &mut MapPanel {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        ComboBox::from_id_source("Layers")
            .selected_text("Layers")
            .show_ui(ui, |ui: &mut Ui| {
                self.field.checkbox(ui);
                self.robot_pose.checkbox(ui);
                self.ball_position.checkbox(ui);
                self.obstacles.checkbox(ui);
                self.path_obstacles.checkbox(ui);
                self.path.checkbox(ui);
                self.kick_decisions.checkbox(ui);
            });

        let field_dimensions: FieldDimensions = match self.field_dimensions.get_latest() {
            Ok(value) => from_value(value).unwrap(),
            Err(error) => return ui.label(format!("{:?}", error)),
        };

        let painter = TwixPainter::new_map(ui, &field_dimensions, self.transformation);

        let _ = self.field.paint(&painter, &field_dimensions);
        let _ = self.robot_pose.paint(&painter, &field_dimensions);
        let _ = self.ball_position.paint(&painter, &field_dimensions);
        let _ = self.obstacles.paint(&painter, &field_dimensions);
        let _ = self.path_obstacles.paint(&painter, &field_dimensions);
        let _ = self.path.paint(&painter, &field_dimensions);
        let _ = self.kick_decisions.paint(&painter, &field_dimensions);

        let drag = painter.response.drag_delta();
        let drag = vector![drag.x, drag.y].component_mul(&vector![
            field_dimensions.length / painter.response.rect.width(),
            field_dimensions.width / painter.response.rect.height()
        ]);
        let zoom_factor = 1.01_f32.powf(ui.input().scroll_delta.y);
        self.transformation.append_scaling_mut(zoom_factor);
        if painter.response.double_clicked() {
            self.transformation = Similarity2::identity();
        }

        self.transformation
            .append_translation_mut(&Translation2::from(drag));

        painter.response
    }
}

use std::sync::Arc;

use eframe::{
    egui::{ComboBox, Ui, Widget},
    Storage,
};
use serde_json::from_value;
use types::{self, FieldDimensions};

use crate::{nao::Nao, panel::Panel, twix_paint::TwixPainter, value_buffer::ValueBuffer};

use super::{layers, EnabledLayer};

pub struct MapPanel {
    field_dimensions: ValueBuffer,
    field: EnabledLayer<layers::Field>,
    robot_pose: EnabledLayer<layers::RobotPose>,
    ball_position: EnabledLayer<layers::BallPosition>,
    path_obstacles: EnabledLayer<layers::PathObstacles>,
    path: EnabledLayer<layers::Path>,
}

impl Panel for MapPanel {
    const NAME: &'static str = "Map";

    fn new(nao: Arc<Nao>, storage: Option<&dyn Storage>) -> Self {
        let field = EnabledLayer::new(nao.clone(), storage, true);
        let robot_pose = EnabledLayer::new(nao.clone(), storage, true);
        let ball_position = EnabledLayer::new(nao.clone(), storage, false);
        let path_obstacles = EnabledLayer::new(nao.clone(), storage, false);
        let path = EnabledLayer::new(nao.clone(), storage, false);

        let field_dimensions = nao.subscribe_parameter("field_dimensions");
        Self {
            field_dimensions,
            field,
            robot_pose,
            ball_position,
            path_obstacles,
            path,
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        self.field.save(storage);
        self.robot_pose.save(storage);
        self.ball_position.save(storage);
        self.path_obstacles.save(storage);
        self.path.save(storage);
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
                self.path_obstacles.checkbox(ui);
                self.path.checkbox(ui);
            });

        let field_dimensions: FieldDimensions = match self.field_dimensions.get_latest() {
            Ok(value) => from_value(value).unwrap(),
            Err(error) => return ui.label(format!("{:?}", error)),
        };

        let painter = TwixPainter::new_map(ui, &field_dimensions);

        self.field.paint(&painter, &field_dimensions);
        self.robot_pose.paint(&painter, &field_dimensions);
        self.ball_position.paint(&painter, &field_dimensions);
        self.path_obstacles.paint(&painter, &field_dimensions);
        self.path.paint(&painter, &field_dimensions);

        painter.response
    }
}

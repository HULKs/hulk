use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use coordinate_systems::{Field, Ground};
use eframe::egui::{ComboBox, Ui, Widget};
use linear_algebra::Isometry2;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use types::{self, field_dimensions::FieldDimensions};

use crate::{
    nao::Nao, panel::Panel, twix_painter::TwixPainter, value_buffer::ValueBuffer,
    zoom_and_pan::ZoomAndPanManager,
};

use self::layer::{EnabledLayer, Layer};

mod layer;
mod layers;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
enum PlotType {
    Field,
    Ground,
}

trait GenericLayer {
    fn generic_paint(
        &mut self,
        painter: &TwixPainter<Field>,
        ground_to_field: Isometry2<Ground, Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()>;
}

impl<T: Layer<Field>> GenericLayer for EnabledLayer<T, Field> {
    fn generic_paint(
        &mut self,
        painter: &TwixPainter<Field>,
        _ground_to_field: Isometry2<Ground, Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        self.paint(painter, field_dimensions)
    }
}

impl<T: Layer<Ground>> GenericLayer for EnabledLayer<T, Ground> {
    fn generic_paint(
        &mut self,
        painter: &TwixPainter<Field>,
        ground_to_field: Isometry2<Ground, Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        self.paint(
            &painter.transform_painter(ground_to_field.inverse()),
            field_dimensions,
        )
    }
}

pub struct MapPanel {
    current_plot_type: PlotType,

    field_dimensions: ValueBuffer,
    ground_to_field: ValueBuffer,
    zoom_and_pan: ZoomAndPanManager,

    field: EnabledLayer<layers::Field, Field>,
    image_segments: EnabledLayer<layers::ImageSegments, Ground>,
    lines: EnabledLayer<layers::Lines, Ground>,
    ball_search_heatmap: EnabledLayer<layers::BallSearchHeatmap, Field>,
    line_correspondences: EnabledLayer<layers::LineCorrespondences, Field>,
    path_obstacles: EnabledLayer<layers::PathObstacles, Ground>,
    obstacles: EnabledLayer<layers::Obstacles, Ground>,
    path: EnabledLayer<layers::Path, Ground>,
    behavior_simulator: EnabledLayer<layers::BehaviorSimulator, Field>,
    robot_pose: EnabledLayer<layers::RobotPose, Ground>,
    referee_position: EnabledLayer<layers::RefereePosition, Field>,
    pose_detection: EnabledLayer<layers::PoseDetection, Field>,
    ball_position: EnabledLayer<layers::BallPosition, Field>,
    kick_decisions: EnabledLayer<layers::KickDecisions, Ground>,
    feet_detection: EnabledLayer<layers::FeetDetection, Ground>,
    ball_filter: EnabledLayer<layers::BallFilter, Ground>,
    obstacle_filter: EnabledLayer<layers::ObstacleFilter, Ground>,
    walking: EnabledLayer<layers::Walking, Ground>,
}

impl Panel for MapPanel {
    const NAME: &'static str = "Map";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let field = EnabledLayer::new(nao.clone(), value, true);
        let image_segments = EnabledLayer::new(nao.clone(), value, false);
        let line_correspondences = EnabledLayer::new(nao.clone(), value, false);
        let lines = EnabledLayer::new(nao.clone(), value, true);
        let ball_search_heatmap = EnabledLayer::new(nao.clone(), value, false);
        let path_obstacles = EnabledLayer::new(nao.clone(), value, false);
        let obstacles = EnabledLayer::new(nao.clone(), value, false);
        let path = EnabledLayer::new(nao.clone(), value, false);
        let behavior_simulator = EnabledLayer::new(nao.clone(), value, false);
        let referee_position = EnabledLayer::new(nao.clone(), value, true);
        let robot_pose = EnabledLayer::new(nao.clone(), value, true);
        let pose_detection = EnabledLayer::new(nao.clone(), value, true);
        let ball_position = EnabledLayer::new(nao.clone(), value, true);
        let kick_decisions = EnabledLayer::new(nao.clone(), value, false);
        let feet_detection = EnabledLayer::new(nao.clone(), value, false);
        let ball_filter = EnabledLayer::new(nao.clone(), value, false);
        let obstacle_filter = EnabledLayer::new(nao.clone(), value, false);
        let walking = EnabledLayer::new(nao.clone(), value, false);

        let field_dimensions = nao.subscribe_parameter("field_dimensions");
        let ground_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ground_to_field").unwrap());
        let zoom_and_pan = ZoomAndPanManager::default();
        Self {
            current_plot_type: PlotType::Field,
            field_dimensions,
            ground_to_field,
            zoom_and_pan,
            field,
            image_segments,
            line_correspondences,
            lines,
            ball_search_heatmap,
            path_obstacles,
            obstacles,
            path,
            behavior_simulator,
            robot_pose,
            pose_detection,
            referee_position,
            ball_position,
            kick_decisions,
            feet_detection,
            ball_filter,
            obstacle_filter,
            walking,
        }
    }

    fn save(&self) -> Value {
        json!({
            "current_plot_type": self.current_plot_type,
            "field": self.field.save(),
            "image_segments": self.image_segments.save(),
            "line_correspondences": self.line_correspondences.save(),
            "lines": self.lines.save(),
            "ball_search_heatmap": self.obstacle_filter.save(),
            "path_obstacles": self.path_obstacles.save(),
            "obstacles": self.obstacles.save(),
            "path": self.path.save(),
            "behavior_simulator": self.behavior_simulator.save(),
            "pose_detection": self.referee_position.save(),
            "robot_pose": self.robot_pose.save(),
            "referee_position": self.referee_position.save(),
            "ball_position": self.ball_position.save(),
            "kick_decisions": self.kick_decisions.save(),
            "feet_detection": self.feet_detection.save(),
            "ball_filter": self.ball_filter.save(),
            "obstacle_filter": self.obstacle_filter.save(),
            "walking": self.walking.save(),
        })
    }
}

impl Widget for &mut MapPanel {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            ui.menu_button("Overlays", |ui| {
                self.field.checkbox(ui);
                self.image_segments.checkbox(ui);
                self.line_correspondences.checkbox(ui);
                self.lines.checkbox(ui);
                self.ball_search_heatmap.checkbox(ui);
                self.path_obstacles.checkbox(ui);
                self.obstacles.checkbox(ui);
                self.path.checkbox(ui);
                self.behavior_simulator.checkbox(ui);
                self.pose_detection.checkbox(ui);
                self.robot_pose.checkbox(ui);
                self.referee_position.checkbox(ui);
                self.ball_position.checkbox(ui);
                self.kick_decisions.checkbox(ui);
                self.feet_detection.checkbox(ui);
                self.ball_filter.checkbox(ui);
                self.obstacle_filter.checkbox(ui);
                self.walking.checkbox(ui);
            });
            ComboBox::from_id_source("plot_type_selector")
                .selected_text(format!("{:?}", self.current_plot_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.current_plot_type, PlotType::Ground, "Ground");
                    ui.selectable_value(&mut self.current_plot_type, PlotType::Field, "Field");
                });
        });

        let field_dimensions: FieldDimensions = match self.field_dimensions.get_latest() {
            Ok(value) => from_value(value).unwrap(),
            Err(error) => return ui.label(format!("{error:?}")),
        };

        let ground_to_field: Isometry2<Ground, Field> =
            self.ground_to_field.parse_latest().unwrap_or_default();
        let (response, mut painter) = match self.current_plot_type {
            PlotType::Field => {
                let (response, painter) = TwixPainter::allocate_new(ui);
                let mut painter = painter.with_map_transforms(&field_dimensions);
                painter.append_transform(self.zoom_and_pan.transformation());
                (response, painter)
            }
            PlotType::Ground => {
                let (response, painter) = TwixPainter::allocate_new(ui);
                let mut painter = painter.with_ground_transforms();
                painter.append_transform(self.zoom_and_pan.transformation());

                (response, painter.transform_painter(ground_to_field))
            }
        };

        // draw largest layers first so they don't obscure smaller ones
        let _ = self
            .field
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .image_segments
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ =
            self.line_correspondences
                .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .lines
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ =
            self.ball_search_heatmap
                .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .path_obstacles
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .obstacles
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .path
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .behavior_simulator
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .robot_pose
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .referee_position
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .pose_detection
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .ball_position
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .kick_decisions
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .feet_detection
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .ball_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .obstacle_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        let _ = self
            .walking
            .generic_paint(&painter, ground_to_field, &field_dimensions);

        self.zoom_and_pan.apply(ui, &mut painter, &response);
        response
    }
}

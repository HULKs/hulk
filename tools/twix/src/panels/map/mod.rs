use std::sync::Arc;

use coordinate_systems::{Field, Ground};
use eframe::egui::{ComboBox, Ui, Widget};
use linear_algebra::{point, vector, Isometry2};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao,
    panel::Panel,
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
    zoom_and_pan::ZoomAndPanTransform,
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
    );
}

impl<T: Layer<Field>> GenericLayer for EnabledLayer<T, Field> {
    fn generic_paint(
        &mut self,
        painter: &TwixPainter<Field>,
        _ground_to_field: Isometry2<Ground, Field>,
        field_dimensions: &FieldDimensions,
    ) {
        self.paint_or_disable(painter, field_dimensions)
    }
}

impl<T: Layer<Ground>> GenericLayer for EnabledLayer<T, Ground> {
    fn generic_paint(
        &mut self,
        painter: &TwixPainter<Field>,
        ground_to_field: Isometry2<Ground, Field>,
        field_dimensions: &FieldDimensions,
    ) {
        self.paint_or_disable(
            &painter.transform_painter(ground_to_field.inverse()),
            field_dimensions,
        )
    }
}

pub struct MapPanel {
    current_plot_type: PlotType,

    field_dimensions: BufferHandle<FieldDimensions>,
    ground_to_field: BufferHandle<Option<Isometry2<Ground, Field>>>,
    zoom_and_pan: ZoomAndPanTransform,

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
        let pose_detection = EnabledLayer::new(nao.clone(), value, false);
        let ball_position = EnabledLayer::new(nao.clone(), value, true);
        let kick_decisions = EnabledLayer::new(nao.clone(), value, false);
        let feet_detection = EnabledLayer::new(nao.clone(), value, false);
        let ball_filter = EnabledLayer::new(nao.clone(), value, false);
        let obstacle_filter = EnabledLayer::new(nao.clone(), value, false);
        let walking = EnabledLayer::new(nao.clone(), value, false);

        let field_dimensions = nao.subscribe_value("parameters.field_dimensions");
        let ground_to_field = nao.subscribe_value("Control.main_outputs.ground_to_field");
        let zoom_and_pan = ZoomAndPanTransform::default();

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

        let field_dimensions: FieldDimensions = match self.field_dimensions.get_last_value() {
            Ok(Some(value)) => value,
            Ok(None) => return ui.label("no response for field dimensions yet"),
            Err(error) => return ui.label(format!("{error:#}")),
        };

        let ground_to_field = self
            .ground_to_field
            .get_last_value()
            .ok()
            .flatten()
            .flatten()
            .unwrap_or_default();
        let (response, mut painter) = match self.current_plot_type {
            PlotType::Field => {
                let width = field_dimensions.width;
                let length = field_dimensions.length;
                let border = field_dimensions.border_strip_width;

                TwixPainter::allocate(
                    ui,
                    vector![2.0 * border + length, 2.0 * border + width],
                    point![
                        border + field_dimensions.length / 2.0,
                        -border - field_dimensions.width / 2.0
                    ],
                    Orientation::RightHanded,
                )
            }
            PlotType::Ground => {
                let (response, painter) = TwixPainter::allocate(
                    ui,
                    vector![2.0, 2.0],
                    point![-1.0, -1.0],
                    Orientation::RightHanded,
                );
                (response, painter.transform_painter(ground_to_field))
            }
        };
        self.zoom_and_pan.apply(ui, &mut painter, &response);

        // draw largest layers first so they don't obscure smaller ones
        self.field
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.image_segments
            .generic_paint(&painter, ground_to_field, &field_dimensions);

        self.line_correspondences
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.lines
            .generic_paint(&painter, ground_to_field, &field_dimensions);

        self.ball_search_heatmap
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.path_obstacles
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.obstacles
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.path
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.behavior_simulator
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.robot_pose
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.referee_position
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.pose_detection
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_position
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.kick_decisions
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.feet_detection
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.obstacle_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.walking
            .generic_paint(&painter, ground_to_field, &field_dimensions);

        response
    }
}

use coordinate_systems::{Field, Ground};
use eframe::egui::{ComboBox, Ui, Widget};
use linear_algebra::{Isometry2, point, vector};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use types::field_dimensions::FieldDimensions;

use crate::{
    panel::{Panel, PanelCreationContext},
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
    selected_localization_hypothesis: Option<usize>,

    field_dimensions: BufferHandle<FieldDimensions>,
    ground_to_field: BufferHandle<Option<Isometry2<Ground, Field>>>,
    zoom_and_pan: ZoomAndPanTransform,

    field: EnabledLayer<layers::Field, Field>,
    image_segments: EnabledLayer<layers::ImageSegments, Ground>,
    lines: EnabledLayer<layers::Lines, Ground>,
    ball_search_heatmap: EnabledLayer<layers::BallSearchHeatmap, Field>,
    path_obstacles: EnabledLayer<layers::PathObstacles, Ground>,
    obstacles: EnabledLayer<layers::Obstacles, Ground>,
    path: EnabledLayer<layers::Path, Ground>,
    behavior_simulator: EnabledLayer<layers::BehaviorSimulator, Field>,
    robot_pose: EnabledLayer<layers::RobotPose, Ground>,
    referee_position: EnabledLayer<layers::RefereePosition, Field>,
    pose_detection: EnabledLayer<layers::PoseDetection, Field>,
    ball_measurement: EnabledLayer<layers::BallMeasurement, Ground>,
    ball_position: EnabledLayer<layers::BallPosition, Field>,
    kick_decisions: EnabledLayer<layers::KickDecisions, Ground>,
    ball_filter: EnabledLayer<layers::BallFilter, Ground>,
    obstacle_filter: EnabledLayer<layers::ObstacleFilter, Ground>,
    localization: EnabledLayer<layers::Localization, Field>,
}

impl<'a> Panel<'a> for MapPanel {
    const NAME: &'static str = "Map";

    fn new(context: PanelCreationContext) -> Self {
        let field = EnabledLayer::new(context.robot.clone(), context.value, true);
        let image_segments = EnabledLayer::new(context.robot.clone(), context.value, false);
        let lines = EnabledLayer::new(context.robot.clone(), context.value, true);
        let ball_search_heatmap = EnabledLayer::new(context.robot.clone(), context.value, false);
        let path_obstacles = EnabledLayer::new(context.robot.clone(), context.value, false);
        let obstacles = EnabledLayer::new(context.robot.clone(), context.value, false);
        let path = EnabledLayer::new(context.robot.clone(), context.value, false);
        let behavior_simulator = EnabledLayer::new(context.robot.clone(), context.value, false);
        let referee_position = EnabledLayer::new(context.robot.clone(), context.value, false);
        let robot_pose = EnabledLayer::new(context.robot.clone(), context.value, true);
        let ball_measurement = EnabledLayer::new(context.robot.clone(), context.value, false);
        let pose_detection = EnabledLayer::new(context.robot.clone(), context.value, false);
        let ball_position = EnabledLayer::new(context.robot.clone(), context.value, true);
        let kick_decisions = EnabledLayer::new(context.robot.clone(), context.value, false);
        let ball_filter = EnabledLayer::new(context.robot.clone(), context.value, false);
        let obstacle_filter = EnabledLayer::new(context.robot.clone(), context.value, false);
        let localization = EnabledLayer::new(context.robot.clone(), context.value, false);

        let field_dimensions = context.robot.subscribe_value("parameters.field_dimensions");
        let ground_to_field = context
            .robot
            .subscribe_value("WorldState.main_outputs.ground_to_field");

        let current_plot_type = context
            .value
            .and_then(|value| value.get("current_plot_type"))
            .and_then(|value| serde_json::from_value::<PlotType>(value.clone()).ok())
            .unwrap_or(PlotType::Ground);
        let zoom_and_pan = context
            .value
            .and_then(|value| value.get("zoom_and_pan"))
            .and_then(|value| serde_json::from_value::<ZoomAndPanTransform>(value.clone()).ok())
            .unwrap_or_default();
        let selected_localization_hypothesis = context
            .value
            .and_then(|value| value.get("selected_localization_hypothesis"))
            .and_then(|value| serde_json::from_value::<Option<usize>>(value.clone()).ok())
            .flatten();

        Self {
            current_plot_type,
            selected_localization_hypothesis,
            field_dimensions,
            ground_to_field,
            zoom_and_pan,
            field,
            image_segments,
            lines,
            ball_search_heatmap,
            path_obstacles,
            obstacles,
            path,
            behavior_simulator,
            robot_pose,
            pose_detection,
            referee_position,
            ball_measurement,
            ball_position,
            kick_decisions,
            ball_filter,
            obstacle_filter,
            localization,
        }
    }

    fn save(&self) -> Value {
        json!({
            "current_plot_type": self.current_plot_type,
            "zoom_and_pan": serde_json::to_value(&self.zoom_and_pan).expect("failed to serialize zoom_and_pan"),
            "selected_localization_hypothesis": self.selected_localization_hypothesis,

            "field": self.field.save(),
            "image_segments": self.image_segments.save(),
            "lines": self.lines.save(),
            "ball_search_heatmap": self.obstacle_filter.save(),
            "path_obstacles": self.path_obstacles.save(),
            "obstacles": self.obstacles.save(),
            "path": self.path.save(),
            "behavior_simulator": self.behavior_simulator.save(),
            "pose_detection": self.referee_position.save(),
            "robot_pose": self.robot_pose.save(),
            "referee_position": self.referee_position.save(),
            "ball_measurements": self.ball_measurement.save(),
            "ball_position": self.ball_position.save(),
            "kick_decisions": self.kick_decisions.save(),
            "ball_filter": self.ball_filter.save(),
            "obstacle_filter": self.obstacle_filter.save(),
            "localization": self.localization.save(),
        })
    }
}

impl Widget for &mut MapPanel {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            ui.menu_button("Overlays", |ui| {
                self.field.checkbox(ui);
                self.image_segments.checkbox(ui);
                self.lines.checkbox(ui);
                self.ball_search_heatmap.checkbox(ui);
                self.path_obstacles.checkbox(ui);
                self.obstacles.checkbox(ui);
                self.path.checkbox(ui);
                self.behavior_simulator.checkbox(ui);
                self.pose_detection.checkbox(ui);
                self.robot_pose.checkbox(ui);
                self.referee_position.checkbox(ui);
                self.ball_measurement.checkbox(ui);
                self.ball_position.checkbox(ui);
                self.kick_decisions.checkbox(ui);
                self.ball_filter.checkbox(ui);
                self.obstacle_filter.checkbox(ui);
                self.localization.checkbox(ui);
            });
            ComboBox::from_id_salt("plot_type_selector")
                .selected_text(format!("{:?}", self.current_plot_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.current_plot_type, PlotType::Ground, "Ground");
                    ui.selectable_value(&mut self.current_plot_type, PlotType::Field, "Field");
                });
        });

        let field_dimensions: FieldDimensions = match self.field_dimensions.get_last_value() {
            Ok(Some(value)) => value,
            Ok(None) => return ui.label("no response for field dimensions"),
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
                    point![1.0, -1.0],
                    Orientation::RightHanded,
                );
                (response, painter.transform_painter(ground_to_field))
            }
        };
        self.zoom_and_pan.apply(ui, &mut painter, &response);

        if response.clicked() {
            if let Some(pointer_position) = response.interact_pointer_pos() {
                if let Some(localization_layer) = self.localization.layer() {
                    match localization_layer.pick_hypothesis_at(
                        painter.transform_pixel_to_world(pointer_position),
                        0.35,
                    ) {
                        Ok(selection) => {
                            self.selected_localization_hypothesis = selection;
                        }
                        Err(error) => return ui.label(format!("{error:#}")),
                    }
                }
            }
        }
        if let Some(localization_layer) = self.localization.layer_mut() {
            localization_layer.set_selected_hypothesis_index(self.selected_localization_hypothesis);
        }

        // draw largest layers first so they don't obscure smaller ones
        self.field
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.image_segments
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
        self.ball_measurement
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_position
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.pose_detection
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_position
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.kick_decisions
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.obstacle_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.localization
            .generic_paint(&painter, ground_to_field, &field_dimensions);

        response
    }
}

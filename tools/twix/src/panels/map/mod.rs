use coordinate_systems::{Field, Ground};
use eframe::egui::{ComboBox, Ui};
use linear_algebra::{Isometry2, point, vector};
use ros_z_debug::TopicObservation;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use types::field_dimensions::FieldDimensions;

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    twix_painter::{Orientation, TwixPainter},
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

    field_dimensions: TopicObservation<FieldDimensions>,
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
    zoom_and_pan: ZoomAndPanTransform,

    field: EnabledLayer<layers::Field, Field>,
    lines: EnabledLayer<layers::Lines, Ground>,
    ball_search_heatmap: EnabledLayer<layers::BallSearchHeatmap, Field>,
    line_correspondences: EnabledLayer<layers::LineCorrespondences, Field>,
    path_obstacles: EnabledLayer<layers::PathObstacles, Ground>,
    obstacles: EnabledLayer<layers::Obstacles, Ground>,
    path: EnabledLayer<layers::Path, Ground>,
    robot_pose: EnabledLayer<layers::RobotPose, Ground>,
    ball_percept: EnabledLayer<layers::BallPercepts, Ground>,
    ball_position: EnabledLayer<layers::BallPosition, Field>,
    ball_filter: EnabledLayer<layers::BallFilter, Ground>,
    obstacle_filter: EnabledLayer<layers::ObstacleFilter, Ground>,
    localization: EnabledLayer<layers::Localization, Field>,
    voronoi_cells: EnabledLayer<layers::VoronoiCell, Field>,
}

impl Panel for MapPanel {
    const STORAGE_ID: &'static str = "map";
    const DISPLAY_NAME: &'static str = "Map";

    fn new(context: PanelCreationContext) -> Self {
        let field = EnabledLayer::new(context.backend.clone(), context.value, true);
        let line_correspondences = EnabledLayer::new(context.backend.clone(), context.value, false);
        let lines = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_search_heatmap = EnabledLayer::new(context.backend.clone(), context.value, false);
        let path_obstacles = EnabledLayer::new(context.backend.clone(), context.value, false);
        let obstacles = EnabledLayer::new(context.backend.clone(), context.value, false);
        let path = EnabledLayer::new(context.backend.clone(), context.value, false);
        let robot_pose = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_percept = EnabledLayer::new(context.backend.clone(), context.value, false);
        let ball_position = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_filter = EnabledLayer::new(context.backend.clone(), context.value, false);
        let obstacle_filter = EnabledLayer::new(context.backend.clone(), context.value, false);
        let localization = EnabledLayer::new(context.backend.clone(), context.value, false);
        let voronoi_cells = EnabledLayer::new(context.backend.clone(), context.value, false);

        let runtime_handle = context.backend.runtime_handle().clone();
        let _runtime_context = runtime_handle.enter();
        let field_dimensions = context
            .backend
            .observer()
            .observe_typed("field_dimensions")
            .expect("failed to construct field_dimensions observer")
            .spawn();
        let ground_to_field = context
            .backend
            .observer()
            .observe_typed("ground_to_field")
            .expect("failed to construct ground_to_field observer")
            .spawn();

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

        Self {
            current_plot_type,
            field_dimensions,
            ground_to_field,
            zoom_and_pan,
            field,
            line_correspondences,
            lines,
            ball_search_heatmap,
            path_obstacles,
            obstacles,
            path,
            robot_pose,
            ball_percept,
            ball_position,
            ball_filter,
            obstacle_filter,
            localization,
            voronoi_cells,
        }
    }

    fn save(&self) -> Value {
        json!({
            "current_plot_type": self.current_plot_type,
            "zoom_and_pan": serde_json::to_value(&self.zoom_and_pan).expect("failed to serialize zoom_and_pan"),

            "field": self.field.save(),
            "line_correspondences": self.line_correspondences.save(),
            "lines": self.lines.save(),
            "ball_search_heatmap": self.ball_search_heatmap.save(),
            "path_obstacles": self.path_obstacles.save(),
            "obstacles": self.obstacles.save(),
            "path": self.path.save(),
            "robot_pose": self.robot_pose.save(),
            "ball_percept": self.ball_percept.save(),
            "ball_position": self.ball_position.save(),
            "ball_filter": self.ball_filter.save(),
            "obstacle_filter": self.obstacle_filter.save(),
            "localization": self.localization.save(),
            "voronoi_cells": self.voronoi_cells.save(),
        })
    }

    fn ui(&mut self, ui: &mut Ui, _context: PanelUiContext<'_>) {
        ui.horizontal(|ui| {
            ui.menu_button("Overlays", |ui| {
                self.field.checkbox(ui);
                self.line_correspondences.checkbox(ui);
                self.lines.checkbox(ui);
                self.ball_search_heatmap.checkbox(ui);
                self.path_obstacles.checkbox(ui);
                self.obstacles.checkbox(ui);
                self.path.checkbox(ui);
                self.robot_pose.checkbox(ui);
                self.ball_percept.checkbox(ui);
                self.ball_position.checkbox(ui);
                self.ball_filter.checkbox(ui);
                self.obstacle_filter.checkbox(ui);
                self.localization.checkbox(ui);
                self.voronoi_cells.checkbox(ui);
            });
            ComboBox::from_id_salt("plot_type_selector")
                .selected_text(format!("{:?}", self.current_plot_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.current_plot_type, PlotType::Ground, "Ground");
                    ui.selectable_value(&mut self.current_plot_type, PlotType::Field, "Field");
                });
        });

        let field_dimensions: FieldDimensions = match self.field_dimensions.latest().as_deref() {
            Some(sample_record) => sample_record.value,
            None => {
                ui.label("no response for field dimensions");
                return;
            }
        };

        let ground_to_field = self
            .ground_to_field
            .latest()
            .map(|sample_record| sample_record.value)
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

        // draw largest layers first so they don't obscure smaller ones
        self.field
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
        self.robot_pose
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_percept
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_position
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.ball_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.obstacle_filter
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.localization
            .generic_paint(&painter, ground_to_field, &field_dimensions);
        self.voronoi_cells
            .generic_paint(&painter, ground_to_field, &field_dimensions);
    }
}

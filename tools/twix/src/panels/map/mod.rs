use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use eframe::egui::{ComboBox, Ui, Widget};
use linear_algebra::{Isometry2, point, vector};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value, json};
use types::field_dimensions::FieldDimensions;

use crate::{
    panel::{Panel, PanelCreationContext},
    twix_painter::{Orientation, TwixPainter},
    value_buffer::{BufferHandle, BufferHistory},
    zoom_and_pan::ZoomAndPanTransform,
};

use self::layer::{EnabledLayer, Layer};

pub mod layer;
mod layers;

const SKIPPED_LAYER_KEYS: &[&str] = &[
    "behavior_simulator",
    "pose_detection",
    "referee_position",
    "path_obstacles",
    "voronoi_cells",
    "ball_position",
    "ball_percept",
];
const GROUND_TO_FIELD_QUEUE_DEPTH: usize = crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH;
pub const BALL_STATE_QUEUE_DEPTH: usize = crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH;

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
    skipped_layers: JsonMap<String, Value>,

    field_dimensions: BufferHandle<FieldDimensions>,
    ground_to_field: BufferHandle<Option<Isometry2<Ground, Field>>>,
    zoom_and_pan: ZoomAndPanTransform,

    field: EnabledLayer<layers::Field, Field>,
    image_segments: EnabledLayer<layers::ImageSegments, Ground>,
    lines: EnabledLayer<layers::Lines, Ground>,
    ball_search_heatmap: EnabledLayer<layers::BallSearchHeatmap, Field>,
    line_correspondences: EnabledLayer<layers::LineCorrespondences, Field>,
    obstacles: EnabledLayer<layers::Obstacles, Ground>,
    path: EnabledLayer<layers::Path, Ground>,
    robot_pose: EnabledLayer<layers::RobotPose, Ground>,
    ball_percepts: EnabledLayer<layers::BallPercepts, Ground>,
    ball_state: EnabledLayer<layers::BallState, Field>,
    ball_filter: EnabledLayer<layers::BallFilter, Ground>,
    obstacle_filter: EnabledLayer<layers::ObstacleFilter, Ground>,
    localization: EnabledLayer<layers::Localization, Field>,
}

fn latest_ground_to_field(
    ground_to_field: &BufferHandle<Option<Isometry2<Ground, Field>>>,
) -> Result<Option<Isometry2<Ground, Field>>> {
    Ok(ground_to_field.get_last_value()?.flatten())
}

fn latest_ground_to_field_or_none(
    ground_to_field: &BufferHandle<Option<Isometry2<Ground, Field>>>,
) -> Option<Isometry2<Ground, Field>> {
    latest_ground_to_field(ground_to_field).ok().flatten()
}

fn latest_ground_to_field_or_identity(
    ground_to_field: &BufferHandle<Option<Isometry2<Ground, Field>>>,
) -> Isometry2<Ground, Field> {
    latest_ground_to_field_or_none(ground_to_field).unwrap_or_default()
}

impl<'a> Panel<'a> for MapPanel {
    const NAME: &'static str = "Map";

    fn new(context: PanelCreationContext) -> Self {
        let field = EnabledLayer::new(context.backend.clone(), context.value, true);
        let image_segments = EnabledLayer::new(context.backend.clone(), context.value, false);
        let line_correspondences = EnabledLayer::new(context.backend.clone(), context.value, false);
        let lines = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_search_heatmap = EnabledLayer::new(context.backend.clone(), context.value, false);
        let obstacles = EnabledLayer::new(context.backend.clone(), context.value, false);
        let path = EnabledLayer::new(context.backend.clone(), context.value, false);
        let robot_pose = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_percepts = EnabledLayer::new(context.backend.clone(), context.value, false);
        let ball_state = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_filter = EnabledLayer::new(context.backend.clone(), context.value, false);
        let obstacle_filter = EnabledLayer::new(context.backend.clone(), context.value, false);
        let localization = EnabledLayer::new(context.backend.clone(), context.value, false);

        let field_dimensions = context
            .backend
            .subscribe_transient_local_value("field_dimensions");
        let ground_to_field = context.backend.subscribe_buffered_value_with_queue_depth(
            "ground_to_field",
            BufferHistory::LatestOnly,
            GROUND_TO_FIELD_QUEUE_DEPTH,
        );

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
        let skipped_layers = preserved_skipped_layer_state(context.value);

        Self {
            current_plot_type,
            skipped_layers,
            field_dimensions,
            ground_to_field,
            zoom_and_pan,
            field,
            image_segments,
            line_correspondences,
            lines,
            ball_search_heatmap,
            obstacles,
            path,
            robot_pose,
            ball_percepts,
            ball_state,
            ball_filter,
            obstacle_filter,
            localization,
        }
    }

    fn save(&self) -> Value {
        let mut value = json!({
            "current_plot_type": self.current_plot_type,
            "zoom_and_pan": serde_json::to_value(&self.zoom_and_pan).expect("failed to serialize zoom_and_pan"),

            "field": self.field.save(),
            "image_segments": self.image_segments.save(),
            "line_correspondences": self.line_correspondences.save(),
            "lines": self.lines.save(),
            "ball_search_heatmap": self.ball_search_heatmap.save(),
            "obstacles": self.obstacles.save(),
            "path": self.path.save(),
            "robot_pose": self.robot_pose.save(),
            "ball_percepts": self.ball_percepts.save(),
            "ball_state": self.ball_state.save(),
            "ball_filter": self.ball_filter.save(),
            "obstacle_filter": self.obstacle_filter.save(),
            "localization": self.localization.save(),
        });

        let Value::Object(object) = &mut value else {
            return value;
        };
        object.extend(self.skipped_layers.clone());
        value
    }
}

fn preserved_skipped_layer_state(value: Option<&Value>) -> JsonMap<String, Value> {
    let Some(Value::Object(object)) = value else {
        return JsonMap::new();
    };

    SKIPPED_LAYER_KEYS
        .iter()
        .filter_map(|key| {
            object
                .get(*key)
                .map(|value| ((*key).to_owned(), value.clone()))
        })
        .collect()
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
                self.obstacles.checkbox(ui);
                self.path.checkbox(ui);
                self.robot_pose.checkbox(ui);
                self.ball_percepts.checkbox(ui);
                self.ball_state.checkbox(ui);
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

        let ground_to_field = latest_ground_to_field_or_identity(&self.ground_to_field);
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
        self.paint_layers(&painter, ground_to_field, &field_dimensions);

        response
    }
}

impl MapPanel {
    fn paint_layers(
        &mut self,
        painter: &TwixPainter<Field>,
        ground_to_field: Isometry2<Ground, Field>,
        field_dimensions: &FieldDimensions,
    ) {
        self.field
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.image_segments
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.line_correspondences
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.lines
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.ball_search_heatmap
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.obstacles
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.path
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.robot_pose
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.ball_percepts
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.ball_state
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.ball_filter
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.obstacle_filter
            .generic_paint(painter, ground_to_field, field_dimensions);
        self.localization
            .generic_paint(painter, ground_to_field, field_dimensions);
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use color_eyre::eyre;
    use serde_json::json;

    use super::*;
    use crate::value_buffer::{Buffer, Datum};

    #[test]
    fn ground_to_field_uses_deployed_optional_message_type() {
        fn assert_ground_to_field(_: &BufferHandle<Option<Isometry2<Ground, Field>>>) {}

        fn assert_map_panel(panel: &MapPanel) {
            assert_ground_to_field(&panel.ground_to_field);
        }

        let _ = assert_map_panel;
    }

    #[tokio::test]
    async fn latest_ground_to_field_or_none_treats_errors_as_missing_transform() {
        let (buffer, handle) = Buffer::<Option<Isometry2<Ground, Field>>, eyre::Report>::new(
            BufferHistory::LatestOnly,
        );

        buffer.send_error(color_eyre::eyre::eyre!("decode failed"));

        assert!(latest_ground_to_field_or_none(&handle).is_none());
    }

    #[tokio::test]
    async fn ground_plot_uses_identity_when_ground_to_field_is_missing() {
        let (_buffer, handle) = Buffer::<Option<Isometry2<Ground, Field>>, eyre::Report>::new(
            BufferHistory::LatestOnly,
        );

        assert_eq!(
            latest_ground_to_field_or_identity(&handle) * point![1.0, 2.0],
            point![1.0, 2.0]
        );
    }

    #[tokio::test]
    async fn ground_plot_uses_identity_when_ground_to_field_is_none() {
        let (buffer, handle) = Buffer::<Option<Isometry2<Ground, Field>>, eyre::Report>::new(
            BufferHistory::LatestOnly,
        );
        buffer
            .push(Datum {
                timestamp: SystemTime::UNIX_EPOCH,
                value: None,
            })
            .await;

        assert_eq!(
            latest_ground_to_field_or_identity(&handle) * point![1.0, 2.0],
            point![1.0, 2.0]
        );
    }

    #[tokio::test]
    async fn ground_plot_uses_latest_ground_to_field_when_present() {
        let (buffer, handle) = Buffer::<Option<Isometry2<Ground, Field>>, eyre::Report>::new(
            BufferHistory::LatestOnly,
        );
        let transform = Isometry2::from_parts(vector![1.0, 0.0], 0.0);
        buffer
            .push(Datum {
                timestamp: SystemTime::UNIX_EPOCH,
                value: Some(transform),
            })
            .await;

        assert_eq!(
            latest_ground_to_field_or_identity(&handle) * point![1.0, 2.0],
            transform * point![1.0, 2.0]
        );
    }

    #[test]
    fn save_preserves_deferred_overlay_state_from_saved_map() {
        let saved = json!({
            "field": { "active": false },
            "behavior_simulator": { "active": true },
            "pose_detection": { "accepted": true },
            "referee_position": { "active": true },
            "path_obstacles": { "active": true },
            "voronoi_cells": { "active": true },
            "ball_position": { "active": true },
            "ball_percept": { "active": true },
        });

        let preserved = preserved_skipped_layer_state(Some(&saved));

        for key in SKIPPED_LAYER_KEYS {
            assert_eq!(preserved.get(*key), saved.get(*key));
        }
        assert!(!preserved.contains_key("field"));
    }
}

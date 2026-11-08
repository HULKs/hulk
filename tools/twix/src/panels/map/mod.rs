use coordinate_systems::{Field, Ground};
use eframe::egui::{ComboBox, Popup, PopupCloseBehavior, Ui};
use linear_algebra::{Isometry2, point, vector};
use ros_z::qos::{QosDurability, QosProfile};
use ros_z_debug::{ObservationPolicy, TopicObservation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use types::field_dimensions::FieldDimensions;

use crate::panel::{Panel, PanelCreationContext, PanelUiContext};
use twix_visualization::{
    twix_painter::{Orientation, TwixPainter},
    zoom_and_pan::ZoomAndPanTransform,
};

use self::{
    ground_frames::{GroundAlignment, GroundFrames},
    layer::{EnabledLayer, Layer},
};

mod ground_frames;

mod layer;
mod layers;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
enum PlotType {
    Field,
    Ground,
}

/// The picture all layers are painted into.
struct Picture<'a> {
    painter: TwixPainter<Field>,
    plot_type: PlotType,
    alignment: GroundAlignment<'a>,
    /// The ground frame shown in the ground view, which is the current one.
    displayed_ground_to_field: Isometry2<Ground, Field>,
}

trait GenericLayer {
    fn generic_paint(&mut self, picture: &Picture, field_dimensions: &FieldDimensions);
}

impl<T: Layer<Field>> GenericLayer for EnabledLayer<T, Field> {
    fn generic_paint(&mut self, picture: &Picture, field_dimensions: &FieldDimensions) {
        self.paint_or_disable(&picture.painter, field_dimensions)
    }
}

impl<T: Layer<Ground>> GenericLayer for EnabledLayer<T, Ground> {
    fn generic_paint(&mut self, picture: &Picture, field_dimensions: &FieldDimensions) {
        let Picture {
            painter,
            plot_type,
            alignment,
            displayed_ground_to_field,
        } = picture;
        let time = self.ground_time();
        // Data of a frame that cannot be placed is left out instead of drawn in another frame.
        let ground_to_picture = match plot_type {
            PlotType::Field => time.map_or_else(
                || alignment.current_ground_to_field(),
                |time| alignment.ground_to_field_at(time),
            ),
            // The ground view stays with the robot, which needs only odometry.
            PlotType::Ground => time.map_or(Some(*displayed_ground_to_field), |time| {
                alignment
                    .ground_to_current_ground(time)
                    .map(|ground_to_current| *displayed_ground_to_field * ground_to_current)
            }),
        };
        if let Some(ground_to_picture) = ground_to_picture {
            self.paint_or_disable(
                &painter.transform_painter(ground_to_picture.inverse()),
                field_dimensions,
            );
        }
    }
}

pub struct MapPanel {
    current_plot_type: PlotType,

    field_dimensions: TopicObservation<FieldDimensions>,
    ground_frames: GroundFrames,
    zoom_and_pan: ZoomAndPanTransform,

    field: EnabledLayer<layers::Field, Field>,
    ball_search_heatmap: EnabledLayer<layers::BallSearchHeatmap, Field>,
    path_obstacles: EnabledLayer<layers::PathObstacles, Ground>,
    obstacles: EnabledLayer<layers::Obstacles, Ground>,
    path: EnabledLayer<layers::Path, Ground>,
    robot_pose: EnabledLayer<layers::RobotPose, Ground>,
    odometry: EnabledLayer<layers::Odometry, Field>,
    ball_percepts: EnabledLayer<layers::BallPercepts, Ground>,
    ball_position: EnabledLayer<layers::BallPosition, Field>,
    ball_filter: EnabledLayer<layers::BallFilter, Ground>,
    obstacle_filter: EnabledLayer<layers::ObstacleFilter, Ground>,
    voronoi_cells: EnabledLayer<layers::VoronoiCell, Field>,
}

impl Panel for MapPanel {
    const STORAGE_ID: &'static str = "map";
    const DISPLAY_NAME: &'static str = "Map";
    const ICON: &'static str = egui_material_icons::icons::ICON_MAP.codepoint;

    fn new(context: PanelCreationContext) -> Self {
        let field = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_search_heatmap = EnabledLayer::new(context.backend.clone(), context.value, false);
        let path_obstacles = EnabledLayer::new(context.backend.clone(), context.value, false);
        let obstacles = EnabledLayer::new(context.backend.clone(), context.value, false);
        let path = EnabledLayer::new(context.backend.clone(), context.value, false);
        let robot_pose = EnabledLayer::new(context.backend.clone(), context.value, true);
        let odometry = EnabledLayer::new(context.backend.clone(), context.value, false);
        let ball_percepts = EnabledLayer::new(context.backend.clone(), context.value, false);
        let ball_position = EnabledLayer::new(context.backend.clone(), context.value, true);
        let ball_filter = EnabledLayer::new(context.backend.clone(), context.value, false);
        let obstacle_filter = EnabledLayer::new(context.backend.clone(), context.value, false);
        let voronoi_cells = EnabledLayer::new(context.backend.clone(), context.value, false);

        let runtime_handle = context.backend.runtime_handle().clone();
        let _runtime_context = runtime_handle.enter();
        let field_dimensions = context
            .backend
            .observer()
            .observe_typed("field_dimensions")
            .expect("failed to construct field_dimensions observer")
            .policy(
                ObservationPolicy::default().with_subscriber_qos(QosProfile {
                    durability: QosDurability::TransientLocal,
                    ..Default::default()
                }),
            )
            .spawn();
        let ground_frames = GroundFrames::new(&context.backend);

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
            ground_frames,
            zoom_and_pan,
            field,
            ball_search_heatmap,
            path_obstacles,
            obstacles,
            path,
            robot_pose,
            odometry,
            ball_percepts,
            ball_position,
            ball_filter,
            obstacle_filter,
            voronoi_cells,
        }
    }

    fn save(&self) -> Value {
        json!({
            "current_plot_type": self.current_plot_type,
            "zoom_and_pan": serde_json::to_value(&self.zoom_and_pan).expect("failed to serialize zoom_and_pan"),

            "field": self.field.save(),
            "ball_search_heatmap": self.ball_search_heatmap.save(),
            "path_obstacles": self.path_obstacles.save(),
            "obstacles": self.obstacles.save(),
            "path": self.path.save(),
            "robot_pose": self.robot_pose.save(),
            "odometry": self.odometry.save(),
            "ball_percepts": self.ball_percepts.save(),
            "ball_position": self.ball_position.save(),
            "ball_filter": self.ball_filter.save(),
            "obstacle_filter": self.obstacle_filter.save(),
            "voronoi_cells": self.voronoi_cells.save(),
        })
    }

    fn header_ui(&mut self, ui: &mut Ui, _context: PanelUiContext<'_>) {
        Popup::menu(&ui.button("Overlays"))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                self.field.checkbox(ui);
                self.ball_search_heatmap.checkbox(ui);
                self.path_obstacles.checkbox(ui);
                self.obstacles.checkbox(ui);
                self.path.checkbox(ui);
                self.robot_pose.checkbox(ui);
                self.odometry.checkbox(ui);
                self.ball_percepts.checkbox(ui);
                self.ball_position.checkbox(ui);
                self.ball_filter.checkbox(ui);
                self.obstacle_filter.checkbox(ui);
                self.voronoi_cells.checkbox(ui);
            });
        ComboBox::from_id_salt("plot_type_selector")
            .selected_text(format!("{:?}", self.current_plot_type))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.current_plot_type, PlotType::Ground, "Ground");
                ui.selectable_value(&mut self.current_plot_type, PlotType::Field, "Field");
            });
    }

    fn ui(&mut self, ui: &mut Ui, _context: PanelUiContext<'_>) {
        let field_dimensions: FieldDimensions = match self.field_dimensions.latest().as_deref() {
            Some(sample_record) => sample_record.value,
            None => {
                ui.label("no response for field dimensions");
                return;
            }
        };

        let alignment = self.ground_frames.alignment();
        let displayed_ground_to_field = alignment.current_ground_to_field().unwrap_or_default();
        let plot_type = self.current_plot_type;
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
                (
                    response,
                    painter.transform_painter(displayed_ground_to_field),
                )
            }
        };
        self.zoom_and_pan.apply(ui, &mut painter, &response);

        let picture = Picture {
            painter,
            plot_type,
            alignment,
            displayed_ground_to_field,
        };

        // draw largest layers first so they don't obscure smaller ones
        self.field.generic_paint(&picture, &field_dimensions);

        self.ball_search_heatmap
            .generic_paint(&picture, &field_dimensions);
        self.path_obstacles
            .generic_paint(&picture, &field_dimensions);
        self.obstacles.generic_paint(&picture, &field_dimensions);
        self.path.generic_paint(&picture, &field_dimensions);
        self.robot_pose.generic_paint(&picture, &field_dimensions);
        self.odometry.generic_paint(&picture, &field_dimensions);
        self.ball_percepts
            .generic_paint(&picture, &field_dimensions);
        self.ball_position
            .generic_paint(&picture, &field_dimensions);
        self.ball_filter.generic_paint(&picture, &field_dimensions);
        self.obstacle_filter
            .generic_paint(&picture, &field_dimensions);
        self.voronoi_cells
            .generic_paint(&picture, &field_dimensions);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use eframe::{
        egui::{Context, RawInput, Rect, pos2, vec2},
        epaint::{Color32, Pos2, Shape, Vec2},
    };
    use geometry::circle::Circle;
    use linear_algebra::Pose2;
    use nalgebra::{SMatrix, SVector};
    use ros_z::pubsub::Publisher;
    use types::{
        ball_percept::BallPercept, multivariate_normal_distribution::MultivariateNormalDistribution,
    };

    use super::{
        ground_frames::tests::{Fixture, eventually, time},
        *,
    };

    const PIXELS_PER_METER: f32 = 400.0;

    fn panel(fixture: &Fixture, plot_type: PlotType, active_layer: &str) -> (MapPanel, Context) {
        let egui_context = Context::default();
        let settings = json!({
            "current_plot_type": plot_type,
            "field": {"active": false},
            "robot_pose": {"active": false},
            "ball_position": {"active": false},
            active_layer: {"active": true},
        });
        let panel = MapPanel::new(PanelCreationContext {
            backend: fixture.backend.clone(),
            value: Some(&settings),
            egui_context: egui_context.clone(),
        });
        (panel, egui_context)
    }

    fn render(
        panel: &mut MapPanel,
        backend: &Arc<crate::backend::RobotBackend>,
        egui_context: &Context,
        color: Color32,
    ) -> Option<Pos2> {
        let output = egui_context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 800.0))),
                ..Default::default()
            },
            |ui| {
                panel.ui(
                    ui,
                    PanelUiContext {
                        backend,
                        egui_context,
                    },
                )
            },
        );
        output.shapes.iter().find_map(|shape| match &shape.shape {
            Shape::Circle(circle) if circle.fill == color => Some(circle.center),
            _ => None,
        })
    }

    /// Publishes the field dimensions, without which the panel paints nothing, while it lives.
    async fn field_dimensions(fixture: &Fixture) -> Publisher<FieldDimensions> {
        fixture
            .backend
            .node()
            .publisher(&format!("{}/field_dimensions", fixture.namespace))
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            })
            .build()
            .await
            .unwrap()
    }

    async fn publish_until_rendered<T: ros_z::Message>(
        fixture: &Fixture,
        panel: &mut MapPanel,
        egui_context: &Context,
        dimensions: &Publisher<FieldDimensions>,
        publisher: &Publisher<T>,
        message: &T,
        color: Color32,
    ) -> Pos2 {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                dimensions
                    .publish_with_source_time(&FieldDimensions::SPL_2025, time(0))
                    .await
                    .unwrap();
                fixture.publish_pose(0, Pose2::zero()).await;
                fixture.publish_localization(0, Isometry2::identity()).await;
                publisher
                    .publish_with_source_time(message, time(0))
                    .await
                    .unwrap();
                // The ground view paints without odometry as well, but the test turns the robot.
                if panel
                    .ground_frames
                    .alignment()
                    .ground_to_field_at(time(0))
                    .is_some()
                    && let Some(center) = render(panel, &fixture.backend, egui_context, color)
                {
                    break center;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the layer did not paint")
    }

    /// Turns the robot without any new data of the layer; returns how far the layer moved.
    async fn motion_after_turn(
        fixture: &Fixture,
        panel: &mut MapPanel,
        egui_context: &Context,
        before: Pos2,
        color: Color32,
    ) -> Vec2 {
        let turned = Pose2::new(point![0.0, 0.0], 0.02);
        eventually(
            || async {
                fixture.publish_pose(20, turned).await;
                fixture
                    .publish_localization(
                        20,
                        Isometry2::wrap(turned.as_transform::<Ground>().inner),
                    )
                    .await;
            },
            || {
                panel
                    .ground_frames
                    .alignment()
                    .ground_to_field_at(time(20))
                    .is_some()
            },
        )
        .await;
        render(panel, &fixture.backend, egui_context, color).expect("still painted") - before
    }

    fn turned_ground_view_motion(distance: f32) -> Vec2 {
        // A turn to the left moves everything ahead to the right, which is down in this view.
        vec2(
            distance * (0.02_f32.cos() - 1.0) * PIXELS_PER_METER,
            distance * 0.02_f32.sin() * PIXELS_PER_METER,
        )
    }

    fn ball_percept() -> Vec<BallPercept> {
        vec![BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: SVector::from([0.6, 0.0]),
                covariance: SMatrix::identity() * 1e-5,
            },
            image_location: Circle {
                center: point![0.0, 0.0],
                radius: 1.0,
            },
        }]
    }

    /// Layers without own transforms take the frame of their stamp instead of the latest one.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ground_data_stays_in_the_field_view_while_the_robot_turns() {
        let fixture = Fixture::new().await;
        let percepts = fixture.publisher("ball_filter/ball_percepts").await;
        let dimensions = field_dimensions(&fixture).await;
        let (mut panel, egui_context) = panel(&fixture, PlotType::Field, "ball_percepts");
        let before = publish_until_rendered(
            &fixture,
            &mut panel,
            &egui_context,
            &dimensions,
            &percepts,
            &ball_percept(),
            Color32::WHITE,
        )
        .await;

        let motion =
            motion_after_turn(&fixture, &mut panel, &egui_context, before, Color32::WHITE).await;

        assert!(motion.length() < 0.1, "moved by {motion:?} px");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ground_data_follows_the_ground_view_while_the_robot_turns() {
        let fixture = Fixture::new().await;
        let percepts = fixture.publisher("ball_filter/ball_percepts").await;
        let dimensions = field_dimensions(&fixture).await;
        let (mut panel, egui_context) = panel(&fixture, PlotType::Ground, "ball_percepts");
        let before = publish_until_rendered(
            &fixture,
            &mut panel,
            &egui_context,
            &dimensions,
            &percepts,
            &ball_percept(),
            Color32::WHITE,
        )
        .await;

        let motion =
            motion_after_turn(&fixture, &mut panel, &egui_context, before, Color32::WHITE).await;

        let expected = turned_ground_view_motion(0.6);
        assert!(
            (motion - expected).length() < 0.1,
            "moved by {motion:?} px instead of {expected:?} px"
        );
    }

    /// Without localization the field view cannot place ground data, but it does not guess.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ground_data_is_left_out_of_the_field_view_without_localization() {
        let fixture = Fixture::new().await;
        let percepts = fixture.publisher("ball_filter/ball_percepts").await;
        let dimensions = field_dimensions(&fixture).await;
        let (mut panel, egui_context) = panel(&fixture, PlotType::Field, "ball_percepts");
        let observed_percepts = percepts_observation(&fixture);
        eventually(
            || async {
                dimensions
                    .publish_with_source_time(&FieldDimensions::SPL_2025, time(0))
                    .await
                    .unwrap();
                fixture.publish_pose(0, Pose2::zero()).await;
                percepts
                    .publish_with_source_time(&ball_percept(), time(0))
                    .await
                    .unwrap();
            },
            || {
                observed_percepts.latest().is_some()
                    && panel
                        .ground_frames
                        .alignment()
                        .ground_to_current_ground(time(0))
                        .is_some()
            },
        )
        .await;

        assert!(render(&mut panel, &fixture.backend, &egui_context, Color32::WHITE).is_none());
    }

    fn percepts_observation(fixture: &Fixture) -> ros_z_debug::TopicObservation<Vec<BallPercept>> {
        let _runtime_handle = fixture.backend.runtime_handle().enter();
        fixture
            .backend
            .observer()
            .observe_typed("ball_filter/ball_percepts")
            .unwrap()
            .spawn()
    }
}

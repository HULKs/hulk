use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::egui::{Color32, Stroke};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Point2, Pose2};
use types::voronoi::Ownership;

use crate::{
    panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct VoronoiCell {
    centroids: BufferHandle<Vec<Option<Point2<Field>>>>,
    voronoi_grid: BufferHandle<Vec<(Point2<Field>, Ownership)>>,
    input_points: BufferHandle<Option<Vec<Pose2<Field>>>>,
}

impl Layer<Field> for VoronoiCell {
    const NAME: &'static str = "Voronoi Cells";

    fn new(robot: Arc<Robot>) -> Self {
        let centroids = robot.subscribe_value("WorldState.main_outputs.centroids");
        let voronoi_grid = robot.subscribe_value("WorldState.main_outputs.voronoi_grid");
        let input_points =
            robot.subscribe_value("WorldState.additional_outputs.voronoi.input_points");

        Self {
            centroids,
            voronoi_grid,
            input_points,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &types::field_dimensions::FieldDimensions,
    ) -> Result<()> {
        if let Some(voronoi_grid) = self.voronoi_grid.get_last_value()? {
            let colors = [
                Color32::from_rgb(0, 114, 178),   // Dark Blue
                Color32::from_rgb(230, 159, 0),   // Orange
                Color32::from_rgb(204, 121, 167), // Reddish Purple
                Color32::from_rgb(86, 180, 233),  // Sky Blue
                Color32::from_rgb(213, 94, 0),    // Vermillion
                Color32::from_rgb(240, 228, 66),  // Yellow
                Color32::from_rgb(0, 0, 0),       // Black
                Color32::from_rgb(255, 255, 255), // White
                Color32::from_rgb(148, 103, 189), // Lavender
                Color32::from_rgb(227, 119, 194), // Pink
                Color32::from_rgb(127, 127, 127), // Grey
                Color32::from_rgb(188, 189, 34),  // Olive
            ];

            for (point, ownership) in voronoi_grid {
                let color = match ownership {
                    Ownership::Blocked => Color32::from_gray(40),
                    Ownership::Robot(player_number) => {
                        let color_index = match player_number {
                            PlayerNumber::One => 0,
                            PlayerNumber::Two => 1,
                            PlayerNumber::Three => 2,
                            PlayerNumber::Four => 3,
                            PlayerNumber::Five => 4,
                        };
                        colors[color_index % colors.len()]
                    }
                    Ownership::Free => Color32::from_gray(120),
                };

                painter.circle_filled(point, 0.02, color);
                painter.circle_stroke(point, 0.02, Stroke::new(0.005, Color32::BLACK));
            }
        }

        if let Some(centroids) = self.centroids.get_last_value()? {
            for centroid in centroids.into_iter().flatten() {
                painter.target(
                    centroid,
                    0.06,
                    Stroke::new(0.01, Color32::GREEN),
                    Color32::RED,
                );
            }
        }
        let pose_color = Color32::from_white_alpha(127);
        let pose_stroke = Stroke {
            width: 0.02,
            color: Color32::BLACK,
        };
        if let Some(input_points) = self.input_points.get_last_value()?.flatten() {
            for input_point in input_points {
                painter.pose(input_point, 0.06, 0.1, pose_color, pose_stroke);
            }
        }

        Ok(())
    }
}

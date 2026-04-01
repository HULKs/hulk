use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::egui::{Color32, Stroke};
use linear_algebra::Point2;

use crate::{
    panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct VoronoiCell {
    centroids: BufferHandle<Vec<Option<Point2<Field>>>>,
    voronoi_cells: BufferHandle<Vec<Vec<Point2<Field>>>>,
    input_points: BufferHandle<Option<Vec<Point2<Field>>>>,
}

impl Layer<Field> for VoronoiCell {
    const NAME: &'static str = "Voronoi Cells";

    fn new(robot: Arc<Robot>) -> Self {
        let centroids = robot.subscribe_value("WorldState.main_outputs.centroids");
        let voronoi_cells = robot.subscribe_value("WorldState.main_outputs.voronoi_cells");
        let input_points =
            robot.subscribe_value("WorldState.additional_outputs.voronoi.input_points");

        Self {
            centroids,
            voronoi_cells,
            input_points,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &types::field_dimensions::FieldDimensions,
    ) -> Result<()> {
        if let Some(voronoi_cells) = self.voronoi_cells.get_last_value()? {
            let colors = [
                Color32::from_rgba_unmultiplied(255, 0, 0, 255),
                Color32::from_rgba_unmultiplied(0, 255, 0, 255),
                Color32::from_rgba_unmultiplied(0, 0, 255, 255),
                Color32::from_rgba_unmultiplied(255, 255, 0, 255),
            ];

            for (index, cell) in voronoi_cells.into_iter().enumerate() {
                let color = colors[index % colors.len()];
                for point in cell {
                    painter.circle_filled(point, 0.02, color);
                }
            }
        }

        if let Some(centroids) = self.centroids.get_last_value()? {
            for centroid in centroids.into_iter().flatten() {
                painter.target(
                    centroid,
                    0.06,
                    Stroke::new(0.01, Color32::GREEN),
                    Color32::from_rgb(255, 80, 80),
                );
            }
        }

        if let Some(input_points) = self.input_points.get_last_value()?.flatten() {
            for input_point in input_points {
                painter.target(
                    input_point,
                    0.04,
                    Stroke::new(0.01, Color32::BLACK),
                    Color32::from_rgb(255, 200, 0),
                );
            }
        }

        Ok(())
    }
}

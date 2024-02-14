use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;
use types::{field_dimensions::FieldDimensions, path_obstacles::PathObstacle};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct PathObstacles {
    robot_to_field: ValueBuffer,
    path_obstacles: ValueBuffer,
}

impl Layer for PathObstacles {
    const NAME: &'static str = "Path Obstacles";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field = nao.subscribe_output("Control.main.robot_to_field");
        let path_obstacles = nao.subscribe_output("Control.additional.path_obstacles");
        Self {
            robot_to_field,
            path_obstacles,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Isometry2<f32> = self.robot_to_field.require_latest()?;
        let path_obstacles: Vec<PathObstacle> = self.path_obstacles.require_latest()?;

        let path_obstacle_stroke = Stroke {
            width: 0.025,
            color: Color32::RED,
        };
        for path_obstacle in path_obstacles {
            match path_obstacle.shape {
                types::path_obstacles::PathObstacleShape::Circle(circle) => painter.circle_stroke(
                    robot_to_field * circle.center,
                    circle.radius,
                    path_obstacle_stroke,
                ),
                types::path_obstacles::PathObstacleShape::LineSegment(line_segment) => painter
                    .line_segment(
                        robot_to_field * line_segment.0,
                        robot_to_field * line_segment.1,
                        path_obstacle_stroke,
                    ),
            }
        }
        Ok(())
    }
}

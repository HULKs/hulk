use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use types::{
    field_dimensions::FieldDimensions,
    path_obstacles::{PathObstacle, PathObstacleShape},
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct PathObstacles {
    path_obstacles: BufferHandle<Option<Vec<PathObstacle>>>,
}

impl Layer<Ground> for PathObstacles {
    const NAME: &'static str = "Path Obstacles";

    fn new(nao: Arc<Nao>) -> Self {
        let path_obstacles = nao.subscribe_value("Control.additional_outputs.path_obstacles");
        Self { path_obstacles }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(path_obstacles) = self.path_obstacles.get_last_value()?.flatten() {
            let path_obstacle_stroke = Stroke {
                width: 0.025,
                color: Color32::RED,
            };
            for path_obstacle in path_obstacles {
                match path_obstacle.shape {
                    PathObstacleShape::Circle(circle) => {
                        painter.circle_stroke(circle.center, circle.radius, path_obstacle_stroke)
                    }
                    PathObstacleShape::LineSegment(line_segment) => {
                        painter.line_segment(line_segment.0, line_segment.1, path_obstacle_stroke)
                    }
                }
            }
        }

        Ok(())
    }
}

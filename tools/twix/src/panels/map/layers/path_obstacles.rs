use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::Ground;
use types::{field_dimensions::FieldDimensions, path_obstacles::PathObstacle};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct PathObstacles {
    path_obstacles: ValueBuffer,
}

impl Layer<Ground> for PathObstacles {
    const NAME: &'static str = "Path Obstacles";

    fn new(nao: Arc<Nao>) -> Self {
        let path_obstacles = nao
            .subscribe_output(CyclerOutput::from_str("Control.additional.path_obstacles").unwrap());
        Self { path_obstacles }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let path_obstacles: Vec<PathObstacle> = self.path_obstacles.require_latest()?;

        let path_obstacle_stroke = Stroke {
            width: 0.025,
            color: Color32::RED,
        };
        for path_obstacle in path_obstacles {
            match path_obstacle.shape {
                types::path_obstacles::PathObstacleShape::Circle(circle) => {
                    painter.circle_stroke(circle.center, circle.radius, path_obstacle_stroke)
                }
                types::path_obstacles::PathObstacleShape::LineSegment(line_segment) => {
                    painter.line_segment(line_segment.0, line_segment.1, path_obstacle_stroke)
                }
            }
        }
        Ok(())
    }
}

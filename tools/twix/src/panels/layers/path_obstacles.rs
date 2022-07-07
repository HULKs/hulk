use std::str::FromStr;

use communication::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use log::error;
use nalgebra::Isometry2;
use serde_json::from_value;
use types::PathObstacle;

use crate::{panels::Layer, value_buffer::ValueBuffer};

pub struct PathObstacles {
    robot_to_field: ValueBuffer,
    path_obstacles: ValueBuffer,
}

impl Layer for PathObstacles {
    const NAME: &'static str = "Path Obstacles";

    fn new(nao: std::sync::Arc<crate::nao::Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("control.main.robot_to_field").unwrap());
        let path_obstacles = nao
            .subscribe_output(CyclerOutput::from_str("control.additional.path_obstacles").unwrap());
        Self {
            robot_to_field,
            path_obstacles,
        }
    }

    fn paint(
        &self,
        painter: &crate::twix_paint::TwixPainter,
        _field_dimensions: &types::FieldDimensions,
    ) {
        let robot_to_field: Option<Isometry2<f32>> = match self.robot_to_field.get_latest() {
            Ok(value) => from_value(value).unwrap(),
            Err(error) => return error!("{:?}", error),
        };
        let path_obstacles: Option<Vec<PathObstacle>> = match self.path_obstacles.get_latest() {
            Ok(value) => from_value(value).unwrap(),
            Err(error) => return error!("{:?}", error),
        };

        let robot_to_field = robot_to_field.unwrap_or_default();

        if let Some(path_obstacles) = path_obstacles {
            let path_obstacle_stroke = Stroke {
                width: 0.025,
                color: Color32::RED,
            };
            for path_obstacle in path_obstacles {
                match path_obstacle.shape {
                    types::PathObstacleShape::Circle(circle) => painter.circle_stroke(
                        robot_to_field * circle.center,
                        circle.radius,
                        path_obstacle_stroke,
                    ),
                    types::PathObstacleShape::LineSegment(line_segment) => painter.line_segment(
                        robot_to_field * line_segment.0,
                        robot_to_field * line_segment.1,
                        path_obstacle_stroke,
                    ),
                }
            }
        }
    }
}

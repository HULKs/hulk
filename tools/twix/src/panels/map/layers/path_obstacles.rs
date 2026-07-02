use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use ros_z_debug::{SampleRecord, TopicObservation};
use types::{field_dimensions::FieldDimensions, path_obstacles::PathObstacleShape};
use world_state::behavior::node::Blackboard;

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct PathObstacles {
    blackboard: TopicObservation<Blackboard>,
}

impl Layer<Ground> for PathObstacles {
    const NAME: &'static str = "Path Obstacles";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let blackboard = backend
            .observer()
            .observe_typed("behavior/blackboard")
            .expect("failed to construct blackboard observer")
            .spawn();

        Self { blackboard }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let latest_blackboard_sample = self.blackboard.latest();

        let Some(SampleRecord {
            value: blackboard, ..
        }) = latest_blackboard_sample.as_deref()
        else {
            return Ok(());
        };

        let path_obstacle_stroke = Stroke {
            width: 0.025,
            color: Color32::RED,
        };
        for path_obstacle in &blackboard.path_obstacles_output {
            match path_obstacle.shape {
                PathObstacleShape::Circle(circle) => {
                    painter.circle_stroke(circle.center, circle.radius, path_obstacle_stroke)
                }
                PathObstacleShape::LineSegment(line_segment) => {
                    painter.line_segment(line_segment.0, line_segment.1, path_obstacle_stroke)
                }
            }
        }

        Ok(())
    }
}

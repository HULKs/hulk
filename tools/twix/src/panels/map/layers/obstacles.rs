use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use types::{field_dimensions::FieldDimensions, obstacles::Obstacle};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Obstacles {
    obstacles: BufferHandle<Vec<Obstacle>>,
}

impl Layer<Ground> for Obstacles {
    const NAME: &'static str = "Obstacles";

    fn new(nao: Arc<Nao>) -> Self {
        let obstacles = nao.subscribe_value("Control.main_outputs.obstacles");
        Self { obstacles }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(obstacles) = self.obstacles.get_last_value()? {
            let hip_height_stroke = Stroke {
                width: 0.025,
                color: Color32::RED,
            };
            let foot_height_stroke = Stroke {
                width: 0.025,
                color: Color32::BLUE,
            };
            for obstacle in obstacles {
                painter.circle_stroke(
                    obstacle.position,
                    obstacle.radius_at_hip_height,
                    hip_height_stroke,
                );
                painter.circle_stroke(
                    obstacle.position,
                    obstacle.radius_at_foot_height,
                    foot_height_stroke,
                );
            }
        }
        Ok(())
    }
}

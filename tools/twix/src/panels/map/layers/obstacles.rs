use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::Ground;
use types::{field_dimensions::FieldDimensions, obstacles::Obstacle};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct Obstacles {
    obstacles: ValueBuffer,
}

impl Layer<Ground> for Obstacles {
    const NAME: &'static str = "Obstacles";

    fn new(nao: Arc<Nao>) -> Self {
        let obstacles =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.obstacles").unwrap());
        Self { obstacles }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let obstacles: Vec<Obstacle> = self.obstacles.require_latest()?;

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
        Ok(())
    }
}

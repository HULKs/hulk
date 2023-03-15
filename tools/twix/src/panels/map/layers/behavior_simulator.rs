use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;
use types::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, players_value_buffer::PlayersValueBuffer,
    twix_painter::TwixPainter,
};

pub struct BehaviorSimulator {
    robot_to_field: PlayersValueBuffer,
}

impl Layer for BehaviorSimulator {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field = PlayersValueBuffer::try_new(
            nao,
            "BehaviorSimulator.main.databases",
            "main_outputs.robot_to_field",
        )
        .unwrap();
        Self { robot_to_field }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        for (_player_number, value_buffer) in self.robot_to_field.0.iter() {
            let Ok(robot_to_field): Result<Isometry2<f32>> = value_buffer.parse_latest() else {
                continue
            };

            let pose_color = Color32::from_white_alpha(63);
            let pose_stroke = Stroke {
                width: 0.02,
                color: Color32::BLACK,
            };
            painter.pose(robot_to_field, 0.15, 0.25, pose_color, pose_stroke);
        }

        Ok(())
    }
}

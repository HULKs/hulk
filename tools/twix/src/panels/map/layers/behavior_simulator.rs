use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;
use types::{FieldDimensions, Players};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

struct PlayersValueBuffer(Players<ValueBuffer>);

impl PlayersValueBuffer {
    pub fn try_new(nao: Arc<Nao>, prefix: &str, output: &str) -> Result<Self> {
        let buffers = Players {
            one: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.one.{output}"))?),
            two: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.two.{output}"))?),
            three: nao
                .subscribe_output(CyclerOutput::from_str(&format!("{prefix}.three.{output}"))?),
            four: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.four.{output}"))?),
            five: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.five.{output}"))?),
        };

        Ok(Self(buffers))
    }
}

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

use std::{collections::HashMap, str::FromStr, sync::Arc};

use behavior_simulator::cycler::Database;
use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;
use spl_network_messages::PlayerNumber;
use types::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BehaviorSimulator {
    databases: ValueBuffer,
}

impl Layer for BehaviorSimulator {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>) -> Self {
        let databases = nao
            .subscribe_output(CyclerOutput::from_str("BehaviorSimulator.main.databases").unwrap());
        Self { databases }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let databases: HashMap<PlayerNumber, Database> = self.databases.require_latest()?;

        for database in databases.values() {
            let robot_to_field: Isometry2<f32> = database.main_outputs.robot_to_field.unwrap();

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

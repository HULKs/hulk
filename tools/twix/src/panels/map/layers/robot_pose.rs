use std::{str::FromStr, sync::Arc};

use behavior_simulator;
use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;
use types::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RobotPose {
    robot_to_field: ValueBuffer,
    databases: ValueBuffer,
}

impl Layer for RobotPose {
    const NAME: &'static str = "Robot Pose";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.robot_to_field").unwrap());
        let databases = nao
            .subscribe_output(CyclerOutput::from_str("BehaviorSimulator.main.databases").unwrap());
        Self {
            robot_to_field,
            databases,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let databases: Vec<behavior_simulator::cycler::Database> =
            self.databases.require_latest()?;

        for database in &databases {
            let robot_to_field: Isometry2<f32> = database.main_outputs.robot_to_field.unwrap();

            let pose_color = Color32::from_white_alpha(127);
            let pose_stroke = Stroke {
                width: 0.02,
                color: Color32::BLACK,
            };
            painter.pose(robot_to_field, 0.15, 0.25, pose_color, pose_stroke);
        }
        let robot_to_field: Isometry2<f32> = self.robot_to_field.require_latest()?;

        let pose_color = Color32::from_white_alpha(127);
        let pose_stroke = Stroke {
            width: 0.02,
            color: Color32::BLACK,
        };
        painter.pose(robot_to_field, 0.15, 0.25, pose_color, pose_stroke);
        Ok(())
    }
}

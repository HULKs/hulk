use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;
use types::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RobotPose {
    robot_to_field: ValueBuffer,
}

impl Layer for RobotPose {
    const NAME: &'static str = "Robot Pose";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("control.main.robot_to_field").unwrap());
        Self { robot_to_field }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
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

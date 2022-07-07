use std::{str::FromStr, sync::Arc};

use communication::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use log::error;
use nalgebra::Isometry2;
use serde_json::from_value;
use types::FieldDimensions;

use crate::{nao::Nao, panels::Layer, twix_paint::TwixPainter, value_buffer::ValueBuffer};

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

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) {
        let robot_to_field: Option<Isometry2<f32>> = match self.robot_to_field.get_latest() {
            Ok(value) => from_value(value).unwrap(),
            Err(error) => return error!("{:?}", error),
        };

        if let Some(robot_to_field) = robot_to_field {
            let pose_color = Color32::from_white_alpha(127);
            let pose_stroke = Stroke {
                width: 0.02,
                color: Color32::BLACK,
            };
            painter.pose(robot_to_field, 0.15, 0.25, pose_color, pose_stroke);
        }
    }
}

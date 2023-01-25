use std::str::FromStr;

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;

use types::{FieldDimensions, Line2};

use crate::{panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer};

pub struct Lines {
    robot_to_field: ValueBuffer,
    lines_in_robot_bottom: ValueBuffer,
    lines_in_robot_top: ValueBuffer,
}

impl Layer for Lines {
    const NAME: &'static str = "Lines";

    fn new(nao: std::sync::Arc<crate::nao::Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.robot_to_field").unwrap());
        let lines_in_robot_bottom = nao.subscribe_output(
            CyclerOutput::from_str("VisionBottom.main.line_data.lines_in_robot").unwrap(),
        );
        let lines_in_robot_top = nao.subscribe_output(
            CyclerOutput::from_str("VisionTop.main.line_data.lines_in_robot").unwrap(),
        );
        Self {
            robot_to_field,
            lines_in_robot_bottom,
            lines_in_robot_top,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Isometry2<f32> = self.robot_to_field.parse_latest().unwrap_or_default();
        let lines: Vec<Line2> = [&self.lines_in_robot_bottom, &self.lines_in_robot_top]
            .iter()
            .filter_map(|buffer| buffer.parse_latest::<Vec<_>>().ok())
            .flatten()
            .collect();
        for line in lines {
            painter.line_segment(
                robot_to_field * line.0,
                robot_to_field * line.1,
                Stroke::new(0.04, Color32::RED),
            );
        }
        Ok(())
    }
}

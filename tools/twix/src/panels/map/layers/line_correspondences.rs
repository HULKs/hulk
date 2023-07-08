use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;

use types::{FieldDimensions, Line2};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct LineCorrespondences {
    robot_to_field: ValueBuffer,
    lines_in_robot_bottom: ValueBuffer,
    lines_in_robot_top: ValueBuffer,
}

impl Layer for LineCorrespondences {
    const NAME: &'static str = "Line Associations";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.robot_to_field").unwrap());
        let lines_in_robot_bottom = nao.subscribe_output(
            CyclerOutput::from_str("VisionBottom.additional.localization.correspondence_lines").unwrap(),
        );
        let lines_in_robot_top = nao.subscribe_output(
            CyclerOutput::from_str("VisionTop.additional.localization.correspondence_lines").unwrap(),
        );
        Self {
            robot_to_field,
            lines_in_robot_bottom,
            lines_in_robot_top,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Isometry2<f32> = self.robot_to_field.parse_latest().unwrap_or_default();
        for line_set_buffer in [&self.lines_in_robot_bottom, &self.lines_in_robot_top] {
            let lines = match line_set_buffer
                .parse_latest::<Vec<Line2>>()
                {
                    Ok(value) => value,
                    Err(error) => {println!("{error:?}"); Default::default() }
                };
            for line in lines {
                painter.line_segment(
                    line.0,
                    line.1,
                    Stroke::new(0.02, Color32::YELLOW),
                );
            }
        }
        Ok(())
    }
}

use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use types::{field_dimensions::FieldDimensions, line::Line2};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct LineCorrespondences {
    lines_in_robot_bottom: ValueBuffer,
    lines_in_robot_top: ValueBuffer,
}

impl Layer for LineCorrespondences {
    const NAME: &'static str = "Line Correspondences";

    fn new(nao: Arc<Nao>) -> Self {
        let lines_in_robot_bottom =
            nao.subscribe_output("VisionBottom.additional.localization.correspondence_lines");
        let lines_in_robot_top =
            nao.subscribe_output("VisionTop.additional.localization.correspondence_lines");
        Self {
            lines_in_robot_bottom,
            lines_in_robot_top,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        for line_set_buffer in [&self.lines_in_robot_bottom, &self.lines_in_robot_top] {
            let lines = match line_set_buffer.parse_latest::<Vec<Line2>>() {
                Ok(value) => value,
                Err(error) => {
                    println!("{error:?}");
                    Default::default()
                }
            };
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.02, Color32::YELLOW));
            }
        }
        Ok(())
    }
}

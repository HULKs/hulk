use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::{Field, Ground};
use geometry::line::Line2;
use linear_algebra::Isometry2;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct Lines {
    ground_to_field: ValueBuffer,
    lines_in_ground_bottom: ValueBuffer,
    lines_in_ground_top: ValueBuffer,
}

impl Layer for Lines {
    const NAME: &'static str = "Lines";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ground_to_field").unwrap());
        let lines_in_ground_bottom = nao.subscribe_output(
            CyclerOutput::from_str("VisionBottom.main.line_data.lines_in_ground").unwrap(),
        );
        let lines_in_ground_top = nao.subscribe_output(
            CyclerOutput::from_str("VisionTop.main.line_data.lines_in_ground").unwrap(),
        );
        Self {
            ground_to_field,
            lines_in_ground_bottom,
            lines_in_ground_top,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_field: Isometry2<Ground, Field> =
            self.ground_to_field.parse_latest().unwrap_or_default();
        let lines: Vec<Line2<Ground>> = [&self.lines_in_ground_bottom, &self.lines_in_ground_top]
            .iter()
            .filter_map(|buffer| buffer.parse_latest::<Vec<_>>().ok())
            .flatten()
            .collect();
        for line in lines {
            painter.line_segment(
                ground_to_field * line.0,
                ground_to_field * line.1,
                Stroke::new(0.04, Color32::RED),
            );
        }
        Ok(())
    }
}

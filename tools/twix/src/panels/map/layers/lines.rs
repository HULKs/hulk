use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::Field;
use geometry::line::Line2;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct Lines {
    lines_in_field: ValueBuffer,
}

impl Layer<Field> for Lines {
    const NAME: &'static str = "Lines";

    fn new(nao: Arc<Nao>) -> Self {
        let lines_in_field = nao.subscribe_output(
            CyclerOutput::from_str("Control.additional.localization.measured_lines_in_field")
                .unwrap(),
        );
        Self { lines_in_field }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let lines = self.lines_in_field.parse_latest::<Vec<Line2<Field>>>()?;
        for line in lines {
            painter.line_segment(line.0, line.1, Stroke::new(0.04, Color32::RED));
        }
        Ok(())
    }
}

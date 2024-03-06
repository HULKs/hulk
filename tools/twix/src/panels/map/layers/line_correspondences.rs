use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::Field;
use types::{field_dimensions::FieldDimensions, line::Line2};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct LineCorrespondences {
    correspondence_lines: ValueBuffer,
}

impl Layer for LineCorrespondences {
    const NAME: &'static str = "Line Correspondences";

    fn new(nao: Arc<Nao>) -> Self {
        let correspondence_lines = nao.subscribe_output(
            CyclerOutput::from_str("Control.additional.localization.correspondence_lines").unwrap(),
        );
        Self {
            correspondence_lines,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let lines = match self
            .correspondence_lines
            .parse_latest::<Vec<Line2<Field>>>()
        {
            Ok(value) => value,
            Err(error) => {
                println!("{error:?}");
                Default::default()
            }
        };
        for line in lines {
            painter.line_segment(line.0, line.1, Stroke::new(0.02, Color32::YELLOW));
        }
        Ok(())
    }
}

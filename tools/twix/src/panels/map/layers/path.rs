use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::Color32;

use communication::client::CyclerOutput;
use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use types::{field_dimensions::FieldDimensions, motion_command::MotionCommand};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct Path {
    ground_to_field: ValueBuffer,
    motion_command: ValueBuffer,
}

impl Layer for Path {
    const NAME: &'static str = "Path";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ground_to_field").unwrap());
        let motion_command =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.motion_command").unwrap());
        Self {
            ground_to_field,
            motion_command,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_field: Isometry2<Ground, Field> = self.ground_to_field.require_latest()?;
        let motion_command: MotionCommand = self.motion_command.require_latest()?;

        if let MotionCommand::Walk { path, .. } = motion_command {
            painter.path(
                ground_to_field,
                path,
                Color32::BLUE,
                Color32::LIGHT_BLUE,
                0.025,
            );
        }
        Ok(())
    }
}

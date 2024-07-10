use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::Color32;

use coordinate_systems::Ground;

use types::{field_dimensions::FieldDimensions, motion_command::MotionCommand};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Path {
    motion_command: BufferHandle<MotionCommand>,
}

impl Layer<Ground> for Path {
    const NAME: &'static str = "Path";

    fn new(nao: Arc<Nao>) -> Self {
        let motion_command = nao.subscribe_value("Control.main_outputs.motion_command");
        Self { motion_command }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(MotionCommand::Walk { path, .. }) = self.motion_command.get_last_value()? {
            painter.path(path, Color32::BLUE, Color32::LIGHT_BLUE, 0.025);
        }
        Ok(())
    }
}

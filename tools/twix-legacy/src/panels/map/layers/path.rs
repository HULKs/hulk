use std::sync::Arc;

use color_eyre::Result;
use eframe::{egui::Stroke, epaint::Color32};

use coordinate_systems::Ground;

use types::{
    behavior_command::BehaviorCommand, field_dimensions::FieldDimensions, path::traits::EndPoints,
};

use crate::{
    panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Path {
    behavior_command: BufferHandle<BehaviorCommand>,
}

impl Layer<Ground> for Path {
    const NAME: &'static str = "Path";

    fn new(robot: Arc<Robot>) -> Self {
        let behavior_command = robot.subscribe_value("WorldState.main_outputs.behavior_command");
        Self { behavior_command }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(BehaviorCommand::Walk {
            path,
            target_orientation,
            ..
        }) = self.behavior_command.get_last_value()?
        {
            let path_end_point = path.end_point();
            let target_direction = target_orientation.as_unit_vector();
            painter.line_segment(
                path_end_point,
                path_end_point + target_direction * 0.1,
                Stroke::new(0.01_f32, Color32::PURPLE),
            );
            painter.path(path, Color32::BLUE, Color32::LIGHT_BLUE, 0.025);
        }
        Ok(())
    }
}

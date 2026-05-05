use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::Color32;
use linear_algebra::Point2;

use coordinate_systems::Ground;
use types::field_dimensions::FieldDimensions;

use crate::{
    panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct WalkPosition {
    walk_position: BufferHandle<Option<Option<Point2<Ground>>>>,
}

impl Layer<Ground> for WalkPosition {
    const NAME: &'static str = "Walk Position";

    fn new(robot: Arc<Robot>) -> Self {
        let walk_position =
            robot.subscribe_value("WorldState.additional_outputs.behavior.walk_position");
        Self { walk_position }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(Some(position)) = self.walk_position.get_last_value()?.flatten() {
            painter.circle_filled(position, 0.05, Color32::BLUE);
        }

        Ok(())
    }
}

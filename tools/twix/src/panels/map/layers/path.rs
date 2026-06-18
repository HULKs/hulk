use std::sync::Arc;

use color_eyre::Result;
use eframe::{egui::Stroke, epaint::Color32};

use coordinate_systems::Ground;
use ros_z_debug::RetentionPolicy;

use types::{
    field_dimensions::FieldDimensions, motion_command::MotionCommand, path::traits::EndPoints,
};

use crate::{
    backend::{TwixBackend, retained_subscription::TypedSubscription},
    panels::map::{latest_value, layer::Layer},
    twix_painter::TwixPainter,
};

pub struct Path {
    motion_command: TypedSubscription<MotionCommand>,
}

impl Layer<Ground> for Path {
    const NAME: &'static str = "Path";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let motion_command = backend.subscribe_typed_retained(
            "behavior/motion_command",
            RetentionPolicy::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        Self { motion_command }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(MotionCommand::Walk {
            path,
            target_orientation,
            ..
        }) = latest_value(&self.motion_command)
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

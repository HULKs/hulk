use std::sync::Arc;

use color_eyre::Result;
use eframe::{egui::Stroke, epaint::Color32};

use coordinate_systems::Ground;
use ros_z_debug::{SampleRecord, TopicObservation};
use types::{
    behavior_command::BehaviorCommand, field_dimensions::FieldDimensions, path::traits::EndPoints,
};

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct Path {
    behavior_command: TopicObservation<BehaviorCommand>,
}

impl Layer<Ground> for Path {
    const NAME: &'static str = "Path";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let behavior_command = backend
            .observer()
            .observe_typed("behavior/behavior_command")
            .expect("failed to construct behavior command observer")
            .spawn();

        Self { behavior_command }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(SampleRecord {
            value:
                BehaviorCommand::Walk {
                    path,
                    target_orientation,
                    ..
                },
            ..
        }) = self.behavior_command.latest().as_deref()
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

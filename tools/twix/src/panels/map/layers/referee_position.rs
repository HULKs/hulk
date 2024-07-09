use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::Point2;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct RefereePosition {
    expected_referee_position: BufferHandle<Option<Point2<Field>>>,
    distance_to_referee_position_threshold: BufferHandle<f32>,
}

impl Layer<Field> for RefereePosition {
    const NAME: &'static str = "Referee Position";

    fn new(nao: Arc<Nao>) -> Self {
        let expected_referee_position =
            nao.subscribe_value("Control.main_outputs.expected_referee_position");
        let distance_to_referee_position_threshold = nao.subscribe_value(
            "parameters.object_detection.object_detection_top.distance_to_referee_position_threshold",
        );
        Self {
            expected_referee_position,
            distance_to_referee_position_threshold,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimension: &types::field_dimensions::FieldDimensions,
    ) -> Result<()> {
        let position_stroke = Stroke {
            width: 0.05,
            color: Color32::BLACK,
        };
        if let Some(expected_referee_position_ground) =
            self.expected_referee_position.get_last_value()?.flatten()
        {
            painter.circle(
                expected_referee_position_ground,
                0.15,
                Color32::BLUE,
                position_stroke,
            );

            if let Some(distance_to_referee_position_threshold) = self
                .distance_to_referee_position_threshold
                .get_last_value()?
            {
                painter.circle(
                    expected_referee_position_ground,
                    distance_to_referee_position_threshold,
                    Color32::TRANSPARENT,
                    position_stroke,
                );
            }
        };
        Ok(())
    }
}

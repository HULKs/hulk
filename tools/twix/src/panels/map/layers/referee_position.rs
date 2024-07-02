use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use coordinate_systems::Field;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::Point2;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RefereePosition {
    expected_referee_position: ValueBuffer,
    distance_to_referee_position_threshold: ValueBuffer,
}

impl Layer<Field> for RefereePosition {
    const NAME: &'static str = "Referee Position";

    fn new(nao: Arc<Nao>) -> Self {
        let expected_referee_position = nao.subscribe_output(
            CyclerOutput::from_str("Control.main.expected_referee_position").unwrap(),
        );
        let distance_to_referee_position_threshold =
            nao.subscribe_parameter("pose_detection.distance_to_referee_position_threshold");
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
        let expected_referee_position_ground: Option<Point2<Field>> =
            self.expected_referee_position.require_latest()?;

        let Some(expected_referee_position_ground) = expected_referee_position_ground else {
            return Ok(());
        };

        painter.circle(
            expected_referee_position_ground,
            0.15,
            Color32::BLUE,
            position_stroke,
        );

        let distance_to_referee_position_threshold: f32 = self
            .distance_to_referee_position_threshold
            .require_latest()?;
        painter.circle(
            expected_referee_position_ground,
            distance_to_referee_position_threshold,
            Color32::TRANSPARENT,
            position_stroke,
        );
        Ok(())
    }
}

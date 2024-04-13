use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use coordinate_systems::{Field, Ground};
use eframe::epaint::{Color32, Stroke};
use linear_algebra::{Isometry2, Point2};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RefereePosition {
    expected_referee_position: ValueBuffer,
    ground_to_field: ValueBuffer,
    distance_to_referee_position_threshold: ValueBuffer,
}

impl Layer<Field> for RefereePosition {
    const NAME: &'static str = "Referee Position";

    fn new(nao: Arc<Nao>) -> Self {
        let expected_referee_position = nao.subscribe_output(
            CyclerOutput::from_str("Control.main.expected_referee_position").unwrap(),
        );
        let ground_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ground_to_field").unwrap());
        let distance_to_referee_position_threshold = nao
            .subscribe_parameter("detection.detection_top.distance_to_referee_position_threshold");
        Self {
            expected_referee_position,
            ground_to_field,
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

        let ground_to_field: Isometry2<Ground, Field> = self.ground_to_field.require_latest()?;

        let expected_referee_position_ground: Point2<Ground> =
            self.expected_referee_position.require_latest()?;

        painter.circle(
            ground_to_field * expected_referee_position_ground,
            0.15,
            Color32::BLUE,
            position_stroke,
        );

        let distance_to_referee_position_threshold: f32 = self
            .distance_to_referee_position_threshold
            .require_latest()?;
        painter.circle(
            ground_to_field * expected_referee_position_ground,
            distance_to_referee_position_threshold,
            Color32::TRANSPARENT,
            position_stroke,
        );
        Ok(())
    }
}

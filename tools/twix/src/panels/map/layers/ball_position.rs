use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use eframe::epaint::Color32;

use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct BallPosition {
    ground_to_field: BufferHandle<Option<Isometry2<Ground, Field>>>,
    ball_position: BufferHandle<Option<ball_filter::BallPosition<Ground>>>,
}

impl Layer<Field> for BallPosition {
    const NAME: &'static str = "Ball Position";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field = nao.subscribe_buffered_value(
            "Control.main_outputs.ground_to_field",
            Duration::from_secs(2),
        );
        let ball_position = nao
            .subscribe_buffered_value("Control.main_outputs.ball_position", Duration::from_secs(2));
        Self {
            ground_to_field,
            ball_position,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_fields = self.ground_to_field.get()?;
        let ball_positions = self.ball_position.get()?;

        for (ball, ground_to_field) in ball_positions.iter().zip(ground_to_fields.iter()) {
            let Some(ball) = ball.value else {
                continue;
            };
            let ground_to_field = ground_to_field.value.unwrap_or_default();
            painter.circle_filled(
                ground_to_field * ball.position,
                field_dimensions.ball_radius,
                Color32::from_white_alpha(10),
            );
        }

        if let Some(ball) = self.ball_position.get_last_value()?.flatten() {
            let ground_to_field = self
                .ground_to_field
                .get_last_value()?
                .flatten()
                .unwrap_or_default();
            painter.ball(
                ground_to_field * ball.position,
                field_dimensions.ball_radius,
            );
        }
        Ok(())
    }
}

use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::Color32;

use communication::client::CyclerOutput;
use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallPosition {
    ground_to_field: ValueBuffer,
    ball_position: ValueBuffer,
}

impl Layer for BallPosition {
    const NAME: &'static str = "Ball Position";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ground_to_field").unwrap());
        ground_to_field.reserve(100);
        let ball_position =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ball_position").unwrap());
        ball_position.reserve(100);
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
        let ground_to_fields: Vec<Option<Isometry2<Ground, Field>>> =
            self.ground_to_field.parse_buffered()?;
        let ball_positions: Vec<Option<types::ball_position::BallPosition<Ground>>> =
            self.ball_position.parse_buffered()?;

        for (ball, ground_to_field) in ball_positions
            .iter()
            .rev()
            .zip(ground_to_fields.iter().rev())
        {
            if let Some(ball) = ball {
                painter.circle_filled(
                    ground_to_field.unwrap_or_default() * ball.position,
                    field_dimensions.ball_radius,
                    Color32::from_white_alpha(10),
                );
            }
        }

        if let (Some(Some(ball)), Some(ground_to_field)) = (
            &ball_positions.first().map(Option::as_ref),
            ground_to_fields.first(),
        ) {
            painter.ball(
                ground_to_field.unwrap_or_default() * ball.position,
                field_dimensions.ball_radius,
            );
        }
        Ok(())
    }
}

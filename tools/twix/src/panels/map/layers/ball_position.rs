use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::Color32;
use nalgebra::Isometry2;
use types::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallPosition {
    robot_to_field: ValueBuffer,
    ball_position: ValueBuffer,
}

impl Layer for BallPosition {
    const NAME: &'static str = "Ball Position";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.robot_to_field").unwrap());
        robot_to_field.set_buffer_capacity(100);
        let ball_position =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ball_position").unwrap());
        ball_position.set_buffer_capacity(100);
        Self {
            robot_to_field,
            ball_position,
        }
    }

    fn paint(&self, painter: &TwixPainter, field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_fields: Vec<Option<Isometry2<f32>>> = self.robot_to_field.parse_buffered()?;
        let ball_positions: Vec<Option<types::BallPosition>> =
            self.ball_position.parse_buffered()?;

        for (ball, robot_to_field) in ball_positions
            .iter()
            .rev()
            .zip(robot_to_fields.iter().rev())
        {
            if let Some(ball) = ball {
                painter.circle_filled(
                    robot_to_field.unwrap_or_default() * ball.position,
                    field_dimensions.ball_radius,
                    Color32::from_white_alpha(10),
                );
            }
        }

        if let (Some(Some(ball)), Some(robot_to_field)) = (
            &ball_positions.first().map(Option::as_ref),
            robot_to_fields.first(),
        ) {
            painter.ball(
                robot_to_field.unwrap_or_default() * ball.position,
                field_dimensions.ball_radius,
            );
        }
        Ok(())
    }
}

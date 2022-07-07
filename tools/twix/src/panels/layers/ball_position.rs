use std::{str::FromStr, sync::Arc};

use communication::CyclerOutput;
use eframe::epaint::Color32;
use log::error;
use nalgebra::Isometry2;
use serde_json::{from_value, Value::Array};
use types::FieldDimensions;

use crate::{nao::Nao, panels::Layer, twix_paint::TwixPainter, value_buffer::ValueBuffer};

pub struct BallPosition {
    robot_to_field: ValueBuffer,
    ball_position: ValueBuffer,
}

impl Layer for BallPosition {
    const NAME: &'static str = "Ball Position";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("control.main.robot_to_field").unwrap());
        robot_to_field.set_buffer_size(100);
        let ball_position =
            nao.subscribe_output(CyclerOutput::from_str("control.main.ball_position").unwrap());
        ball_position.set_buffer_size(100);
        Self {
            robot_to_field,
            ball_position,
        }
    }

    fn paint(&self, painter: &TwixPainter, field_dimensions: &FieldDimensions) {
        let robot_to_fields: Vec<Option<Isometry2<f32>>> = match self.robot_to_field.get_buffered()
        {
            Ok(value) => from_value(Array(value)).unwrap(),
            Err(error) => return error!("{:?}", error),
        };
        let ball_positions: Vec<Option<types::BallPosition>> =
            match self.ball_position.get_buffered() {
                Ok(value) => from_value(Array(value)).unwrap(),
                Err(error) => return error!("{:?}", error),
            };

        for (ball, robot_to_field) in ball_positions.iter().zip(robot_to_fields.iter()) {
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
    }
}

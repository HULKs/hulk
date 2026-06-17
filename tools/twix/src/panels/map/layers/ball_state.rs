use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::epaint::Color32;
use types::{field_dimensions::FieldDimensions, world_state::BallState as WorldBallState};

use crate::{
    backend::TwixBackend,
    panels::map::{BALL_STATE_QUEUE_DEPTH, layer::Layer},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct BallState {
    ball_state: BufferHandle<Option<WorldBallState>>,
}

impl Layer<Field> for BallState {
    const NAME: &'static str = "Ball State";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let ball_state = backend.subscribe_buffered_value_with_queue_depth(
            "ball_state",
            Duration::from_secs(2),
            BALL_STATE_QUEUE_DEPTH,
        );
        Self { ball_state }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ball_states = self.ball_state.get()?;

        for ball_state in ball_states.iter().filter_map(|datum| datum.value) {
            painter.circle_filled(
                ball_state.ball_in_field,
                field_dimensions.ball_radius,
                Color32::from_white_alpha(10),
            );
        }

        if let Some(ball_state) = ball_states.iter().rev().find_map(|datum| datum.value) {
            painter.ball(
                ball_state.ball_in_field,
                field_dimensions.ball_radius,
                Color32::WHITE,
            );
        }

        Ok(())
    }
}

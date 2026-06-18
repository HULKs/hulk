use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::epaint::Color32;
use ros_z::time::Time;
use types::{field_dimensions::FieldDimensions, world_state::BallState as WorldBallState};

use crate::{
    backend::{TwixBackend, retained_subscription::TypedSubscription},
    panels::map::{BALL_STATE_QUEUE_DEPTH, layer::Layer, retained_window, time_window_retention},
    twix_painter::TwixPainter,
};

pub struct BallState {
    ball_state: TypedSubscription<Option<WorldBallState>>,
}

impl Layer<Field> for BallState {
    const NAME: &'static str = "Ball State";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let ball_state = backend.subscribe_typed_retained(
            "ball_state",
            time_window_retention(Duration::from_secs(2)),
            BALL_STATE_QUEUE_DEPTH,
        );
        Self { ball_state }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ball_states =
            retained_window(&self.ball_state, Time::zero(), Time::from_nanos(i64::MAX));

        for ball_state in ball_states
            .iter()
            .filter_map(|record| record.value.as_ref())
        {
            painter.circle_filled(
                ball_state.ball_in_field,
                field_dimensions.ball_radius,
                Color32::from_white_alpha(10),
            );
        }

        if let Some(ball_state) = ball_states
            .iter()
            .rev()
            .find_map(|record| record.value.as_ref())
        {
            painter.ball(
                ball_state.ball_in_field,
                field_dimensions.ball_radius,
                Color32::WHITE,
            );
        }

        Ok(())
    }
}

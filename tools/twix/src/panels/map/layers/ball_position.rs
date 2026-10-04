use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};
use std::{sync::Arc, time::Duration};

use color_eyre::{Result, eyre::OptionExt};
use eframe::epaint::Color32;

use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use ros_z_debug::{RetentionPolicy, SampleRecord, TopicObservation};
use types::field_dimensions::FieldDimensions;

use crate::panels::ball_visualization::TEAM_BALL_COLOR;
use crate::{backend::RobotBackend, panels::map::layer::Layer};
use twix_visualization::twix_painter::TwixPainter;

pub struct BallPosition {
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
    ball_position: TopicObservation<Option<types::ball_position::BallPosition<Ground>>>,
    team_ball: TopicObservation<Option<types::ball_position::BallPosition<Field>>>,
}

impl Layer<Field> for BallPosition {
    const NAME: &'static str = "Ball Filter";
    const STORAGE_KEY: Option<&'static str> = Some("selected_ball_filter");

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let ground_to_field = backend
            .observer()
            .observe_typed("ground_to_field")
            .expect("failed to construct ground_to_field observer")
            .retention(RetentionPolicy::time_window(Duration::from_secs(2)).unwrap())
            .spawn();

        let ball_position = backend
            .observer()
            .observe_typed("ball_filter/ball_position")
            .expect("failed to construct ball_position observer")
            .retention(RetentionPolicy::time_window(Duration::from_secs(2)).unwrap())
            .spawn();

        let team_ball = backend
            .observer()
            .observe_typed("team_ball")
            .expect("failed to construct team_ball observer")
            .spawn();

        Self {
            ground_to_field,
            ball_position,
            team_ball,
        }
    }

    fn repaint_on_updates(&self, context: &impl ObservationContext) -> Vec<ObservationRepaint> {
        vec![
            self.ground_to_field.repaint_on_updates(context),
            self.ball_position.repaint_on_updates(context),
            self.team_ball.repaint_on_updates(context),
        ]
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        for ball_position_sample in self.ball_position.get_all() {
            let SampleRecord {
                value: Some(ball),
                source_time,
                ..
            } = *ball_position_sample
            else {
                continue;
            };

            let ground_to_field = self
                .ground_to_field
                .get_nearest(source_time)
                .ok_or_eyre("failed to find matching ground_to_field")?
                .value;

            painter.circle_filled(
                ground_to_field * ball.position,
                field_dimensions.ball_radius,
                Color32::BLUE.gamma_multiply(10.0 / 255.0),
            );
        }

        if let Some(SampleRecord {
            value: Some(ball), ..
        }) = self.team_ball.latest().as_deref()
        {
            painter.ball(ball.position, field_dimensions.ball_radius, TEAM_BALL_COLOR);
        }

        if let Some(SampleRecord {
            value: Some(ball), ..
        }) = self.ball_position.latest().as_deref()
            && let Some(SampleRecord {
                value: ground_to_field,
                ..
            }) = self.ground_to_field.latest().as_deref()
        {
            painter.ball(
                ground_to_field * ball.position,
                field_dimensions.ball_radius,
                Color32::BLUE,
            );
        }
        Ok(())
    }
}

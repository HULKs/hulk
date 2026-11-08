use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use eframe::epaint::Color32;

use coordinate_systems::{Field, Ground};
use ros_z_debug::{RetentionPolicy, SampleRecord, TopicObservation};
use types::field_dimensions::FieldDimensions;

use crate::{
    backend::RobotBackend,
    panels::map::{ground_frames::GroundFrames, layer::Layer},
};
use twix_visualization::twix_painter::TwixPainter;

pub struct BallPosition {
    ground_frames: GroundFrames,
    ball_position: TopicObservation<Option<types::ball_position::BallPosition<Ground>>>,
    team_ball: TopicObservation<Option<types::ball_position::BallPosition<Field>>>,
}

impl Layer<Field> for BallPosition {
    const NAME: &'static str = "Ball Position";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let ground_frames = GroundFrames::new(&backend);

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
            ground_frames,
            ball_position,
            team_ball,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let alignment = self.ground_frames.alignment();
        for ball_position_sample in self.ball_position.get_all() {
            let SampleRecord {
                value: Some(ball),
                source_time,
                ..
            } = *ball_position_sample
            else {
                continue;
            };

            if let Some(ground_to_field) = alignment.ground_to_field_at(source_time) {
                painter.circle_filled(
                    ground_to_field * ball.position,
                    field_dimensions.ball_radius,
                    Color32::from_white_alpha(10),
                );
            }
        }

        if let Some(SampleRecord {
            value: Some(ball), ..
        }) = self.team_ball.latest().as_deref()
        {
            painter.ball(ball.position, field_dimensions.ball_radius, Color32::RED);
        }

        if let Some(SampleRecord {
            value: Some(ball),
            source_time,
            ..
        }) = self.ball_position.latest().as_deref()
            && let Some(ground_to_field) = alignment.ground_to_field_at(*source_time)
        {
            painter.ball(
                ground_to_field * ball.position,
                field_dimensions.ball_radius,
                Color32::WHITE,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
impl BallPosition {
    pub(in crate::panels::map) fn ground_frames(&self) -> &GroundFrames {
        &self.ground_frames
    }

    pub(in crate::panels::map) fn ball_is_known(&self) -> bool {
        self.ball_position.latest().is_some()
    }
}

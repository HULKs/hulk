use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use ball_filter::{BallFilter as BallFiltering, BallMode};
use coordinate_systems::Ground;
use linear_algebra::{Point, vector};
use ros_z_debug::TopicObservation;
use types::field_dimensions::FieldDimensions;

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct BallFilter {
    filter: TopicObservation<BallFiltering>,
}

impl Layer<Ground> for BallFilter {
    const NAME: &'static str = "Ball Filter";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let filter = backend
            .observer()
            .observe_typed("ball_filter/ball_filter_state")
            .expect("failed to construct ball filter state observer")
            .spawn();

        Self { filter }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(ros_z_debug::SampleRecord { value: filter, .. }) =
            self.filter.latest().as_deref()
        {
            for hypothesis in &filter.hypotheses {
                let stroke = Stroke::new(0.01_f32, Color32::BLACK);
                match hypothesis.mode {
                    BallMode::Resting(resting) => {
                        let position = Point::from(resting.mean.xy());
                        let covariance = resting.covariance.fixed_view::<2, 2>(0, 0).into_owned();
                        let yellow = Color32::from_rgba_unmultiplied(255, 255, 0, 100);
                        painter.covariance(position, covariance, stroke, yellow);
                        painter.target(position, 0.02, stroke, yellow);
                    }
                    BallMode::Moving(moving) => {
                        let position = Point::from(moving.mean.xy());
                        let covariance = moving.covariance.fixed_view::<2, 2>(0, 0).into_owned();
                        let pink = Color32::from_rgba_unmultiplied(255, 102, 204, 100);
                        painter.covariance(position, covariance, stroke, pink);
                        painter.target(position, 0.02, stroke, pink);

                        let velocity = vector![moving.mean.z, moving.mean.w];
                        let velocity_target = position + velocity;
                        painter.line_segment(position, velocity_target, stroke)
                    }
                }
            }
        }

        Ok(())
    }
}

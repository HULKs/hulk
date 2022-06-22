use std::time::{Duration, SystemTime};

use macros::{module, require_some};
use nalgebra::{Isometry2, Point2, Vector2};

use crate::{
    control::filtering::LowPassFilter,
    types::{Ball, BallPosition, SensorData},
};

pub struct BallFilter {
    ball_position: LowPassFilter<Vector2<f32>>,
    last_seen: Option<SystemTime>,
}

#[module(control)]
#[parameter(path = control.ball_filter.last_seen_timeout, data_type = Duration)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = current_odometry_to_last_odometry, data_type = Isometry2<f32>)]
#[perception_input(name = balls_top, path = balls, data_type = Vec<Ball>, cycler = vision_top)]
#[perception_input(name = balls_bottom, path = balls, data_type = Vec<Ball>, cycler = vision_bottom)]
#[main_output(name = ball_position, data_type = BallPosition )]
impl BallFilter {}

impl BallFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            ball_position: LowPassFilter::with_alpha(Vector2::zeros(), 0.8),
            last_seen: None,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let current_odometry_to_last_odometry =
            require_some!(context.current_odometry_to_last_odometry);

        let balls = context
            .balls_top
            .persistent
            .iter()
            .zip(context.balls_bottom.persistent.values());
        for ((&detection_time, balls_top), balls_bottom) in balls {
            let balls_in_control_cycle = balls_top
                .iter()
                .chain(balls_bottom.iter())
                .filter_map(|&data| data.as_ref());
            // predict
            // this is knowingly using the odometry of the current cycle representatively for all
            // cycles. Fix after GORE
            let ball_of_last_cycle = self.ball_position.state();
            self.ball_position
                .reset(current_odometry_to_last_odometry.inverse() * ball_of_last_cycle);

            for balls in balls_in_control_cycle {
                for ball in balls {
                    self.ball_position.update(ball.position.coords);
                    self.last_seen = Some(detection_time);
                }
            }
        }
        let ball_position = match self.last_seen {
            Some(last_seen)
                if cycle_start_time
                    .duration_since(last_seen)
                    .expect("Clock may have gone backwards")
                    < *context.last_seen_timeout =>
            {
                Some(BallPosition {
                    position: Some(Point2::from(self.ball_position.state())),
                    last_seen,
                })
            }
            _ => Some(BallPosition {
                position: None,
                last_seen: cycle_start_time,
            }),
        };
        Ok(MainOutputs { ball_position })
    }
}

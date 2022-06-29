use std::time::{Duration, SystemTime};

use macros::{module, require_some};
use nalgebra::{
    matrix, vector, Isometry2, Matrix2, Matrix2x4, Matrix4, Matrix4x2, Point2, Vector2, Vector4,
};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::{
    control::filtering::KalmanFilter,
    types::{Ball, BallPosition, SensorData},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallFilterHypothesis {
    filter: KalmanFilter<4>,
    validity: f32,
    last_update: SystemTime,
}

pub struct BallFilter {
    hypotheses: Vec<BallFilterHypothesis>,
}

#[module(control)]
#[parameter(path = control.ball_filter.hypothesis_timeout, data_type = Duration)]
#[parameter(path = control.ball_filter.measurement_matching_distance, data_type = f32)]
#[parameter(path = control.ball_filter.hypothesis_merge_distance, data_type = f32)]
#[parameter(path = control.ball_filter.process_noise, data_type = Vector4<f32>)]
#[parameter(path = control.ball_filter.measurement_noise, data_type = Vector2<f32>)]
#[parameter(path = control.ball_filter.initial_covariance, data_type = Vector4<f32>)]
#[parameter(path = control.ball_filter.validity_exponential_decay_factor, data_type = f32)]
#[input(path = sensor_data, data_type = SensorData)]
#[historic_input(path = current_odometry_to_last_odometry, data_type = Isometry2<f32>)]
#[perception_input(name = balls_top, path = balls, data_type = Vec<Ball>, cycler = vision_top)]
#[perception_input(name = balls_bottom, path = balls, data_type = Vec<Ball>, cycler = vision_bottom)]
#[additional_output(path = ball_filter_hypotheses, data_type = Vec<BallFilterHypothesis>)]
#[main_output(name = ball_position, data_type = BallPosition )]
impl BallFilter {}

impl BallFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            hypotheses: Vec::new(),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let measured_balls = context
            .balls_top
            .persistent
            .iter()
            .zip(context.balls_bottom.persistent.values());
        for ((&detection_time, balls_top), balls_bottom) in measured_balls {
            let current_odometry_to_last_odometry =
                match context.current_odometry_to_last_odometry.historic.get(&detection_time){
                    Some(&current_odometry_to_last_odometry) => current_odometry_to_last_odometry,
                    None => panic!("Failed to get matching current_odometry_to_last_odometry from ball detection time"),
                };
            let measured_balls_in_control_cycle = balls_top
                .iter()
                .chain(balls_bottom.iter())
                .filter_map(|&data| data.as_ref());
            if let Some(current_odometry_to_last_odometry) = current_odometry_to_last_odometry {
                self.predict_hypotheses_with_odometry(
                    current_odometry_to_last_odometry.inverse(),
                    Matrix4::from_diagonal(context.process_noise),
                    *context.validity_exponential_decay_factor,
                );

                for balls in measured_balls_in_control_cycle {
                    for ball in balls {
                        self.update_hypotheses_with_measurement(
                            ball.position,
                            detection_time,
                            *context.measurement_matching_distance,
                            Matrix2::from_diagonal(context.measurement_noise),
                            Matrix4::from_diagonal(context.initial_covariance),
                        );
                    }
                }
            }
        }

        self.remove_hypotheses(
            cycle_start_time,
            *context.hypothesis_timeout,
            *context.hypothesis_merge_distance,
        );

        let best_hypothesis = self.find_best_hypothesis();
        let ball_position = best_hypothesis.map(|hypothesis| BallPosition {
            position: Point2::from(hypothesis.filter.state().xy()),
            last_seen: hypothesis.last_update,
        });
        context
            .ball_filter_hypotheses
            .fill_on_subscription(|| self.hypotheses.clone());
        Ok(MainOutputs { ball_position })
    }

    fn predict_hypotheses_with_odometry(
        &mut self,
        last_odometry_to_current_odometry: Isometry2<f32>,
        process_noise: Matrix4<f32>,
        validity_exponential_decay_factor: f32,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            hypothesis.validity *= validity_exponential_decay_factor;
            let cycle_time = 0.012;
            let constant_velocity_prediction = matrix![
                1.0, 0.0, cycle_time, 0.0;
                0.0, 1.0, 0.0, cycle_time;
                0.0, 0.0, 1.0, 0.0;
                0.0, 0.0, 0.0, 1.0;
            ];
            let rotation = last_odometry_to_current_odometry
                .rotation
                .to_rotation_matrix();
            let state_rotation = matrix![
                rotation[(0, 0)], rotation[(0, 1)], 0.0, 0.0;
                rotation[(1, 0)], rotation[(1, 1)], 0.0, 0.0;
                0.0, 0.0, rotation[(0, 0)], rotation[(0, 1)];
                0.0, 0.0, rotation[(1, 0)], rotation[(1, 1)];
            ];
            let state_prediction = constant_velocity_prediction * state_rotation;
            let control_input_model = Matrix4x2::identity();
            let odometry_translation = last_odometry_to_current_odometry.translation.vector;
            hypothesis.filter.predict(
                state_prediction,
                control_input_model,
                odometry_translation,
                process_noise,
            )
        }
    }

    fn update_hypotheses_with_measurement(
        &mut self,
        detected_position: Point2<f32>,
        detection_time: SystemTime,
        matching_distance: f32,
        measurement_noise: Matrix2<f32>,
        initial_covariance: Matrix4<f32>,
    ) {
        let mut matching_hypotheses = self
            .hypotheses
            .iter_mut()
            .filter(|hypothesis| {
                (hypothesis.filter.state().xy() - detected_position.coords).norm()
                    < matching_distance
            })
            .peekable();
        if matching_hypotheses.peek().is_none() {
            self.spawn_hypothesis(detected_position, detection_time, initial_covariance);
            return;
        }
        matching_hypotheses.for_each(|hypothesis| {
            hypothesis.filter.update(
                Matrix2x4::identity(),
                detected_position.coords,
                measurement_noise * detected_position.coords.norm_squared(),
            );
            hypothesis.validity += 1.0;
            hypothesis.last_update = detection_time;
        });
    }

    fn find_best_hypothesis(&self) -> Option<&BallFilterHypothesis> {
        self.hypotheses.iter().max_by_key(|hypothesis| {
            NotNan::new(hypothesis.validity).expect("Ball Hypothesis validity is NaN")
        })
    }

    fn spawn_hypothesis(
        &mut self,
        detected_position: Point2<f32>,
        detection_time: SystemTime,
        initial_covariance: Matrix4<f32>,
    ) {
        let initial_state = vector![
            detected_position.coords.x,
            detected_position.coords.y,
            0.0,
            0.0
        ];
        let new_hypothesis = BallFilterHypothesis {
            filter: KalmanFilter::new(initial_state, initial_covariance),
            validity: 1.0,
            last_update: detection_time,
        };
        self.hypotheses.push(new_hypothesis);
    }

    fn remove_hypotheses(
        &mut self,
        now: SystemTime,
        hypothesis_timeout: Duration,
        merge_distance: f32,
    ) {
        self.hypotheses.retain(|hypothesis| {
            now.duration_since(hypothesis.last_update)
                .expect("Time has run backwards")
                < hypothesis_timeout
        });
        let mut deduplicated_hypotheses = Vec::<BallFilterHypothesis>::new();
        for hypothesis in self.hypotheses.drain(..) {
            let hypothesis_in_merge_distance =
                deduplicated_hypotheses
                    .iter_mut()
                    .find(|existing_hypothesis| {
                        (existing_hypothesis.filter.state().xy() - hypothesis.filter.state().xy())
                            .norm()
                            < merge_distance
                    });
            match hypothesis_in_merge_distance {
                Some(existing_hypothesis) => {
                    existing_hypothesis.filter.update(
                        Matrix4::identity(),
                        hypothesis.filter.state(),
                        hypothesis.filter.covariance(),
                    );
                }
                None => deduplicated_hypotheses.push(hypothesis),
            }
        }
        self.hypotheses = deduplicated_hypotheses;
    }
}

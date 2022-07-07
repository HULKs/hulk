use std::time::{Duration, SystemTime};

use module_derive::{module, require_some};
use nalgebra::{Isometry2, Matrix2, Point2, Vector2};
use serde::{Deserialize, Serialize};
use types::{DetectedRobots, RobotPosition, SensorData};

use crate::control::filtering::KalmanFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotFilterHypothesis {
    filter: KalmanFilter<2>,
    measurement_count: usize,
    last_update: SystemTime,
}

pub struct RobotFilter {
    hypotheses: Vec<RobotFilterHypothesis>,
}

#[module(control)]
#[parameter(path = control.robot_filter.hypothesis_timeout, data_type = Duration)]
#[parameter(path = control.robot_filter.measurement_matching_distance, data_type = f32)]
#[parameter(path = control.robot_filter.hypothesis_merge_distance, data_type = f32)]
#[parameter(path = control.robot_filter.process_noise, data_type = Vector2<f32>)]
#[parameter(path = control.robot_filter.measurement_noise, data_type = Vector2<f32>)]
#[parameter(path = control.robot_filter.initial_covariance, data_type = Vector2<f32>)]
#[parameter(path = control.robot_filter.measurement_count_threshold, data_type = usize)]
#[input(path = sensor_data, data_type = SensorData)]
#[historic_input(path = current_odometry_to_last_odometry, data_type = Isometry2<f32>)]
#[perception_input(name = detected_robots_top, path = detected_robots, data_type = DetectedRobots, cycler = vision_top)]
#[perception_input(name = detected_robots_bottom, path = detected_robots, data_type = DetectedRobots, cycler = vision_bottom)]
#[additional_output(path = robot_filter_hypotheses, data_type = Vec<RobotFilterHypothesis>)]
#[main_output(name = robot_positions, data_type = Vec<RobotPosition> )]
impl RobotFilter {}

impl RobotFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            hypotheses: Vec::new(),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let measured_robots = context
            .detected_robots_top
            .persistent
            .iter()
            .zip(context.detected_robots_bottom.persistent.values());
        for ((&detection_time, robots_top), robots_bottom) in measured_robots {
            let current_odometry_to_last_odometry =
                context.current_odometry_to_last_odometry.historic.get(&detection_time).expect("Failed to get matching current_odometry_to_last_odometry from ball detection time").expect("current_odometry_to_last_odometry should not be None");
            let measured_robots_in_control_cycle = robots_top
                .iter()
                .chain(robots_bottom.iter())
                .filter_map(|data| data.as_ref());
            self.predict_hypotheses_with_odometry(
                current_odometry_to_last_odometry.inverse(),
                Matrix2::from_diagonal(context.process_noise),
            );

            for obstacles in measured_robots_in_control_cycle {
                for obstacle in obstacles.robot_positions.iter() {
                    self.update_hypotheses_with_measurement(
                        obstacle.center,
                        detection_time,
                        *context.measurement_matching_distance,
                        Matrix2::from_diagonal(context.measurement_noise),
                        Matrix2::from_diagonal(context.initial_covariance),
                    );
                }
            }
        }

        self.remove_hypotheses(
            cycle_start_time,
            *context.hypothesis_timeout,
            *context.hypothesis_merge_distance,
        );

        let obstacle_positions = self
            .hypotheses
            .iter()
            .filter(|hypothesis| {
                hypothesis.measurement_count > *context.measurement_count_threshold
            })
            .map(|hypothesis| RobotPosition {
                position: hypothesis.filter.state().into(),
                last_seen: hypothesis.last_update,
            })
            .collect();
        context
            .robot_filter_hypotheses
            .fill_on_subscription(|| self.hypotheses.clone());
        Ok(MainOutputs {
            robot_positions: Some(obstacle_positions),
        })
    }

    fn predict_hypotheses_with_odometry(
        &mut self,
        last_odometry_to_current_odometry: Isometry2<f32>,
        process_noise: Matrix2<f32>,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            let state_prediction = last_odometry_to_current_odometry
                .rotation
                .to_rotation_matrix();
            let control_input_model = Matrix2::identity();
            let odometry_translation = last_odometry_to_current_odometry.translation.vector;
            hypothesis.filter.predict(
                *state_prediction.matrix(),
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
        initial_covariance: Matrix2<f32>,
    ) {
        let mut matching_hypotheses = self
            .hypotheses
            .iter_mut()
            .filter(|hypothesis| {
                (hypothesis.filter.state() - detected_position.coords).norm() < matching_distance
            })
            .peekable();
        if matching_hypotheses.peek().is_none() {
            self.spawn_hypothesis(detected_position, detection_time, initial_covariance);
            return;
        }
        matching_hypotheses.for_each(|hypothesis| {
            hypothesis.filter.update(
                Matrix2::identity(),
                detected_position.coords,
                measurement_noise * detected_position.coords.norm_squared(),
            );
            hypothesis.measurement_count += 1;
            hypothesis.last_update = detection_time;
        });
    }

    fn spawn_hypothesis(
        &mut self,
        detected_position: Point2<f32>,
        detection_time: SystemTime,
        initial_covariance: Matrix2<f32>,
    ) {
        let initial_state = detected_position.coords;
        let new_hypothesis = RobotFilterHypothesis {
            filter: KalmanFilter::new(initial_state, initial_covariance),
            measurement_count: 1,
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
        let mut deduplicated_hypotheses = Vec::<RobotFilterHypothesis>::new();
        for hypothesis in self.hypotheses.drain(..) {
            let hypothesis_in_merge_distance =
                deduplicated_hypotheses
                    .iter_mut()
                    .find(|existing_hypothesis| {
                        (existing_hypothesis.filter.state() - hypothesis.filter.state()).norm()
                            < merge_distance
                    });
            match hypothesis_in_merge_distance {
                Some(existing_hypothesis) => {
                    existing_hypothesis.filter.update(
                        Matrix2::identity(),
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

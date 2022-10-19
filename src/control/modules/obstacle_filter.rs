use std::time::{Duration, SystemTime};

use itertools::{chain, iproduct};
use module_derive::{module, require_some};
use nalgebra::{distance, point, Isometry2, Matrix2, Point2};
use serde::{Deserialize, Serialize};
use types::{DetectedRobots, FieldDimensions, Obstacle, ObstacleKind, SensorData, SonarObstacle};

use crate::control::filtering::KalmanFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleFilterHypothesis {
    filter: KalmanFilter<2>,
    measurement_count: usize,
    last_update: SystemTime,
    obstacle_kind: ObstacleKind,
}

pub struct ObstacleFilter {
    hypotheses: Vec<ObstacleFilterHypothesis>,
}

#[module(control)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(path = control.obstacle_filter, data_type = crate::framework::configuration::ObstacleFilter)]
#[parameter(path = control.obstacle_filter.robot_obstacle_radius_at_hip_height, data_type = f32)]
#[parameter(path = control.obstacle_filter.robot_obstacle_radius_at_foot_height, data_type = f32)]
#[parameter(path = control.obstacle_filter.unknown_obstacle_radius, data_type = f32)]
#[parameter(path = control.obstacle_filter.goal_post_obstacle_radius, data_type = f32)]
#[input(path = sensor_data, data_type = SensorData)]
#[historic_input(path = network_robot_obstacles, data_type = Vec<Point2<f32>>)]
#[historic_input(path = sonar_obstacles, data_type = Vec<SonarObstacle>)]
#[historic_input(path = robot_to_field, data_type = Isometry2<f32>)]
#[historic_input(path = current_odometry_to_last_odometry, data_type = Isometry2<f32>)]
#[perception_input(name = detected_robots_top, path = detected_robots, data_type = DetectedRobots, cycler = vision_top)]
#[perception_input(name = detected_robots_bottom, path = detected_robots, data_type = DetectedRobots, cycler = vision_bottom)]
#[additional_output(path = obstacle_filter_hypotheses, data_type = Vec<ObstacleFilterHypothesis>)]
#[main_output(name = obstacles, data_type = Vec<Obstacle> )]
impl ObstacleFilter {}

impl ObstacleFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            hypotheses: Vec::new(),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let field_dimensions = context.field_dimensions;
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let measured_robots = context
            .detected_robots_top
            .persistent
            .iter()
            .zip(context.detected_robots_bottom.persistent.values());
        for ((&detection_time, robots_top), robots_bottom) in measured_robots {
            let current_odometry_to_last_odometry = context
                .current_odometry_to_last_odometry
                .get(detection_time)
                .expect("current_odometry_to_last_odometry should not be None");

            self.predict_hypotheses_with_odometry(
                current_odometry_to_last_odometry.inverse(),
                Matrix2::from_diagonal(&context.obstacle_filter.process_noise),
            );

            let network_robot_obstacles = context
                .network_robot_obstacles
                .get(detection_time)
                .as_ref()
                .expect("network_robot_obstacles should not be None");
            let current_robot_to_field = context.robot_to_field.get(detection_time);
            let goal_posts =
                calculate_goal_post_positions(current_robot_to_field, field_dimensions);

            for network_robot_obstacle in network_robot_obstacles {
                self.update_hypotheses_with_measurement(
                    *network_robot_obstacle,
                    ObstacleKind::Robot,
                    detection_time,
                    context
                        .obstacle_filter
                        .network_robot_measurement_matching_distance,
                    Matrix2::from_diagonal(
                        &context.obstacle_filter.network_robot_measurement_noise,
                    ),
                );
            }

            if context.obstacle_filter.use_robot_detection_measurements {
                let measured_robots_in_control_cycle = robots_top
                    .iter()
                    .chain(robots_bottom.iter())
                    .filter_map(|data| data.as_ref());

                for obstacles in measured_robots_in_control_cycle {
                    for obstacle in obstacles.robot_positions.iter() {
                        self.update_hypotheses_with_measurement(
                            obstacle.center,
                            ObstacleKind::Robot,
                            detection_time,
                            context
                                .obstacle_filter
                                .robot_detection_measurement_matching_distance,
                            Matrix2::from_diagonal(
                                &context.obstacle_filter.robot_measurement_noise,
                            ),
                        );
                    }
                }
            }

            if let Some(sonar_obstacles) = context.sonar_obstacles.get(detection_time) {
                for sonar_obstacle in sonar_obstacles.iter() {
                    // TODO: Use a clever more intelligent metric

                    if context.obstacle_filter.use_sonar_measurements
                        && goal_posts.clone().into_iter().all(|goal_post| {
                            distance(&goal_post, &sonar_obstacle.position_in_robot)
                                > context
                                    .obstacle_filter
                                    .goal_post_measurement_matching_distance
                        })
                    {
                        self.update_hypotheses_with_measurement(
                            sonar_obstacle.position_in_robot,
                            ObstacleKind::Unknown,
                            detection_time,
                            context.obstacle_filter.sonar_goal_post_matching_distance,
                            Matrix2::from_diagonal(
                                &context.obstacle_filter.sonar_measurement_noise,
                            ),
                        );
                    }
                }
            }
        }

        self.remove_hypotheses(
            cycle_start_time,
            context.obstacle_filter.hypothesis_timeout,
            context.obstacle_filter.hypothesis_merge_distance,
        );

        let robot_obstacles = self
            .hypotheses
            .iter()
            .filter(|hypothesis| {
                hypothesis.measurement_count > context.obstacle_filter.measurement_count_threshold
            })
            .map(|hypothesis| {
                let (radius_at_hip_height, radius_at_foot_height) = match hypothesis.obstacle_kind {
                    ObstacleKind::GoalPost => (
                        *context.goal_post_obstacle_radius,
                        *context.goal_post_obstacle_radius,
                    ),
                    ObstacleKind::Robot => (
                        *context.robot_obstacle_radius_at_hip_height,
                        *context.robot_obstacle_radius_at_foot_height,
                    ),
                    ObstacleKind::Unknown => (
                        *context.unknown_obstacle_radius,
                        *context.unknown_obstacle_radius,
                    ),
                    _ => panic!("Unexpected obstacle radius"),
                };
                Obstacle {
                    position: hypothesis.filter.state().into(),
                    kind: hypothesis.obstacle_kind,
                    radius_at_hip_height,
                    radius_at_foot_height,
                }
            })
            .collect::<Vec<_>>();
        let current_robot_to_field = context.robot_to_field.get(cycle_start_time);
        let goal_posts = calculate_goal_post_positions(current_robot_to_field, field_dimensions);
        let goal_post_obstacles = goal_posts.into_iter().map(|goal_post| {
            Obstacle::goal_post(goal_post, field_dimensions.goal_post_diameter / 2.0)
        });
        context
            .obstacle_filter_hypotheses
            .fill_on_subscription(|| self.hypotheses.clone());
        Ok(MainOutputs {
            obstacles: Some(chain!(robot_obstacles, goal_post_obstacles).collect()),
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
        detected_obstacle_kind: ObstacleKind,
        detection_time: SystemTime,
        matching_distance: f32,
        measurement_noise: Matrix2<f32>,
    ) {
        let mut matching_hypotheses = self
            .hypotheses
            .iter_mut()
            .filter(|hypothesis| {
                (hypothesis.filter.state() - detected_position.coords).norm() < matching_distance
            })
            .peekable();
        if matching_hypotheses.peek().is_none() {
            self.spawn_hypothesis(
                detected_position,
                detected_obstacle_kind,
                detection_time,
                measurement_noise,
            );
            return;
        }
        matching_hypotheses.for_each(|hypothesis| {
            hypothesis.filter.update(
                Matrix2::identity(),
                detected_position.coords,
                measurement_noise * detected_position.coords.norm_squared(),
            );
            hypothesis.obstacle_kind = match hypothesis.obstacle_kind {
                ObstacleKind::Robot => hypothesis.obstacle_kind,
                ObstacleKind::Unknown => detected_obstacle_kind,
                _ => panic!("Unexpected obstacle kind"),
            };
            hypothesis.measurement_count += 1;
            hypothesis.last_update = detection_time;
        });
    }

    fn spawn_hypothesis(
        &mut self,
        detected_position: Point2<f32>,
        obstacle_kind: ObstacleKind,
        detection_time: SystemTime,
        initial_covariance: Matrix2<f32>,
    ) {
        let initial_state = detected_position.coords;
        let new_hypothesis = ObstacleFilterHypothesis {
            filter: KalmanFilter::new(initial_state, initial_covariance),
            obstacle_kind,
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
        let mut deduplicated_hypotheses = Vec::<ObstacleFilterHypothesis>::new();
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
                    existing_hypothesis.obstacle_kind = match existing_hypothesis.obstacle_kind {
                        ObstacleKind::Robot => existing_hypothesis.obstacle_kind,
                        ObstacleKind::Unknown => hypothesis.obstacle_kind,
                        _ => panic!("Unexpected obstacle kind"),
                    };
                }
                None => deduplicated_hypotheses.push(hypothesis),
            }
        }
        self.hypotheses = deduplicated_hypotheses;
    }
}

fn calculate_goal_post_positions(
    current_robot_to_field: &Option<Isometry2<f32>>,
    field_dimensions: &FieldDimensions,
) -> Vec<Point2<f32>> {
    current_robot_to_field
        .map(|robot_to_field| {
            let field_to_robot = robot_to_field.inverse();
            iproduct!([-1.0, 1.0], [-1.0, 1.0]).map(move |(x_sign, y_sign)| {
                let radius = field_dimensions.goal_post_diameter / 2.0;
                let position_on_field = point![
                    x_sign
                        * (field_dimensions.length / 2.0
                            + field_dimensions.goal_post_diameter / 2.0
                            - field_dimensions.line_width / 2.0),
                    y_sign * (field_dimensions.goal_inner_width / 2.0 + radius)
                ];
                field_to_robot * position_on_field
            })
        })
        .into_iter()
        .flatten()
        .collect()
}

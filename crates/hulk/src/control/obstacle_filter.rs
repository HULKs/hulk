use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use filtering::KalmanFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use itertools::{chain, iproduct};
use nalgebra::{distance, point, Isometry2, Matrix2, Point2};
use types::{
    configuration::ObstacleFilter as ObstacleFilterConfiguration,
    obstacle_filter_hypothesis::ObstacleFilterHypothesisSnapshot, CycleTime, DetectedRobots,
    FieldDimensions, Obstacle, ObstacleKind, SensorData, SonarObstacle,
};

pub struct ObstacleFilter {
    hypotheses: Vec<ObstacleFilterHypothesis>,
}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub obstacle_filter_configuration: Parameter<ObstacleFilterConfiguration, "obstacle_filter">,
}

#[context]
pub struct CycleContext {
    pub obstacle_filter_hypotheses:
        AdditionalOutput<Vec<ObstacleFilterHypothesisSnapshot>, "obstacle_filter_hypotheses">,

    pub current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    pub network_robot_obstacles: HistoricInput<Vec<Point2<f32>>, "network_robot_obstacles">,
    pub robot_to_field: HistoricInput<Option<Isometry2<f32>>, "robot_to_field?">,
    pub sonar_obstacles: HistoricInput<Vec<SonarObstacle>, "sonar_obstacles">,

    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,

    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub goal_post_obstacle_radius: Parameter<f32, "obstacle_filter.goal_post_obstacle_radius">,
    pub obstacle_filter_configuration: Parameter<ObstacleFilterConfiguration, "obstacle_filter">,
    pub robot_obstacle_radius_at_foot_height:
        Parameter<f32, "obstacle_filter.robot_obstacle_radius_at_foot_height">,
    pub robot_obstacle_radius_at_hip_height:
        Parameter<f32, "obstacle_filter.robot_obstacle_radius_at_hip_height">,
    pub unknown_obstacle_radius: Parameter<f32, "obstacle_filter.unknown_obstacle_radius">,

    pub detected_robots_bottom:
        PerceptionInput<Option<DetectedRobots>, "VisionBottom", "detected_robots?">,
    pub detected_robots_top:
        PerceptionInput<Option<DetectedRobots>, "VisionTop", "detected_robots?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub obstacles: MainOutput<Vec<Obstacle>>,
}

impl ObstacleFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            hypotheses: Vec::new(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let field_dimensions = context.field_dimensions;
        let cycle_start_time = context.cycle_time.start_time;
        let measured_robots = context
            .detected_robots_top
            .persistent
            .iter()
            .zip(context.detected_robots_bottom.persistent.values());
        for ((detection_time, robots_top), robots_bottom) in measured_robots {
            let current_odometry_to_last_odometry = context
                .current_odometry_to_last_odometry
                .get(detection_time)
                .expect("current_odometry_to_last_odometry should not be None");

            self.predict_hypotheses_with_odometry(
                current_odometry_to_last_odometry.inverse(),
                Matrix2::from_diagonal(&context.obstacle_filter_configuration.process_noise),
            );

            let network_robot_obstacles = context.network_robot_obstacles.get(detection_time);
            let current_robot_to_field = context.robot_to_field.get(detection_time);
            let goal_posts =
                calculate_goal_post_positions(current_robot_to_field, field_dimensions);

            for network_robot_obstacle in network_robot_obstacles {
                self.update_hypotheses_with_measurement(
                    *network_robot_obstacle,
                    ObstacleKind::Robot,
                    *detection_time,
                    context
                        .obstacle_filter_configuration
                        .network_robot_measurement_matching_distance,
                    Matrix2::from_diagonal(
                        &context
                            .obstacle_filter_configuration
                            .network_robot_measurement_noise,
                    ),
                );
            }

            if context
                .obstacle_filter_configuration
                .use_robot_detection_measurements
            {
                let measured_robots_in_control_cycle = robots_top
                    .iter()
                    .chain(robots_bottom.iter())
                    .filter_map(|data| data.as_ref());

                for obstacles in measured_robots_in_control_cycle {
                    for obstacle in obstacles.robot_positions.iter() {
                        self.update_hypotheses_with_measurement(
                            obstacle.center,
                            ObstacleKind::Robot,
                            *detection_time,
                            context
                                .obstacle_filter_configuration
                                .robot_detection_measurement_matching_distance,
                            Matrix2::from_diagonal(
                                &context
                                    .obstacle_filter_configuration
                                    .robot_measurement_noise,
                            ),
                        );
                    }
                }
            }

            for sonar_obstacle in context.sonar_obstacles.get(detection_time) {
                // TODO: Use a clever more intelligent metric

                if context.obstacle_filter_configuration.use_sonar_measurements
                    && goal_posts.clone().into_iter().all(|goal_post| {
                        distance(&goal_post, &sonar_obstacle.position_in_robot)
                            > context
                                .obstacle_filter_configuration
                                .goal_post_measurement_matching_distance
                    })
                {
                    self.update_hypotheses_with_measurement(
                        sonar_obstacle.position_in_robot,
                        ObstacleKind::Unknown,
                        *detection_time,
                        context
                            .obstacle_filter_configuration
                            .sonar_goal_post_matching_distance,
                        Matrix2::from_diagonal(
                            &context
                                .obstacle_filter_configuration
                                .sonar_measurement_noise,
                        ),
                    );
                }
            }
        }

        self.remove_hypotheses(
            cycle_start_time,
            context.obstacle_filter_configuration.hypothesis_timeout,
            context
                .obstacle_filter_configuration
                .hypothesis_merge_distance,
        );

        let robot_obstacles = self
            .hypotheses
            .iter()
            .filter(|hypothesis| {
                hypothesis.measurement_count
                    > context
                        .obstacle_filter_configuration
                        .measurement_count_threshold
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
        let current_robot_to_field = context.robot_to_field.get(&cycle_start_time);
        let goal_posts = calculate_goal_post_positions(current_robot_to_field, field_dimensions);
        let goal_post_obstacles = goal_posts.into_iter().map(|goal_post| {
            Obstacle::goal_post(goal_post, field_dimensions.goal_post_diameter / 2.0)
        });
        context
            .obstacle_filter_hypotheses
            .fill_if_subscribed(|| self.hypotheses.iter().map(Into::into).collect());
        Ok(MainOutputs {
            obstacles: chain!(robot_obstacles, goal_post_obstacles)
                .collect::<Vec<_>>()
                .into(),
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

#[derive(Debug, Clone)]
struct ObstacleFilterHypothesis {
    filter: KalmanFilter<2>,
    measurement_count: usize,
    last_update: SystemTime,
    obstacle_kind: ObstacleKind,
}

impl From<&ObstacleFilterHypothesis> for ObstacleFilterHypothesisSnapshot {
    fn from(hypothesis: &ObstacleFilterHypothesis) -> Self {
        Self {
            filter: (&hypothesis.filter).into(),
            measurement_count: hypothesis.measurement_count,
            last_update: hypothesis.last_update,
            obstacle_kind: hypothesis.obstacle_kind,
        }
    }
}

fn calculate_goal_post_positions(
    current_robot_to_field: Option<&Isometry2<f32>>,
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

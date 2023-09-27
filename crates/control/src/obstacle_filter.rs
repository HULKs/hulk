use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use filtering::kalman_filter::KalmanFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use itertools::{chain, iproduct};
use nalgebra::{distance, point, Isometry2, Matrix2, Point2};
use types::{
    cycle_time::CycleTime,
    detected_feet::DetectedFeet,
    detected_robots::DetectedRobots,
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    obstacle_filter::Hypothesis,
    obstacles::{Obstacle, ObstacleKind},
    parameters::ObstacleFilterParameters,
    primary_state::PrimaryState,
    sonar_obstacle::SonarObstacle,
};

pub struct ObstacleFilter {
    hypotheses: Vec<Hypothesis>,
    last_primary_state: PrimaryState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    obstacle_filter_hypotheses: AdditionalOutput<Vec<Hypothesis>, "obstacle_filter_hypotheses">,

    current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    network_robot_obstacles: HistoricInput<Vec<Point2<f32>>, "network_robot_obstacles">,
    robot_to_field: HistoricInput<Option<Isometry2<f32>>, "robot_to_field?">,
    sonar_obstacles: HistoricInput<Vec<SonarObstacle>, "sonar_obstacles">,

    cycle_time: Input<CycleTime, "cycle_time">,
    primary_state: Input<PrimaryState, "primary_state">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    goal_post_obstacle_radius: Parameter<f32, "obstacle_filter.goal_post_obstacle_radius">,
    obstacle_filter_parameters: Parameter<ObstacleFilterParameters, "obstacle_filter">,
    robot_obstacle_radius_at_foot_height:
        Parameter<f32, "obstacle_filter.robot_obstacle_radius_at_foot_height">,
    robot_obstacle_radius_at_hip_height:
        Parameter<f32, "obstacle_filter.robot_obstacle_radius_at_hip_height">,
    unknown_obstacle_radius: Parameter<f32, "obstacle_filter.unknown_obstacle_radius">,

    detected_feet_bottom: PerceptionInput<DetectedFeet, "VisionBottom", "detected_feet">,
    detected_feet_top: PerceptionInput<DetectedFeet, "VisionTop", "detected_feet">,
    detected_robots_bottom: PerceptionInput<DetectedRobots, "VisionBottom", "detected_robots">,
    detected_robots_top: PerceptionInput<DetectedRobots, "VisionTop", "detected_robots">,
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
            last_primary_state: PrimaryState::Unstiff,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let field_dimensions = context.field_dimensions;
        let cycle_start_time = context.cycle_time.start_time;
        let measurements = context
            .detected_feet_top
            .persistent
            .iter()
            .zip(context.detected_feet_bottom.persistent.values())
            .zip(context.detected_robots_top.persistent.values())
            .zip(context.detected_robots_bottom.persistent.values());
        for ((((detection_time, feet_top), feet_bottom), robots_top), robots_bottom) in measurements
        {
            let current_odometry_to_last_odometry = context
                .current_odometry_to_last_odometry
                .get(detection_time)
                .expect("current_odometry_to_last_odometry should not be None");

            self.predict_hypotheses_with_odometry(
                current_odometry_to_last_odometry.inverse(),
                Matrix2::from_diagonal(&context.obstacle_filter_parameters.process_noise),
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
                        .obstacle_filter_parameters
                        .network_robot_measurement_matching_distance,
                    Matrix2::from_diagonal(
                        &context
                            .obstacle_filter_parameters
                            .network_robot_measurement_noise,
                    ),
                );
            }

            if context
                .obstacle_filter_parameters
                .use_feet_detection_measurements
            {
                let measured_positions_in_control_cycle = feet_top
                    .iter()
                    .chain(feet_bottom.iter())
                    .flat_map(|obstacles| obstacles.positions.iter());

                for position in measured_positions_in_control_cycle {
                    self.update_hypotheses_with_measurement(
                        *position,
                        ObstacleKind::Robot,
                        *detection_time,
                        context
                            .obstacle_filter_parameters
                            .feet_detection_measurement_matching_distance,
                        Matrix2::from_diagonal(
                            &context.obstacle_filter_parameters.feet_measurement_noise,
                        ),
                    );
                }
            }

            if context
                .obstacle_filter_parameters
                .use_robot_detection_measurements
            {
                let measured_positions_in_control_cycle = robots_top
                    .iter()
                    .chain(robots_bottom.iter())
                    .flat_map(|obstacles| obstacles.on_ground.iter());

                for position in measured_positions_in_control_cycle {
                    self.update_hypotheses_with_measurement(
                        *position,
                        ObstacleKind::Robot,
                        *detection_time,
                        context
                            .obstacle_filter_parameters
                            .robot_detection_measurement_matching_distance,
                        Matrix2::from_diagonal(
                            &context.obstacle_filter_parameters.robot_measurement_noise,
                        ),
                    );
                }
            }

            for sonar_obstacle in context.sonar_obstacles.get(detection_time) {
                // TODO: Use a clever more intelligent metric

                if context.obstacle_filter_parameters.use_sonar_measurements
                    && goal_posts.clone().into_iter().all(|goal_post| {
                        distance(&goal_post, &sonar_obstacle.position_in_robot)
                            > context
                                .obstacle_filter_parameters
                                .goal_post_measurement_matching_distance
                    })
                {
                    self.update_hypotheses_with_measurement(
                        sonar_obstacle.position_in_robot,
                        ObstacleKind::Unknown,
                        *detection_time,
                        context
                            .obstacle_filter_parameters
                            .sonar_goal_post_matching_distance,
                        Matrix2::from_diagonal(
                            &context.obstacle_filter_parameters.sonar_measurement_noise,
                        ),
                    );
                }
            }
        }

        self.remove_hypotheses(
            cycle_start_time,
            context.obstacle_filter_parameters.hypothesis_timeout,
            context.obstacle_filter_parameters.hypothesis_merge_distance,
        );

        if self.last_primary_state == PrimaryState::Penalized
            && *context.primary_state != PrimaryState::Penalized
        {
            self.hypotheses = Vec::new();
        }
        self.last_primary_state = *context.primary_state;

        let robot_obstacles = self
            .hypotheses
            .iter()
            .filter(|hypothesis| {
                hypothesis.measurement_count
                    > context
                        .obstacle_filter_parameters
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
                    position: hypothesis.state.mean.into(),
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
            .fill_if_subscribed(|| self.hypotheses.clone());
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
            hypothesis.state.predict(
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
                (hypothesis.state.mean - detected_position.coords).norm() < matching_distance
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
            hypothesis.state.update(
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
        let new_hypothesis = Hypothesis {
            state: MultivariateNormalDistribution {
                mean: initial_state,
                covariance: initial_covariance,
            },
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
        let mut deduplicated_hypotheses = Vec::<Hypothesis>::new();
        for hypothesis in self.hypotheses.drain(..) {
            let hypothesis_in_merge_distance =
                deduplicated_hypotheses
                    .iter_mut()
                    .find(|existing_hypothesis| {
                        (existing_hypothesis.state.mean - hypothesis.state.mean).norm()
                            < merge_distance
                    });
            match hypothesis_in_merge_distance {
                Some(existing_hypothesis) => {
                    existing_hypothesis.state.update(
                        Matrix2::identity(),
                        hypothesis.state.mean,
                        hypothesis.state.covariance,
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

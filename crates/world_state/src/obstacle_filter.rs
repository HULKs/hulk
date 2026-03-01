use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use filtering::kalman_filter::KalmanFilter;
use framework::{AdditionalOutput, MainOutput};
use itertools::{chain, iproduct};
use linear_algebra::{point, IntoFramed, Isometry2, Point2};
use nalgebra::Matrix2;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    fall_state::FallState,
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    obstacle_filter::Hypothesis,
    obstacles::{Obstacle, ObstacleKind},
    parameters::ObstacleFilterParameters,
    primary_state::PrimaryState,
};

#[derive(PartialEq)]
enum MeasurementKind {
    #[expect(unused)] // to be removed once we have working obstacle detection
    Own,
    NetworkRobot,
}

#[derive(Deserialize, Serialize)]
pub struct ObstacleFilter {
    hypotheses: Vec<Hypothesis>,
    last_primary_state: PrimaryState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    obstacle_filter_hypotheses: AdditionalOutput<Vec<Hypothesis>, "obstacle_filter_hypotheses">,

    network_robot_obstacles: Input<Vec<Point2<Ground>>, "network_robot_obstacles">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    current_odometry_to_last_odometry:
        Input<Option<nalgebra::Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    primary_state: Input<PrimaryState, "primary_state">,
    fall_state: Input<FallState, "fall_state">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    goal_post_obstacle_radius: Parameter<f32, "obstacle_filter.goal_post_obstacle_radius">,
    obstacle_filter_parameters: Parameter<ObstacleFilterParameters, "obstacle_filter">,
    robot_obstacle_radius_at_foot_height:
        Parameter<f32, "obstacle_filter.robot_obstacle_radius_at_foot_height">,
    robot_obstacle_radius_at_hip_height:
        Parameter<f32, "obstacle_filter.robot_obstacle_radius_at_hip_height">,
    unknown_obstacle_radius: Parameter<f32, "obstacle_filter.unknown_obstacle_radius">,
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
            last_primary_state: PrimaryState::Safe,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let field_dimensions = context.field_dimensions;
        let cycle_start_time = context.cycle_time.start_time;

        let current_odometry_to_last_odometry = context
            .current_odometry_to_last_odometry
            .copied()
            .unwrap_or_default();

        self.predict_hypotheses_with_odometry(
            current_odometry_to_last_odometry.inverse(),
            Matrix2::from_diagonal(&context.obstacle_filter_parameters.process_noise),
        );

        for network_robot_obstacle in context.network_robot_obstacles {
            self.update_hypotheses_with_measurement(
                *network_robot_obstacle,
                ObstacleKind::Robot,
                cycle_start_time,
                context
                    .obstacle_filter_parameters
                    .network_robot_measurement_matching_distance,
                Matrix2::from_diagonal(
                    &context
                        .obstacle_filter_parameters
                        .network_robot_measurement_noise,
                ),
                MeasurementKind::NetworkRobot,
            );
        }

        self.remove_hypotheses(
            cycle_start_time,
            context.obstacle_filter_parameters.hypothesis_timeout,
            context.obstacle_filter_parameters.hypothesis_merge_distance,
        );

        let became_unpenalized = self.last_primary_state == PrimaryState::Penalized
            && *context.primary_state != PrimaryState::Penalized;
        let is_upright = *context.fall_state == FallState::Upright;
        self.last_primary_state = *context.primary_state;

        if became_unpenalized {
            self.hypotheses.clear();
        }
        if !is_upright {
            self.hypotheses
                .retain(|obstacle| obstacle.obstacle_kind != ObstacleKind::Unknown);
        }

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
                    position: hypothesis.state.mean.framed().as_point(),
                    kind: hypothesis.obstacle_kind,
                    radius_at_hip_height,
                    radius_at_foot_height,
                }
            })
            .collect::<Vec<_>>();

        let goal_posts = calculate_goal_post_positions(context.ground_to_field, field_dimensions);
        let goal_post_obstacles = goal_posts
            .into_iter()
            .map(|goal_post| Obstacle::goal_post(goal_post, *context.goal_post_obstacle_radius));
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
        last_odometry_to_current_odometry: nalgebra::Isometry2<f32>,
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
        detected_position: Point2<Ground>,
        detected_obstacle_kind: ObstacleKind,
        detection_time: SystemTime,
        matching_distance: f32,
        measurement_noise: Matrix2<f32>,
        kind: MeasurementKind,
    ) {
        let mut matching_hypotheses = self
            .hypotheses
            .iter_mut()
            .filter(|hypothesis| {
                (hypothesis.state.mean - detected_position.inner.coords).norm() < matching_distance
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
                detected_position.inner.coords,
                if kind == MeasurementKind::NetworkRobot {
                    measurement_noise
                } else {
                    measurement_noise * (detected_position.coords().norm_squared() + f32::EPSILON)
                },
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
        detected_position: Point2<Ground>,
        obstacle_kind: ObstacleKind,
        detection_time: SystemTime,
        initial_covariance: Matrix2<f32>,
    ) {
        let initial_state = detected_position.inner.coords;
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
    ground_to_field: Option<&Isometry2<Ground, Field>>,
    field_dimensions: &FieldDimensions,
) -> Vec<Point2<Ground>> {
    ground_to_field
        .map(|ground_to_field| {
            let field_to_robot = ground_to_field.inverse();
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

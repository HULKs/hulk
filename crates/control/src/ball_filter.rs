use std::{
    collections::BTreeMap,
    time::SystemTime,
};

use color_eyre::Result;
use hungarian_algorithm::AssignmentProblem;
use itertools::Itertools;
use nalgebra::{Matrix2, Matrix4};
use ndarray::Array2;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use ball_filter::{BallFilter as BallFiltering, BallHypothesis};
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use geometry::circle::Circle;
use linear_algebra::{distance, IntoTransform, Isometry2, Point2};
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix, Projection};
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    limb::{is_above_limbs, Limb, ProjectedLimbs},
    parameters::BallFilterParameters,
};
use walking_engine::mode::Mode;

#[derive(Deserialize, Serialize)]
pub struct BallFilter {
    ball_filter: BallFiltering,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    filter_state: AdditionalOutput<BallFiltering, "ball_filter_state">,
    best_ball_hypothesis: AdditionalOutput<Option<BallHypothesis>, "best_ball_hypothesis">,

    filtered_balls_in_image_bottom:
        AdditionalOutput<Vec<Circle<Pixel>>, "filtered_balls_in_image_bottom">,
    filtered_balls_in_image_top:
        AdditionalOutput<Vec<Circle<Pixel>>, "filtered_balls_in_image_top">,

    current_odometry_to_last_odometry:
        HistoricInput<Option<nalgebra::Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    historic_camera_matrices: HistoricInput<Option<CameraMatrices>, "camera_matrices?">,
    had_ground_contact: HistoricInput<bool, "has_ground_contact">,
    historic_cycle_times: HistoricInput<CycleTime, "cycle_time">,

    camera_matrices: Input<Option<CameraMatrices>, "camera_matrices?">,
    cycle_time: Input<CycleTime, "cycle_time">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    ball_filter_configuration: Parameter<BallFilterParameters, "ball_filter">,

    balls_bottom: PerceptionInput<Option<Vec<BallPercept>>, "VisionBottom", "balls?">,
    balls_top: PerceptionInput<Option<Vec<BallPercept>>, "VisionTop", "balls?">,
    projected_limbs: PerceptionInput<Option<ProjectedLimbs>, "VisionBottom", "projected_limbs?">,
    walking_engine_mode: CyclerState<Mode, "walking_engine_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_position: MainOutput<Option<BallPosition<Ground>>>,
    pub removed_ball_positions: MainOutput<Vec<Point2<Ground>>>,
    pub hypothetical_ball_positions: MainOutput<Vec<HypotheticalBallPosition<Ground>>>,
}

impl BallFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            ball_filter: Default::default(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn advance_all_hypotheses(
        &mut self,
        measurements: BTreeMap<SystemTime, Vec<&BallPercept>>,
        current_to_last_odometry: HistoricInput<Option<&nalgebra::Isometry2<f32>>>,
        camera_matrices: HistoricInput<Option<&CameraMatrices>>,
        had_ground_contact: HistoricInput<&bool>,
        historic_cycle_times: HistoricInput<&CycleTime>,
        projected_limbs: PerceptionInput<Vec<Option<&ProjectedLimbs>>>,
        filter_parameters: &BallFilterParameters,
        field_dimensions: &FieldDimensions,
        cycle_time: &CycleTime,
        walking_engine_mode: Mode,
    ) -> Vec<BallHypothesis> {
        for (detection_time, balls) in measurements {
            self.ball_filter.hypotheses.retain(|hypothesis| {
                hypothesis.validity > filter_parameters.validity_discard_threshold
            });

            let delta_time = historic_cycle_times
                .get(&detection_time)
                .last_cycle_duration;
            let current_to_last_odometry: Isometry2<Ground, Ground> = current_to_last_odometry
                .get(&detection_time)
                .copied()
                .unwrap_or_default()
                .framed_transform();

            self.ball_filter.predict(
                delta_time,
                current_to_last_odometry.inverse(),
                filter_parameters.velocity_decay_factor,
                Matrix4::from_diagonal(&filter_parameters.noise.process_noise_moving),
                Matrix2::from_diagonal(&filter_parameters.noise.process_noise_resting),
                filter_parameters.resting_ball_velocity_threshold,
            );

            if !had_ground_contact.get(&detection_time) {
                self.ball_filter.reset();
                continue;
            }
            let camera_matrices = camera_matrices.get(&detection_time);

            let projected_limbs_bottom = projected_limbs
                .persistent
                .get(&detection_time)
                .and_then(|limbs| limbs.last())
                .and_then(|limbs| *limbs);

            self.ball_filter.decay_hypotheses(|hypothesis| {
                decide_validity_decay_for_hypothesis(
                    hypothesis,
                    camera_matrices,
                    projected_limbs_bottom,
                    field_dimensions.ball_radius,
                    filter_parameters,
                )
            });

            let match_matrix =
                mahalanobis_matrix_of_hypotheses_and_percepts(&self.ball_filter.hypotheses, &balls);

            let assignment = AssignmentProblem::from_costs(match_matrix).solve();

            let mut used_percepts = vec![];

            for (hypothesis, assigned_percept) in self
                .ball_filter
                .hypotheses
                .iter_mut()
                .zip_eq(assignment.iter())
            {
                if let Some(assigned_percept) = assigned_percept {
                    let mahalanobis_distance = -assigned_percept.cost;
                    if mahalanobis_distance > filter_parameters.maximum_matching_cost {
                        continue;
                    }
                    let validity_increase = assigned_percept.cost.exp();
                    let percept = balls[assigned_percept.to];
                    used_percepts.push(assigned_percept.to);
                    hypothesis.update(detection_time, percept.percept_in_ground, validity_increase);
                }
            }

            let unused_percepts = {
                let mut all_percepts = balls.clone();
                used_percepts.sort_unstable();
                for index in used_percepts.into_iter().rev() {
                    all_percepts.remove(index);
                }
                all_percepts
            };

            for percept in unused_percepts {
                self.ball_filter.spawn(
                    detection_time,
                    percept.percept_in_ground,
                    Matrix4::from_diagonal(&filter_parameters.noise.initial_covariance),
                );
            }
        }

        let is_hypothesis_valid = |hypothesis: &BallHypothesis| {
            let ball = hypothesis.position();
            let duration_since_last_observation = cycle_time
                .start_time
                .duration_since(ball.last_seen)
                .expect("time ran backwards");
            let validity_high_enough =
                hypothesis.validity >= filter_parameters.validity_discard_threshold;
            let ball_kicked = matches!(walking_engine_mode, Mode::Kicking(_));
            is_ball_inside_field(ball, field_dimensions)
                && validity_high_enough
                && duration_since_last_observation < filter_parameters.hypothesis_timeout
                && !ball_kicked
        };

        let should_merge_hypotheses =
            |hypothesis1: &BallHypothesis, hypothesis2: &BallHypothesis| {
                let ball1 = hypothesis1.position();
                let ball2 = hypothesis2.position();

                distance(ball1.position, ball2.position)
                    < filter_parameters.hypothesis_merge_distance
            };

        self.ball_filter
            .remove_hypotheses(is_hypothesis_valid, should_merge_hypotheses)
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let persistent_updates = time_ordered_balls(
            context.balls_top.persistent,
            context.balls_bottom.persistent,
        );

        let filter_parameters = context.ball_filter_configuration;
        let removed_hypotheses = self.advance_all_hypotheses(
            persistent_updates,
            context.current_odometry_to_last_odometry,
            context.historic_camera_matrices,
            context.had_ground_contact,
            context.historic_cycle_times,
            context.projected_limbs,
            filter_parameters,
            context.field_dimensions,
            context.cycle_time,
            *context.walking_engine_mode,
        );

        let velocity_threshold = filter_parameters.resting_ball_velocity_threshold;

        context
            .filter_state
            .fill_if_subscribed(|| self.ball_filter.clone());

        let best_hypothesis = self
            .ball_filter
            .best_hypothesis(filter_parameters.validity_output_threshold);
        context
            .best_ball_hypothesis
            .fill_if_subscribed(|| best_hypothesis.cloned());

        let filtered_ball = best_hypothesis.map(|hypothesis| hypothesis.position());

        let output_balls: Vec<_> = self
            .ball_filter
            .hypotheses
            .iter()
            .filter_map(|hypothesis| {
                if hypothesis.validity >= filter_parameters.validity_output_threshold {
                    Some(hypothesis.position())
                } else {
                    None
                }
            })
            .collect();

        let ball_radius = context.field_dimensions.ball_radius;
        context.filtered_balls_in_image_top.fill_if_subscribed(|| {
            context.camera_matrices.map_or(vec![], |camera_matrices| {
                project_to_image(&output_balls, &camera_matrices.top, ball_radius)
            })
        });
        context
            .filtered_balls_in_image_bottom
            .fill_if_subscribed(|| {
                context.camera_matrices.map_or(vec![], |camera_matrices| {
                    project_to_image(&output_balls, &camera_matrices.bottom, ball_radius)
                })
            });

        let removed_ball_positions = removed_hypotheses
            .into_iter()
            .filter(|hypothesis| {
                hypothesis.validity >= context.ball_filter_configuration.validity_output_threshold
            })
            .map(|hypothesis| hypothesis.position().position)
            .collect::<Vec<_>>();

        Ok(MainOutputs {
            ball_position: filtered_ball.into(),
            removed_ball_positions: removed_ball_positions.into(),
            hypothetical_ball_positions: self
                .hypothetical_ball_positions(velocity_threshold)
                .into(),
        })
    }

    fn hypothetical_ball_positions(
        &self,
        validity_limit: f32,
    ) -> Vec<HypotheticalBallPosition<Ground>> {
        self.ball_filter
            .hypotheses
            .iter()
            .filter_map(|hypothesis| {
                if hypothesis.validity < validity_limit {
                    Some(HypotheticalBallPosition {
                        position: hypothesis.position().position,
                        validity: hypothesis.validity,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

fn mahalanobis_matrix_of_hypotheses_and_percepts(
    hypotheses: &[BallHypothesis],
    percepts: &[&BallPercept],
) -> Array2<NotNan<f32>> {
    Array2::from_shape_fn((hypotheses.len(), percepts.len()), |(i, j)| {
        let hypothesis = &hypotheses[i];
        let percept = &percepts[j];
        let ball = hypothesis.position();

        let residual = percept.percept_in_ground.mean - ball.position.inner.coords;
        let covariance = hypothesis.position_covariance();

        let mahalanobis_distance = residual.dot(
            &covariance
                .cholesky()
                .expect("covariance not invertible")
                .solve(&residual),
        );

        NotNan::new(-mahalanobis_distance).expect("mahalanobis distance is NaN")
    })
}

fn time_ordered_balls<'a>(
    balls_top: BTreeMap<SystemTime, Vec<Option<&'a Vec<BallPercept>>>>,
    balls_bottom: BTreeMap<SystemTime, Vec<Option<&'a Vec<BallPercept>>>>,
) -> BTreeMap<SystemTime, Vec<&'a BallPercept>> {
    let mut time_ordered_balls = BTreeMap::<SystemTime, Vec<&BallPercept>>::new();
    for (detection_time, balls) in balls_top.into_iter().chain(balls_bottom) {
        let balls = balls.into_iter().flatten().flatten();
        time_ordered_balls
            .entry(detection_time)
            .or_default()
            .extend(balls);
    }
    time_ordered_balls
}

fn decide_validity_decay_for_hypothesis(
    hypothesis: &BallHypothesis,
    camera_matrices: Option<&CameraMatrices>,
    projected_limbs: Option<&ProjectedLimbs>,
    ball_radius: f32,
    configuration: &BallFilterParameters,
) -> f32 {
    let is_ball_in_view =
        camera_matrices
            .zip(projected_limbs)
            .map_or(false, |(camera_matrices, projected_limbs)| {
                let ball = hypothesis.position();
                is_visible_to_camera(
                    &ball,
                    &camera_matrices.bottom,
                    ball_radius,
                    &projected_limbs.limbs,
                ) || is_visible_to_camera(&ball, &camera_matrices.top, ball_radius, &[])
            });

    match is_ball_in_view {
        true => configuration.visible_validity_exponential_decay_factor,
        false => configuration.hidden_validity_exponential_decay_factor,
    }
}

fn is_ball_inside_field(ball: BallPosition<Ground>, field_dimensions: &FieldDimensions) -> bool {
    ball.position.x().abs() < field_dimensions.length / 2.0
        && ball.position.y().abs() < field_dimensions.width / 2.0
}

fn project_to_image(
    filtered_balls: &[BallPosition<Ground>],
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
) -> Vec<Circle<Pixel>> {
    filtered_balls
        .iter()
        .filter_map(|filtered_ball| {
            let position_in_image = camera_matrix
                .ground_with_z_to_pixel(filtered_ball.position, ball_radius)
                .ok()?;
            let radius = camera_matrix
                .get_pixel_radius(ball_radius, position_in_image)
                .ok()?;
            Some(Circle {
                center: position_in_image,
                radius,
            })
        })
        .collect()
}

fn is_visible_to_camera(
    ball: &BallPosition<Ground>,
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
    projected_limbs: &[Limb],
) -> bool {
    let position_in_image = match camera_matrix.ground_with_z_to_pixel(ball.position, ball_radius) {
        Ok(position_in_image) => position_in_image,
        Err(_) => return false,
    };
    (0.0..640.0).contains(&position_in_image.x())
        && (0.0..480.0).contains(&position_in_image.y())
        && is_above_limbs(position_in_image, projected_limbs)
}

#[cfg(test)]
mod tests {
    use ball_filter::BallMode;
    use linear_algebra::point;
    use nalgebra::vector;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    use super::*;

    #[test]
    fn hypothesis_update_matching() {
        let hypothesis1 = BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::vector![0.0, 1.0, 0.0, 0.0],
                covariance: Matrix4::identity(),
            }),
            last_seen: SystemTime::now(),
            validity: 0.0,
        };
        let hypothesis2 = BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::vector![0.0, -1.0, 0.0, 0.0],
                covariance: Matrix4::identity(),
            }),
            last_seen: SystemTime::now(),
            validity: 0.0,
        };

        let percept1 = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: vector![0.0, 0.4],
                covariance: Matrix2::identity(),
            },
            image_location: Circle::new(point![0.0, 0.0], 1.0),
        };
        let percept2 = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: vector![0.0, -0.6],
                covariance: Matrix2::identity(),
            },
            image_location: Circle::new(point![0.0, 0.0], 1.0),
        };

        let hypotheses = vec![hypothesis1, hypothesis2];
        let percepts = vec![&percept1, &percept2];

        let costs = mahalanobis_matrix_of_hypotheses_and_percepts(&hypotheses, &percepts);
        let assignment = AssignmentProblem::from_costs(costs).solve();

        let percept_of_hypothesis1 = assignment[0].unwrap().to;
        assert_eq!(percept_of_hypothesis1, 0);

        let percept_of_hypothesis2 = assignment[1].unwrap().to;
        assert_eq!(percept_of_hypothesis2, 1);

        assert_eq!(assignment.len(), 2);
        assert_eq!(assignment.into_iter().flatten().count(), 2);
    }
}

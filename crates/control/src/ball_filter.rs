use std::{collections::BTreeMap, time::SystemTime};

use color_eyre::Result;
use nalgebra::{Matrix2, Matrix4};
use serde::{Deserialize, Serialize};

use ball_filter::{BallFilter as BallFiltering, BallHypothesis};
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use geometry::circle::Circle;
use linear_algebra::{distance, IntoFramed, IntoTransform, Isometry2, Point2};
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix, Projection};
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    limb::{is_above_limbs, Limb, ProjectedLimbs},
    parameters::BallFilterParameters,
};

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
    had_ground_contact: HistoricInput<bool, "has_firm_ground_contact">,
    historic_cycle_times: HistoricInput<CycleTime, "cycle_time">,

    camera_matrices: Input<Option<CameraMatrices>, "camera_matrices?">,
    cycle_time: Input<CycleTime, "cycle_time">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    ball_filter_configuration: Parameter<BallFilterParameters, "ball_filter">,

    balls_bottom: PerceptionInput<Option<Vec<BallPercept>>, "VisionBottom", "balls?">,
    balls_top: PerceptionInput<Option<Vec<BallPercept>>, "VisionTop", "balls?">,
    projected_limbs: PerceptionInput<Option<ProjectedLimbs>, "VisionBottom", "projected_limbs?">,
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
    ) -> Vec<BallHypothesis> {
        for (detection_time, balls) in measurements {
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

            let camera_matrices = camera_matrices.get(&detection_time);

            if !had_ground_contact.get(&detection_time) {
                self.ball_filter.reset();
                continue;
            }

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

            for ball in balls {
                let mean_position = ball.percept_in_ground.mean.framed().as_point();
                let is_hypothesis_detected = |hypothesis: &BallHypothesis| {
                    distance(
                        hypothesis
                            .choose_ball(filter_parameters.resting_ball_velocity_threshold)
                            .position,
                        mean_position,
                    ) < filter_parameters.measurement_matching_distance
                };
                let is_any_hypothesis_updated = self.ball_filter.update(
                    detection_time,
                    ball.percept_in_ground,
                    is_hypothesis_detected,
                );
                if !is_any_hypothesis_updated {
                    self.ball_filter.spawn(
                        detection_time,
                        mean_position,
                        Matrix4::from_diagonal(&filter_parameters.noise.initial_covariance),
                        Matrix2::from_diagonal(&filter_parameters.noise.initial_covariance.xy()),
                    )
                }
            }
        }

        let is_hypothesis_valid = |hypothesis: &BallHypothesis| {
            let ball = hypothesis.choose_ball(filter_parameters.resting_ball_velocity_threshold);
            let duration_since_last_observation = cycle_time
                .start_time
                .duration_since(ball.last_seen)
                .expect("time ran backwards");
            let validity_high_enough =
                hypothesis.validity >= filter_parameters.validity_discard_threshold;
            is_ball_inside_field(ball, field_dimensions)
                && validity_high_enough
                && duration_since_last_observation < filter_parameters.hypothesis_timeout
        };

        let should_merge_hypotheses =
            |hypothesis1: &BallHypothesis, hypothesis2: &BallHypothesis| {
                let ball1 =
                    hypothesis1.choose_ball(filter_parameters.resting_ball_velocity_threshold);
                let ball2 =
                    hypothesis2.choose_ball(filter_parameters.resting_ball_velocity_threshold);

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

        let filtered_ball =
            best_hypothesis.map(|hypothesis| hypothesis.choose_ball(velocity_threshold));

        let output_balls: Vec<_> = self
            .ball_filter
            .hypotheses()
            .iter()
            .filter_map(|hypothesis| {
                if hypothesis.validity >= filter_parameters.validity_output_threshold {
                    Some(hypothesis.choose_ball(velocity_threshold))
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
            .map(|hypothesis| hypothesis.choose_ball(velocity_threshold).position)
            .collect::<Vec<_>>();

        Ok(MainOutputs {
            ball_position: filtered_ball.into(),
            removed_ball_positions: removed_ball_positions.into(),
            hypothetical_ball_positions: self
                .hypothetical_ball_positions(
                    velocity_threshold,
                    filter_parameters.validity_output_threshold,
                )
                .into(),
        })
    }

    fn hypothetical_ball_positions(
        &self,
        velocity_threshold: f32,
        validity_limit: f32,
    ) -> Vec<HypotheticalBallPosition<Ground>> {
        self.ball_filter
            .hypotheses()
            .iter()
            .filter_map(|hypothesis| {
                if hypothesis.validity < validity_limit {
                    Some(HypotheticalBallPosition {
                        position: hypothesis.choose_ball(velocity_threshold).position,
                        validity: hypothesis.validity,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
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
                let ball = hypothesis.choose_ball(configuration.resting_ball_velocity_threshold);
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

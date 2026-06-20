use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use hungarian_algorithm::AssignmentProblem;
use linear_algebra::{IntoFramed, Isometry2};
use nalgebra::{Matrix2, Matrix4};
use ndarray::Array2;
use ordered_float::NotNan;
use ros_z::qos::QosDurability;

use booster::Odometer;
use coordinate_systems::{Ground, Pixel};
use geometry::circle::Circle;
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{context::Context, prelude::*, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Object, RobocupObjectLabel},
    parameters::BallFilterParameters,
    time_wrapper::TimeWrapper,
};

pub use crate::{
    filter::BallFilter,
    hypothesis::{BallHypothesis, BallMode},
};

mod filter;
mod hypothesis;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ball_filter").build().await?;

    let parameters = node.bind_parameter_as::<BallFilterParameters>("ball_filter")?;
    let field_dimensions_sub = node
        .create_cache::<FieldDimensions>("field_dimensions", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let camera_matrix_cache = node
        .create_cache::<TimeWrapper<CameraMatrix>>("camera_matrix", 10)?
        .with_stamp(|wrapper: &TimeWrapper<CameraMatrix>| wrapper.time)
        .build()
        .await?;
    let mut future_map = node
        .create_future_map_builder()
        .create_future_subscriber::<Odometer>("inputs/odometer", Duration::from_millis(1))
        .await?
        .create_future_subscriber::<Vec<Object<RobocupObjectLabel>>>(
            "detected_objects",
            Duration::from_millis(1),
        )
        .await?
        .build();
    let filter_state_pub = node
        .publisher::<BallFilter>("ball_filter/ball_filter_state")?
        .build()
        .await?;
    let best_ball_hypothesis_pub = node
        .publisher::<Option<BallHypothesis>>("ball_filter/best_ball_hypothesis")?
        .build()
        .await?;
    let filtered_balls_in_image_pub = node
        .publisher::<Vec<Circle<Pixel>>>("ball_filter/filtered_balls_in_image")?
        .build()
        .await?;
    let ball_percepts_pub = node
        .publisher::<Vec<BallPercept>>("ball_filter/ball_percepts")?
        .build()
        .await?;
    let ball_position_pub = node
        .publisher::<Option<BallPosition<Ground>>>("ball_filter/ball_position")?
        .build()
        .await?;
    let hypothetical_ball_positions_pub = node
        .publisher::<Vec<HypotheticalBallPosition<Ground>>>(
            "ball_filter/hypothetical_ball_positions",
        )?
        .build()
        .await?;

    let mut ball_filter = BallFilter::default();
    let mut last_odometer = None;
    let mut last_prediction_time = None;

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let future_map_item = future_map.recv().await?;

        let Some(field_dimensions) = field_dimensions_sub.get_latest() else {
            continue;
        };

        let output_time = future_map_item
            .persistent
            .last_key_value()
            .map(|(time, _)| *time);
        let mut ball_percepts = Vec::new();

        for (time, (odometer, detected_objects)) in future_map_item.persistent {
            if let Some(odometer) = odometer {
                predict_hypotheses_from_odometry(
                    &mut ball_filter,
                    time,
                    odometer,
                    &mut last_odometer,
                    &mut last_prediction_time,
                    parameters,
                );
            }

            if let Some(detected_objects) = detected_objects {
                let timed_camera_matrix = camera_matrix_cache.get_nearest(time);
                let camera_matrix = timed_camera_matrix
                    .as_ref()
                    .map(|camera_matrix| &camera_matrix.inner);
                let Some(projected_balls) = project_detected_balls(
                    Some(&detected_objects),
                    camera_matrix,
                    parameters,
                    field_dimensions.ball_radius,
                ) else {
                    continue;
                };

                ball_percepts.extend_from_slice(&projected_balls);

                advance_all_hypotheses(
                    &mut ball_filter,
                    time,
                    &projected_balls,
                    camera_matrix,
                    parameters,
                    &field_dimensions,
                );
            }
        }

        if let Some(output_time) = output_time {
            remove_invalid_and_merge_hypotheses(
                &mut ball_filter,
                output_time,
                parameters,
                &field_dimensions,
            );
        }

        ball_percepts_pub.publish(&ball_percepts).await?;
        filter_state_pub.publish(&ball_filter.clone()).await?;

        let best_hypothesis = ball_filter.best_hypothesis(parameters.validity_output_threshold);

        best_ball_hypothesis_pub
            .publish(&best_hypothesis.cloned())
            .await?;

        let filtered_ball = best_hypothesis.map(|hypothesis| hypothesis.position());

        let output_balls: Vec<_> = ball_filter
            .hypotheses
            .iter()
            .filter_map(|hypothesis| {
                if hypothesis.validity >= parameters.validity_output_threshold {
                    Some(hypothesis.position())
                } else {
                    None
                }
            })
            .collect();

        let ball_radius = field_dimensions.ball_radius;

        let projection_time = output_time.or_else(|| {
            future_map_item
                .temporary
                .first_key_value()
                .map(|(time, _)| *time)
        });

        let filtered_balls_in_image = if let Some(time) = projection_time
            && let Some(timed_camera_matrix) = camera_matrix_cache.get_nearest(time)
        {
            project_to_image(&output_balls, &timed_camera_matrix.inner, ball_radius)
        } else {
            vec![]
        };
        filtered_balls_in_image_pub
            .publish(&filtered_balls_in_image)
            .await?;

        ball_position_pub.publish(&filtered_ball).await?;
        let hypothetical_ball_positions =
            hypothetical_ball_positions(&ball_filter, parameters.validity_output_threshold);
        hypothetical_ball_positions_pub
            .publish(&hypothetical_ball_positions)
            .await?;
    }
}

fn predict_hypotheses_from_odometry(
    ball_filter: &mut BallFilter,
    time: Time,
    odometer: Odometer,
    last_odometer: &mut Option<Odometer>,
    last_prediction_time: &mut Option<Time>,
    filter_parameters: &BallFilterParameters,
) {
    let last_to_current = match *last_odometer {
        None => Isometry2::identity(),
        Some(previous_odometer) => previous_odometer.to(odometer),
    };
    let delta_time =
        last_prediction_time.map_or(Duration::ZERO, |last_time| time.duration_since(last_time));
    *last_odometer = Some(odometer);
    *last_prediction_time = Some(time);

    ball_filter
        .hypotheses
        .retain(|hypothesis| hypothesis.validity > filter_parameters.validity_discard_threshold);

    ball_filter.predict(
        delta_time,
        last_to_current,
        filter_parameters.velocity_decay_factor,
        Matrix4::from_diagonal(&filter_parameters.noise.process_noise_moving),
        Matrix2::from_diagonal(&filter_parameters.noise.process_noise_resting),
        filter_parameters.log_likelihood_of_zero_velocity_threshold,
    );
}

fn advance_all_hypotheses(
    ball_filter: &mut BallFilter,
    time: Time,
    ball_percepts: &[BallPercept],
    camera_matrix: Option<&CameraMatrix>,
    filter_parameters: &BallFilterParameters,
    field_dimensions: &FieldDimensions,
) {
    ball_filter
        .hypotheses
        .retain(|hypothesis| hypothesis.validity > filter_parameters.validity_discard_threshold);

    ball_filter.decay_hypotheses(|hypothesis| {
        decide_validity_decay_for_hypothesis(
            hypothesis,
            camera_matrix,
            field_dimensions.ball_radius,
            filter_parameters,
        )
    });

    if !ball_percepts.is_empty() {
        let match_matrix =
            mahalanobis_matrix_of_hypotheses_and_percepts(&ball_filter.hypotheses, ball_percepts);

        let assignment = AssignmentProblem::from_costs(match_matrix).solve();

        let mut used_percepts = vec![];

        for (hypothesis, assigned_percept) in
            ball_filter.hypotheses.iter_mut().zip(assignment.iter())
        {
            if let Some(assigned_percept) = assigned_percept {
                let mahalanobis_distance = -assigned_percept.cost;
                if mahalanobis_distance > filter_parameters.maximum_matching_cost {
                    hypothesis.validity *=
                        filter_parameters.maximum_matching_cost_validity_penalty_factor;
                    continue;
                }
                let validity_increase = assigned_percept.cost.exp();
                let percept = ball_percepts[assigned_percept.to];
                used_percepts.push(assigned_percept.to);
                hypothesis.update(time, percept.percept_in_ground, validity_increase);
            }
        }

        let unused_percepts = {
            let mut all_percepts = ball_percepts.to_vec();
            used_percepts.sort_unstable();
            for index in used_percepts.into_iter().rev() {
                all_percepts.remove(index);
            }
            all_percepts
        };

        for percept in unused_percepts {
            ball_filter.spawn(
                time,
                percept.percept_in_ground,
                Matrix4::from_diagonal(&filter_parameters.noise.initial_covariance),
            );
        }
    }
}

fn remove_invalid_and_merge_hypotheses(
    ball_filter: &mut BallFilter,
    time: Time,
    filter_parameters: &BallFilterParameters,
    field_dimensions: &FieldDimensions,
) {
    let is_hypothesis_valid = |hypothesis: &BallHypothesis| {
        let ball = hypothesis.position();
        let Some(duration_since_last_observation) = ball.age_at(time) else {
            return false;
        };
        let validity_high_enough =
            hypothesis.validity >= filter_parameters.validity_discard_threshold;
        is_ball_inside_field(ball, field_dimensions)
            && validity_high_enough
            && duration_since_last_observation < filter_parameters.hypothesis_timeout
    };

    let should_merge_hypotheses =
        |hypothesis1: &BallHypothesis, hypothesis2: &BallHypothesis| match (
            &hypothesis1.mode,
            &hypothesis2.mode,
        ) {
            (BallMode::Resting(ball1), BallMode::Resting(ball2)) => {
                (ball1.mean - ball2.mean).norm() < filter_parameters.hypothesis_merge_distance
            }
            _ => false,
        };

    ball_filter.remove_hypotheses(is_hypothesis_valid, should_merge_hypotheses);
    ball_filter
        .hypotheses
        .sort_unstable_by(|a, b| b.validity.total_cmp(&a.validity));
    ball_filter
        .hypotheses
        .truncate(filter_parameters.maximum_number_of_hypotheses);
}

fn hypothetical_ball_positions(
    ball_filter: &BallFilter,
    validity_limit: f32,
) -> Vec<HypotheticalBallPosition<Ground>> {
    ball_filter
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

fn mahalanobis_matrix_of_hypotheses_and_percepts(
    hypotheses: &[BallHypothesis],
    percepts: &[BallPercept],
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

fn project_detected_balls(
    detections: Option<&[Object<RobocupObjectLabel>]>,
    camera_matrix: Option<&CameraMatrix>,
    parameters: &BallFilterParameters,
    ball_radius: f32,
) -> Option<Vec<BallPercept>> {
    let (Some(detections), Some(camera_matrix)) = (detections, camera_matrix) else {
        return None;
    };
    Some(
        detections
            .iter()
            .filter_map(|detection| {
                if detection.label != RobocupObjectLabel::Ball {
                    return None;
                }
                let area = detection.bounding_box.area;
                let position = camera_matrix
                    .pixel_to_ground_with_z(area.center(), ball_radius)
                    .ok()?;

                let detected_ball_radius =
                    (area.max.x() - area.min.x()).min(area.max.y() - area.min.y()) / 2.0;

                let circle = Circle {
                    center: area.center(),
                    radius: detected_ball_radius,
                };

                let projected_covariance = {
                    let scaled_noise = parameters
                        .noise
                        .detection_noise
                        .inner
                        .map(|x| (detected_ball_radius * x).powi(2))
                        .framed();
                    camera_matrix
                        .project_noise_to_ground(position, scaled_noise)
                        .ok()?
                };

                Some(BallPercept {
                    percept_in_ground: MultivariateNormalDistribution {
                        mean: position.inner.coords,
                        covariance: projected_covariance,
                    },
                    image_location: circle,
                })
            })
            .collect(),
    )
}

fn decide_validity_decay_for_hypothesis(
    hypothesis: &BallHypothesis,
    camera_matrix: Option<&CameraMatrix>,
    ball_radius: f32,
    configuration: &BallFilterParameters,
) -> f32 {
    let is_ball_in_view = camera_matrix.is_some_and(|camera_matrix| {
        let ball = hypothesis.position();
        is_visible_to_camera(&ball, camera_matrix, ball_radius)
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
) -> bool {
    let position_in_image = match camera_matrix.ground_with_z_to_pixel(ball.position, ball_radius) {
        Ok(position_in_image) => position_in_image,
        Err(_) => return false,
    };
    (0.0..640.0).contains(&position_in_image.x()) && (0.0..480.0).contains(&position_in_image.y())
}

#[cfg(test)]
mod tests {
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
            last_seen: Time::zero(),
            validity: 0.0,
        };
        let hypothesis2 = BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::vector![0.0, -1.0, 0.0, 0.0],
                covariance: Matrix4::identity(),
            }),
            last_seen: Time::zero(),
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
        let percepts = vec![percept1, percept2];

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

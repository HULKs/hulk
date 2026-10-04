mod reacquisition;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::{Result, eyre::WrapErr};
use linear_algebra::{IntoFramed, Isometry2, Pose2};
use linear_sum_assignment::{AssignmentSolver, Objective};
use nalgebra::{Matrix2, Matrix4};
use ndarray::Array2;
use ros_z::qos::QosDurability;

use coordinate_systems::{Field, Ground, Odometry, Pixel};
use geometry::circle::Circle;
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{context::Context, prelude::*, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use tokio::task::block_in_place;
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Object, RobocupObjectLabel},
    obstacles::Obstacle,
    odometry,
    parameters::BallFilterParameters,
    time_wrapper::TimeWrapper,
};

pub use crate::{
    filter::BallFilter,
    hypothesis::{BallHypothesis, BallMode},
};

mod competition;
mod field_prior;
mod filter;
mod hypothesis;
mod negative_evidence;
mod obstacle_input;
pub mod tracker;
mod validity_decay;
use tracker::{InputStamp, Tracker, UpdateSchedule, camera_is_recent};

struct BallFilterOutput {
    time: Option<Time>,
    ball_percepts: Vec<TimeWrapper<Vec<BallPercept>>>,
    filter_state: BallFilter,
    best_hypothesis: Option<BallHypothesis>,
    filtered_ball: Option<BallPosition<Ground>>,
    filtered_balls_in_image: Vec<Circle<Pixel>>,
    hypothetical_ball_positions: Vec<HypotheticalBallPosition<Ground>>,
    schedule: UpdateSchedule,
    field_prior_pose: Option<Isometry2<Ground, Field>>,
    obstacle_inputs: Vec<TimeWrapper<Option<TimeWrapper<Vec<Obstacle>>>>>,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ball_filter").build().await?;

    let parameters = node.bind_parameter_as::<BallFilterParameters>("ball_filter")?;
    let field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let camera_matrix_cache = node
        .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .cache(200)
        .with_stamp(|wrapper: &TimeWrapper<CameraMatrix>| wrapper.time)
        .build()
        .await?;
    let field_pose_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await?;
    let field_prior_pose_pub = node
        .publisher::<TimeWrapper<Option<Isometry2<Ground, Field>>>>(tracker::FIELD_PRIOR_POSE_TOPIC)
        .build()
        .await?;
    let mut field_poses = field_prior::FieldPoseHistory::default();
    let obstacle_sub = node
        .subscriber::<Vec<Obstacle>>("obstacles")
        .build()
        .await?;
    let selected_obstacles_pub = node
        .publisher::<TimeWrapper<Option<TimeWrapper<Vec<Obstacle>>>>>(
            tracker::SELECTED_OBSTACLES_TOPIC,
        )
        .build()
        .await?;
    let mut obstacles = obstacle_input::ObstacleHistory::default();
    let mut future_map = node
        .create_future_map_builder()
        .create_future_subscriber::<Pose2<Odometry>>("inputs/odometry", Duration::from_millis(1))
        .await?
        .create_future_subscriber::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>(
            "detected_objects",
            Duration::from_millis(25),
        )
        .await?
        .build();
    let filter_state_pub = node
        .publisher::<BallFilter>("ball_filter/ball_filter_state")
        .build()
        .await?;
    let best_ball_hypothesis_pub = node
        .publisher::<Option<BallHypothesis>>("ball_filter/best_ball_hypothesis")
        .build()
        .await?;
    let filtered_balls_in_image_pub = node
        .publisher::<Vec<Circle<Pixel>>>("ball_filter/filtered_balls_in_image")
        .build()
        .await?;
    let ball_percepts_pub = node
        .publisher::<Vec<BallPercept>>("ball_filter/ball_percepts")
        .build()
        .await?;
    let ball_position_pub = node
        .publisher::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
        .build()
        .await?;
    let hypothetical_ball_positions_pub = node
        .publisher::<Vec<HypotheticalBallPosition<Ground>>>(
            "ball_filter/hypothetical_ball_positions",
        )
        .build()
        .await?;

    let schedule_pub = node
        .publisher::<UpdateSchedule>("ball_filter/update_schedule")
        .build()
        .await?;
    let mut tracker = Tracker::default();
    let mut sequence = 0_u64;
    let mut last_field_prior_pose = None;

    loop {
        let future_map_item = tokio::select! {
            received = field_pose_sub.recv_with_metadata() => {
                let received = received?;
                field_poses.insert(received.source_time, received.message);
                continue;
            }
            received = obstacle_sub.recv_with_metadata() => {
                let received = received?;
                obstacles.insert(received.source_time, received.message);
                continue;
            }
            item = future_map.recv() => item?,
        };
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let Some(field_dimensions) = field_dimensions_sub.get_latest() else {
            continue;
        };

        let output = block_in_place(|| -> Result<BallFilterOutput> {
            let output_time = future_map_item
                .persistent
                .last_key_value()
                .map(|(time, _)| *time);
            let projection_time = output_time.or_else(|| {
                future_map_item
                    .temporary
                    .first_key_value()
                    .map(|(time, _)| *time)
            });
            let mut ball_percepts = Vec::new();
            let mut obstacle_inputs = Vec::new();

            let mut schedule = UpdateSchedule {
                sequence,
                inputs: Vec::new(),
            };
            for (time, (odometry_pose, detected_objects)) in future_map_item.persistent {
                let camera = camera_matrix_cache.get_nearest(time);
                schedule.inputs.push(InputStamp {
                    time,
                    odometry: odometry_pose.is_some(),
                    detections: detected_objects.is_some(),
                    camera_time: camera.as_ref().map(|c| c.time),
                });
                let selected_obstacles = if detected_objects.is_some() {
                    let selected = obstacles.at(time, parameters.maximum_obstacle_time_difference);
                    obstacle_inputs.push(TimeWrapper {
                        time,
                        inner: selected.clone(),
                    });
                    selected
                } else {
                    None
                };
                let frame_percepts = tracker.advance_with_obstacles(
                    time,
                    odometry_pose,
                    detected_objects.as_ref().map(|d| d.inner.as_slice()),
                    camera.as_deref(),
                    selected_obstacles.as_ref(),
                    parameters,
                    &field_dimensions,
                )?;
                // An empty actual detector exposure revokes visual contact. Odometry-only
                // fusion batches must not masquerade as empty camera observations.
                if detected_objects.is_some() {
                    ball_percepts.push(TimeWrapper {
                        time,
                        inner: frame_percepts,
                    });
                }
            }
            // Temporary-only batches leave the state at its previous timestamp.
            // Keep the same prior for their diagnostic outputs, too.
            let field_prior_pose = output_time
                .map(|time| field_poses.at(time))
                .unwrap_or(last_field_prior_pose);
            if let Some(time) = output_time {
                tracker.finish_with_field_pose(
                    time,
                    parameters,
                    &field_dimensions,
                    field_prior_pose,
                );
                sequence += 1;
            }
            let ball_filter = &tracker.filter;

            let filter_state = ball_filter.clone();
            let best_hypothesis = ball_filter
                .best_hypothesis_with_field_pose(parameters, &field_dimensions, field_prior_pose)
                .cloned();
            let filtered_ball = best_hypothesis
                .as_ref()
                .map(|hypothesis| hypothesis.position());

            let output_balls: Vec<_> = ball_filter
                .hypotheses
                .iter()
                .filter_map(|hypothesis| {
                    if field_prior::effective_validity(
                        hypothesis,
                        field_prior_pose,
                        &field_dimensions,
                        parameters,
                    ) >= parameters.validity_output_threshold
                    {
                        Some(hypothesis.position())
                    } else {
                        None
                    }
                })
                .collect();

            let ball_radius = field_dimensions.ball_radius;
            let filtered_balls_in_image = if let Some(time) = projection_time
                && let Some(timed_camera_matrix) = camera_matrix_cache.get_nearest(time)
                && camera_is_recent(
                    time,
                    timed_camera_matrix.time,
                    parameters.maximum_camera_matrix_age,
                ) {
                project_to_image(&output_balls, &timed_camera_matrix.inner, ball_radius)
            } else {
                vec![]
            };
            let hypothetical_ball_positions = hypothetical_ball_positions(
                ball_filter,
                parameters,
                &field_dimensions,
                field_prior_pose,
            );

            Ok(BallFilterOutput {
                time: output_time,
                ball_percepts,
                filter_state,
                best_hypothesis,
                filtered_ball,
                filtered_balls_in_image,
                hypothetical_ball_positions,
                schedule,
                field_prior_pose,
                obstacle_inputs,
            })
        })?;

        for obstacles in &output.obstacle_inputs {
            selected_obstacles_pub
                .publish_with_source_time(obstacles, obstacles.time)
                .await?;
        }
        for percepts in &output.ball_percepts {
            ball_percepts_pub
                .publish_with_source_time(&percepts.inner, percepts.time)
                .await?;
        }
        best_ball_hypothesis_pub
            .publish(&output.best_hypothesis)
            .await?;
        filtered_balls_in_image_pub
            .publish(&output.filtered_balls_in_image)
            .await?;

        // Preserve the state timestamp: publication happens after the fusion safety lag.
        // Consumers comparing estimates with sensor truth must not use delivery time.
        if let Some(time) = output.time {
            filter_state_pub
                .publish_with_source_time(&output.filter_state, time)
                .await?;
            last_field_prior_pose = output.field_prior_pose;
            field_prior_pose_pub
                .publish_with_source_time(
                    &TimeWrapper {
                        time,
                        inner: output.field_prior_pose,
                    },
                    time,
                )
                .await?;
            schedule_pub
                .publish_if_subscribed(|| async { output.schedule })
                .await?;
            ball_position_pub
                .publish_with_source_time(&output.filtered_ball, time)
                .await?;
        }
        hypothetical_ball_positions_pub
            .publish(&output.hypothetical_ball_positions)
            .await?;
    }
}

fn predict_hypotheses_from_odometry(
    ball_filter: &mut BallFilter,
    time: Time,
    odometry_pose: Pose2<Odometry>,
    last_odometry: &mut Option<Pose2<Odometry>>,
    last_prediction_time: &mut Option<Time>,
    filter_parameters: &BallFilterParameters,
) {
    let last_to_current = match *last_odometry {
        None => Isometry2::identity(),
        Some(previous_odometry) => odometry::previous_to_current(previous_odometry, odometry_pose),
    };
    let delta_time =
        last_prediction_time.map_or(Duration::ZERO, |last_time| time.duration_since(last_time));
    *last_odometry = Some(odometry_pose);
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
    let resting_speed = filter_parameters.resting_velocity_threshold;
    if resting_speed.is_finite() && resting_speed > 0.0 {
        for hypothesis in &mut ball_filter.hypotheses {
            if let hypothesis::BallMode::Moving(state) = hypothesis.mode
                && state.mean.z.hypot(state.mean.w) <= resting_speed
            {
                hypothesis.mode = hypothesis::BallMode::Resting(MultivariateNormalDistribution {
                    mean: state.mean.xy(),
                    covariance: state.covariance.fixed_view::<2, 2>(0, 0).into_owned(),
                });
                hypothesis.motion_evidence = None;
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Keep independently timestamped sensor inputs explicit at the replay boundary"
)]
fn advance_all_hypotheses(
    ball_filter: &mut BallFilter,
    assignment_solver: &mut AssignmentSolver,
    time: Time,
    ball_percepts: &[BallPercept],
    camera_matrix: Option<&CameraMatrix>,
    obstacles: Option<&[Obstacle]>,
    detections: &[Object<RobocupObjectLabel>],
    filter_parameters: &BallFilterParameters,
    field_dimensions: &FieldDimensions,
) -> Result<()> {
    ball_filter
        .hypotheses
        .retain(|hypothesis| hypothesis.validity > filter_parameters.validity_discard_threshold);

    // Association depends on position/covariance, not confidence, so solve
    // before applying decay to distinguish a matched track from a clear miss.
    let mut match_matrix =
        mahalanobis_matrix_of_hypotheses_and_percepts(&ball_filter.hypotheses, ball_percepts);
    // Uncertainty grows during occlusion. It must not authorize an arbitrarily
    // distant false percept to update a retained track when this gate is enabled.
    if filter_parameters.maximum_matching_distance.is_finite()
        && filter_parameters.maximum_matching_distance > 0.0
    {
        let maximum_squared = filter_parameters.maximum_matching_distance.powi(2);
        for ((hypothesis, percept), cost) in match_matrix.indexed_iter_mut() {
            let residual = ball_percepts[percept].percept_in_ground.mean
                - ball_filter.hypotheses[hypothesis]
                    .position()
                    .position
                    .inner
                    .coords;
            if residual.norm_squared() > maximum_squared {
                *cost = f32::NEG_INFINITY;
            }
        }
    }
    // A diffuse retained prediction must not turn an isolated distant false
    // detection into high-confidence history after an observation gap. Gating
    // leaves that history intact and lets the percept spawn its own hypothesis.
    let reacquisition_distance = filter_parameters.reacquisition_matching_distance;
    if reacquisition_distance.is_finite() && reacquisition_distance > 0.0 {
        for ((row, column), score) in match_matrix.indexed_iter_mut() {
            let hypothesis = &ball_filter.hypotheses[row];
            if reacquisition::protect_prior(hypothesis, time, obstacles, filter_parameters)
                && (ball_percepts[column].percept_in_ground.mean
                    - hypothesis.position().position.inner.coords)
                    .norm_squared()
                    > reacquisition_distance.powi(2)
            {
                *score = f32::NEG_INFINITY;
            }
        }
    }
    let assignment = if ball_percepts.is_empty() {
        None
    } else {
        let mut assignment_scores =
            gated_assignment_scores(&match_matrix, filter_parameters.maximum_matching_cost);
        let uncertainty_weight = filter_parameters.association_uncertainty_weight;
        if uncertainty_weight.is_finite() && uncertainty_weight > 0.0 {
            for (row, hypothesis) in ball_filter.hypotheses.iter().enumerate() {
                let variance = hypothesis.position_covariance().trace().max(0.0);
                let penalty = uncertainty_weight.min(1.0) * variance / (1.0 + variance);
                for column in 0..ball_percepts.len() {
                    assignment_scores[(row, column)] -= penalty;
                }
            }
        }
        Some(
            assignment_solver
                .solve(assignment_scores.view(), Objective::Maximize)
                .wrap_err("failed to solve ball assignment")?,
        )
    };
    let mut used_percepts = vec![];
    let mut matched = vec![false; ball_filter.hypotheses.len()];
    for (hypothesis_index, hypothesis) in ball_filter.hypotheses.iter_mut().enumerate() {
        let percept_index = assignment
            .as_ref()
            .and_then(|assignment| assignment[hypothesis_index])
            .filter(|&index| index < ball_percepts.len());
        let legacy_factor = decide_validity_decay_for_hypothesis(
            hypothesis,
            camera_matrix,
            field_dimensions.ball_radius,
            obstacles,
            detections,
            filter_parameters,
        );
        let visibility = if percept_index.is_none() && validity_decay::enabled(filter_parameters) {
            camera_matrix.map_or(negative_evidence::Visibility::Unknown, |camera| {
                hypothesis_visibility(
                    hypothesis,
                    camera,
                    field_dimensions.ball_radius,
                    obstacles,
                    detections,
                    filter_parameters,
                )
            })
        } else {
            negative_evidence::Visibility::Unknown
        };
        let decay_factor = validity_decay::factor(
            hypothesis,
            time,
            visibility,
            percept_index.is_some(),
            legacy_factor,
            filter_parameters,
        );
        hypothesis.validity *= decay_factor;
        if let Some(percept_index) = percept_index {
            let score = match_matrix[(hypothesis_index, percept_index)];
            used_percepts.push(percept_index);
            matched[hypothesis_index] = true;
            hypothesis.update(
                time,
                ball_percepts[percept_index].percept_in_ground,
                score.exp(),
            );
            if filter_parameters.publication_filter_blend > 0.0 {
                hypothesis.last_observation_size_plausible = radius_consistency(
                    &ball_percepts[percept_index],
                    camera_matrix,
                    field_dimensions.ball_radius,
                );
            }
        }
    }
    for (index, percept) in ball_percepts.iter().enumerate() {
        if !used_percepts.contains(&index) {
            ball_filter.spawn(
                time,
                percept.percept_in_ground,
                Matrix4::from_diagonal(&filter_parameters.noise.initial_covariance),
                filter_parameters.nearby_spawn_validity_factor,
            );
            if filter_parameters.publication_filter_blend > 0.0
                && let Some(hypothesis) = ball_filter.hypotheses.last_mut()
            {
                hypothesis.last_observation_size_plausible =
                    radius_consistency(percept, camera_matrix, field_dimensions.ball_radius);
            }
            matched.push(true);
        }
    }
    competition::apply(
        ball_filter,
        time,
        &matched,
        camera_matrix,
        obstacles,
        detections,
        field_dimensions.ball_radius,
        filter_parameters,
    );

    if negative_evidence::enabled(filter_parameters) {
        ball_filter.hypotheses.retain_mut(|hypothesis| {
            if hypothesis.last_seen == time {
                hypothesis.negative_evidence = None;
                return true;
            }
            let position = hypothesis.position();
            let clearly_visible = camera_matrix.is_some_and(|camera| {
                hypothesis_visibility(
                    hypothesis,
                    camera,
                    field_dimensions.ball_radius,
                    obstacles,
                    detections,
                    filter_parameters,
                ) == negative_evidence::Visibility::Visible
            });
            let evidence = hypothesis
                .negative_evidence
                .get_or_insert_with(Default::default);
            if clearly_visible {
                let expired = evidence
                    .observe_clear_miss(time, filter_parameters.visible_missed_detection_timeout);
                let near = negative_evidence::near_enabled(filter_parameters)
                    && position.position.coords().norm()
                        <= filter_parameters.near_visible_missed_detection_distance;
                if near {
                    let (near_expired, interval) = evidence.observe_near_miss(
                        time,
                        filter_parameters.near_visible_missed_detection_timeout,
                    );
                    hypothesis.validity *=
                        negative_evidence::near_decay_factor(interval, filter_parameters);
                    !expired && !near_expired
                } else {
                    evidence.near = None;
                    !expired
                }
            } else {
                evidence.pause();
                true
            }
        });
    }

    Ok(())
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
        is_ball_within_field_range(ball, field_dimensions)
            && validity_high_enough
            && duration_since_last_observation < filter_parameters.hypothesis_timeout
    };

    let should_merge_hypotheses = |left: &BallHypothesis, right: &BallHypothesis| {
        left.can_merge(
            right,
            filter_parameters.hypothesis_merge_distance,
            filter_parameters.validity_output_threshold,
        )
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
    parameters: &BallFilterParameters,
    dimensions: &FieldDimensions,
    ground_to_field: Option<Isometry2<Ground, Field>>,
) -> Vec<HypotheticalBallPosition<Ground>> {
    ball_filter
        .hypotheses
        .iter()
        .filter_map(|hypothesis| {
            let validity = field_prior::effective_validity(
                hypothesis,
                ground_to_field,
                dimensions,
                parameters,
            );
            if validity < parameters.validity_output_threshold {
                Some(HypotheticalBallPosition {
                    position: hypothesis.position().position,
                    validity,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Gate before assignment: an impossible pair must not consume a percept that
/// another hypothesis could use. Each row can instead select an unmatched column.
/// Unmatched tracks already receive visibility decay; an unrelated detection is
/// not additional evidence against a track hidden behind another robot.
fn gated_assignment_scores(scores: &Array2<f32>, maximum_cost: f32) -> Array2<f32> {
    Array2::from_shape_fn(
        (scores.nrows(), scores.ncols() + scores.nrows()),
        |(row, column)| {
            if column >= scores.ncols() {
                -(maximum_cost + 1.0)
            } else if -scores[(row, column)] > maximum_cost {
                f32::NEG_INFINITY
            } else {
                scores[(row, column)]
            }
        },
    )
}

fn mahalanobis_matrix_of_hypotheses_and_percepts(
    hypotheses: &[BallHypothesis],
    percepts: &[BallPercept],
) -> Array2<f32> {
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

        -mahalanobis_distance
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
    if !camera_matrix
        .intrinsics
        .as_matrix()
        .iter()
        .all(|value| value.is_finite())
        || !camera_matrix
            .intrinsics
            .focals
            .iter()
            .all(|value| *value > 0.0)
        || !camera_matrix
            .ground_to_camera
            .inner
            .to_homogeneous()
            .iter()
            .all(|value| value.is_finite())
        || !ball_radius.is_finite()
        || ball_radius <= 0.0
    {
        return None;
    }
    Some(
        detections
            .iter()
            .filter_map(|detection| {
                if detection.label != RobocupObjectLabel::Ball {
                    return None;
                }
                let confidence = detection.bounding_box.confidence;
                if !confidence.is_finite()
                    || !(0.0..=1.0).contains(&confidence)
                    || confidence < parameters.ball_confidence_threshold
                {
                    return None;
                }

                let area = detection.bounding_box.area;
                if ![area.min.x(), area.min.y(), area.max.x(), area.max.y()]
                    .iter()
                    .all(|value| value.is_finite())
                    || area.min.x() >= area.max.x()
                    || area.min.y() >= area.max.y()
                {
                    return None;
                }
                // The projection already rejects rays above the ball-height
                // horizon. Validate its result before association or spawning:
                // near-horizon geometry can otherwise produce enormous tracks.
                let position = camera_matrix
                    .pixel_to_ground_with_z(area.center(), ball_radius)
                    .ok()?;
                if !position.x().is_finite() || !position.y().is_finite() {
                    return None;
                }
                let maximum_distance = parameters.maximum_detection_distance;
                if maximum_distance.is_finite()
                    && maximum_distance > 0.0
                    && position.coords().norm() > maximum_distance
                {
                    return None;
                }

                let detected_ball_radius =
                    (area.max.x() - area.min.x()).min(area.max.y() - area.min.y()) / 2.0;
                if !detected_ball_radius.is_finite() || detected_ball_radius <= 0.0 {
                    return None;
                }
                let maximum_ratio = parameters.maximum_detection_radius_ratio;
                let radius_distance = parameters.radius_consistency_maximum_distance;
                let check_size = !radius_distance.is_finite()
                    || radius_distance <= 0.0
                    || position.coords().norm() <= radius_distance;
                if maximum_ratio.is_finite() && maximum_ratio > 1.0 && check_size {
                    let in_camera = camera_matrix.ground_to_camera
                        * linear_algebra::point![position.x(), position.y(), ball_radius];
                    let depth = in_camera.z();
                    if !depth.is_finite() || depth <= 0.0 {
                        return None;
                    }
                    let expected = ball_radius
                        * camera_matrix
                            .intrinsics
                            .focals
                            .x
                            .min(camera_matrix.intrinsics.focals.y)
                        / depth;
                    if !expected.is_finite()
                        || expected <= 0.0
                        || detected_ball_radius > maximum_ratio * expected
                        || expected > maximum_ratio * detected_ball_radius
                    {
                        return None;
                    }
                }

                let circle = Circle {
                    center: area.center(),
                    radius: detected_ball_radius,
                };

                let projected_covariance = {
                    if !parameters
                        .noise
                        .detection_noise
                        .inner
                        .iter()
                        .all(|value| value.is_finite() && *value >= 0.0)
                    {
                        return None;
                    }
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
                if !projected_covariance.iter().all(|value| value.is_finite())
                    || projected_covariance[(0, 0)] < 0.0
                    || projected_covariance[(1, 1)] < 0.0
                {
                    return None;
                }

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

// A larger image can be an airborne ball. Only an undersized image implies
// a sphere below the ground plane, so only that direction permits correction.
fn radius_consistency(
    percept: &BallPercept,
    camera: Option<&CameraMatrix>,
    ball_radius: f32,
) -> Option<bool> {
    let camera = camera?;
    let position = percept.percept_in_ground.mean;
    let camera_position =
        camera.ground_to_camera * linear_algebra::point![position.x, position.y, ball_radius];
    let depth = camera_position.z();
    let observed = percept.image_location.radius;
    let expected = ball_radius * camera.intrinsics.focals.x.min(camera.intrinsics.focals.y) / depth;
    (depth.is_finite()
        && depth > 0.0
        && observed.is_finite()
        && observed > 0.0
        && expected.is_finite()
        && expected > 0.0)
        .then_some(expected <= 1.5 * observed)
}

fn hypothesis_visibility(
    hypothesis: &BallHypothesis,
    camera: &CameraMatrix,
    ball_radius: f32,
    obstacles: Option<&[Obstacle]>,
    detections: &[Object<RobocupObjectLabel>],
    parameters: &BallFilterParameters,
) -> negative_evidence::Visibility {
    use negative_evidence::Visibility;
    let mut ball = hypothesis.position();
    let center = negative_evidence::classify_with_detections(
        &ball,
        camera,
        ball_radius,
        obstacles,
        detections,
    );
    let scale = parameters.visibility_uncertainty_scale;
    if center != Visibility::Visible || !scale.is_finite() || scale <= 0.0 {
        return center;
    }
    let covariance = hypothesis.position_covariance();
    if !covariance.iter().all(|x| x.is_finite()) {
        return Visibility::Unknown;
    }
    // The axis-aligned rectangle encloses the scaled covariance ellipse.
    // Requiring its corners to be visible prevents a confident clear miss when
    // plausible locations extend outside the camera or into known occlusion.
    let x = scale * covariance[(0, 0)].max(0.0).sqrt();
    let y = scale * covariance[(1, 1)].max(0.0).sqrt();
    let position = ball.position;
    for (dx, dy) in [(-x, -y), (-x, y), (x, -y), (x, y)] {
        ball.position = position + linear_algebra::vector![dx, dy];
        if negative_evidence::classify_with_detections(
            &ball,
            camera,
            ball_radius,
            obstacles,
            detections,
        ) != Visibility::Visible
        {
            return Visibility::Unknown;
        }
    }
    center
}

fn decide_validity_decay_for_hypothesis(
    hypothesis: &BallHypothesis,
    camera_matrix: Option<&CameraMatrix>,
    ball_radius: f32,
    obstacles: Option<&[Obstacle]>,
    detections: &[Object<RobocupObjectLabel>],
    configuration: &BallFilterParameters,
) -> f32 {
    let is_ball_in_view = camera_matrix.is_some_and(|camera_matrix| {
        let ball = hypothesis.position();
        if !negative_evidence::enabled(configuration) {
            is_visible_to_camera(&ball, camera_matrix, ball_radius)
        } else {
            hypothesis_visibility(
                hypothesis,
                camera_matrix,
                ball_radius,
                obstacles,
                detections,
                configuration,
            ) == negative_evidence::Visibility::Visible
        }
    });

    match is_ball_in_view {
        true => configuration.visible_validity_exponential_decay_factor,
        false => configuration.hidden_validity_exponential_decay_factor,
    }
}

fn is_ball_within_field_range(
    ball: BallPosition<Ground>,
    field_dimensions: &FieldDimensions,
) -> bool {
    // Ground is centered on the robot, not the field. Without localization we
    // can only reject distances beyond the whole field's diagonal, including
    // its border strip. Half-field bounds incorrectly delete tracks when the
    // robot walks or turns away from a legitimate ball.
    let length = field_dimensions.length + 2.0 * field_dimensions.border_strip_width;
    let width = field_dimensions.width + 2.0 * field_dimensions.border_strip_width;
    ball.position.coords().norm_squared() <= length * length + width * width
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
    (0.0..camera_matrix.image_size.x()).contains(&position_in_image.x())
        && (0.0..camera_matrix.image_size.y()).contains(&position_in_image.y())
}

#[cfg(test)]
mod tests {
    use linear_algebra::point;
    use nalgebra::vector;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    use super::*;

    fn horizontal_test_camera() -> CameraMatrix {
        // Camera one metre above Ground, looking along +x; pixel y increases down.
        let rotation = nalgebra::Rotation3::from_matrix_unchecked(nalgebra::Matrix3::new(
            0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0,
        ));
        let ground_to_camera = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.0, 1.0, 0.0),
            nalgebra::UnitQuaternion::from_rotation_matrix(&rotation),
        );
        CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            linear_algebra::vector![640.0, 544.0],
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::wrap(ground_to_camera),
        )
    }

    fn test_ball_detection(center: linear_algebra::Point2<Pixel>) -> Object<RobocupObjectLabel> {
        Object {
            label: RobocupObjectLabel::Ball,
            bounding_box: types::bounding_box::BoundingBox {
                area: geometry::rectangle::Rectangle {
                    min: center - linear_algebra::vector![2.0, 2.0],
                    max: center + linear_algebra::vector![2.0, 2.0],
                },
                confidence: 0.9,
            },
        }
    }

    #[test]
    fn above_horizon_and_excessively_distant_percepts_never_spawn_hypotheses() {
        let camera = horizontal_test_camera();
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = BallFilterParameters::default();
        parameters.maximum_camera_matrix_age = Duration::from_millis(20);
        parameters.maximum_detection_distance = 15.0;
        parameters.noise.detection_noise.inner.fill(0.05);
        parameters.noise.initial_covariance.fill(1.0);
        let above_horizon = point![320.0, 260.0];
        assert!(camera.is_above_horizon(above_horizon, dimensions.ball_radius));
        let too_far = camera
            .ground_with_z_to_pixel(point![20.0, 0.0], dimensions.ball_radius)
            .unwrap();
        let detections = [
            test_ball_detection(above_horizon),
            test_ball_detection(too_far),
        ];
        let mut tracker = Tracker::default();
        let time = Time::from_nanos(40_000_000);
        let percepts = tracker
            .advance(
                time,
                None,
                Some(&detections),
                Some(&TimeWrapper {
                    time,
                    inner: camera.clone(),
                }),
                &parameters,
                &dimensions,
            )
            .unwrap();
        assert!(percepts.is_empty());
        assert!(tracker.filter.hypotheses.is_empty());

        // A real ball across much of the field remains a valid measurement.
        let distant_but_plausible = camera
            .ground_with_z_to_pixel(point![10.0, 0.0], dimensions.ball_radius)
            .unwrap();
        let time = Time::from_nanos(80_000_000);
        let percepts = tracker
            .advance(
                time,
                None,
                Some(&[test_ball_detection(distant_but_plausible)]),
                Some(&TimeWrapper {
                    time,
                    inner: camera.clone(),
                }),
                &parameters,
                &dimensions,
            )
            .unwrap();
        assert_eq!(percepts.len(), 1);
        assert_eq!(tracker.filter.hypotheses.len(), 1);
        assert!((tracker.filter.hypotheses[0].position().position.x() - 10.0).abs() < 1e-3);

        // Legacy baselines with the range gate disabled retain finite far percepts.
        parameters.maximum_detection_distance = 0.0;
        let percepts = project_detected_balls(
            Some(&[test_ball_detection(too_far)]),
            Some(&camera),
            &parameters,
            dimensions.ball_radius,
        )
        .unwrap();
        assert_eq!(percepts.len(), 1);
    }

    #[test]
    fn publication_correction_rejects_undersized_images_but_allows_airborne_size() {
        let camera = horizontal_test_camera();
        let radius = FieldDimensions::SPL_2025.ball_radius;
        let position = point![1.0, 0.2];
        let center = camera.ground_with_z_to_pixel(position, radius).unwrap();
        let depth = (camera.ground_to_camera * point![position.x(), position.y(), radius]).z();
        let expected = radius * camera.intrinsics.focals.x.min(camera.intrinsics.focals.y) / depth;
        for (scale, plausible) in [(0.5, false), (1.0, true), (2.0, true)] {
            let percept = BallPercept {
                percept_in_ground: MultivariateNormalDistribution {
                    mean: position.inner.coords,
                    covariance: Matrix2::identity(),
                },
                image_location: Circle {
                    center,
                    radius: expected * scale,
                },
            };
            assert_eq!(
                radius_consistency(&percept, Some(&camera), radius),
                Some(plausible)
            );
            assert_eq!(radius_consistency(&percept, None, radius), None);
        }
    }

    #[test]
    fn optional_radius_gate_accepts_size_uncertainty_and_rejects_inconsistent_boxes() {
        let camera = horizontal_test_camera();
        let radius = FieldDimensions::SPL_2025.ball_radius;
        let mut parameters = BallFilterParameters {
            maximum_detection_radius_ratio: 2.0,
            ..Default::default()
        };
        parameters.noise.detection_noise.inner.fill(0.05);
        for distance in [0.8, 2.0, 6.0] {
            let center = camera
                .ground_with_z_to_pixel(point![distance, 0.0], radius)
                .unwrap();
            let expected = camera.get_pixel_radius(radius, center).unwrap();
            for scale in [0.2, 0.75, 1.0, 1.5, 3.0] {
                let mut detection = test_ball_detection(center);
                let offset = linear_algebra::vector![expected * scale, expected * scale];
                detection.bounding_box.area.min = center - offset;
                detection.bounding_box.area.max = center + offset;
                let output =
                    project_detected_balls(Some(&[detection]), Some(&camera), &parameters, radius)
                        .unwrap();
                assert_eq!(output.len(), usize::from((0.5..=2.0).contains(&scale)));
            }
        }
    }

    #[test]
    fn radius_consistency_can_be_limited_to_near_geometry() {
        let camera = horizontal_test_camera();
        let radius = FieldDimensions::SPL_2025.ball_radius;
        let mut parameters = BallFilterParameters {
            maximum_detection_radius_ratio: 2.0,
            radius_consistency_maximum_distance: 1.5,
            ..Default::default()
        };
        parameters.noise.detection_noise.inner.fill(0.05);
        for distance in [0.8, 2.0, 6.0] {
            let center = camera
                .ground_with_z_to_pixel(point![distance, 0.0], radius)
                .unwrap();
            let expected = camera.get_pixel_radius(radius, center).unwrap();
            let mut detection = test_ball_detection(center);
            let offset = linear_algebra::vector![expected * 0.2, expected * 0.2];
            detection.bounding_box.area.min = center - offset;
            detection.bounding_box.area.max = center + offset;
            let output =
                project_detected_balls(Some(&[detection]), Some(&camera), &parameters, radius)
                    .unwrap();
            assert_eq!(output.len(), usize::from(distance > 1.5));
        }
    }

    #[test]
    fn malformed_boxes_and_nonfinite_projection_noise_are_rejected() {
        let camera = horizontal_test_camera();
        let radius = FieldDimensions::SPL_2025.ball_radius;
        let mut parameters = BallFilterParameters::default();
        parameters.noise.detection_noise.inner.fill(0.05);
        let center = camera
            .ground_with_z_to_pixel(point![2.0, 0.0], radius)
            .unwrap();
        let valid = test_ball_detection(center);
        let mut malformed = vec![];
        let mut detection = valid;
        detection.bounding_box.confidence = f32::NAN;
        malformed.push(detection);
        detection = valid;
        detection.bounding_box.area.min = detection.bounding_box.area.max;
        malformed.push(detection);
        detection = valid;
        detection.bounding_box.area.min = point![f32::INFINITY, 0.0];
        malformed.push(detection);
        detection = valid;
        detection.bounding_box.area.min =
            detection.bounding_box.area.max + linear_algebra::vector![1.0, 1.0];
        malformed.push(detection);
        assert!(
            project_detected_balls(Some(&malformed), Some(&camera), &parameters, radius)
                .unwrap()
                .is_empty()
        );
        for noise in [f32::NAN, f32::INFINITY, -1.0, f32::MAX] {
            parameters.noise.detection_noise.inner.fill(noise);
            assert!(
                project_detected_balls(Some(&[valid]), Some(&camera), &parameters, radius)
                    .unwrap()
                    .is_empty(),
                "noise {noise}"
            );
        }
        parameters.noise.detection_noise.inner.fill(0.05);
        let mut invalid_geometry = camera;
        invalid_geometry.intrinsics.focals.x = f32::NAN;
        assert!(
            project_detected_balls(Some(&[valid]), Some(&invalid_geometry), &parameters, radius)
                .is_none()
        );
    }

    #[test]
    fn camera_tolerance_is_symmetric_and_inclusive() {
        let frame = Time::from_nanos(100_000_000);
        let tolerance = Duration::from_millis(20);
        for stamp in [80_000_000, 100_000_000, 120_000_000] {
            assert!(camera_is_recent(frame, Time::from_nanos(stamp), tolerance));
        }
        for stamp in [79_999_999, 120_000_001] {
            assert!(!camera_is_recent(frame, Time::from_nanos(stamp), tolerance));
        }
    }

    #[test]
    fn zero_camera_tolerance_requires_matching_source_timestamps() {
        let frame = Time::from_nanos(100_000_000);
        assert!(camera_is_recent(frame, frame, Duration::ZERO));
        for stamp in [
            frame - Duration::from_nanos(1),
            frame + Duration::from_nanos(1),
        ] {
            assert!(!camera_is_recent(frame, stamp, Duration::ZERO));
        }
    }

    #[test]
    fn visibility_uses_camera_image_dimensions() {
        let camera = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            linear_algebra::vector![640.0, 544.0],
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::from_translation(0.0, 0.0, 1.0),
        );
        let position = camera
            .pixel_to_ground_with_z(point![320.0, 520.0], 0.105)
            .unwrap();
        let ball = BallPosition {
            position,
            velocity: linear_algebra::Vector2::zeros(),
            last_seen: Time::zero(),
        };
        assert!(is_visible_to_camera(&ball, &camera, 0.105));
        let shorter_camera = CameraMatrix {
            image_size: linear_algebra::vector![640.0, 480.0],
            ..camera
        };
        assert!(!is_visible_to_camera(&ball, &shorter_camera, 0.105));
    }

    #[test]
    fn uncertain_position_cannot_certify_a_clear_camera_miss() {
        let camera = horizontal_test_camera();
        let radius = FieldDimensions::SPL_2025.ball_radius;
        let mut parameters = BallFilterParameters {
            visibility_uncertainty_scale: 1.0,
            ..Default::default()
        };
        let mut hypothesis = BallHypothesis::new(
            MultivariateNormalDistribution {
                mean: nalgebra::vector![2.0, 0.0, 0.0, 0.0],
                covariance: Matrix4::identity() * 0.0001,
            },
            Time::zero(),
        );
        assert_eq!(
            hypothesis_visibility(&hypothesis, &camera, radius, Some(&[]), &[], &parameters),
            negative_evidence::Visibility::Visible
        );
        if let BallMode::Moving(state) = &mut hypothesis.mode {
            state.covariance *= 100_000.0;
        }
        assert_eq!(
            hypothesis_visibility(&hypothesis, &camera, radius, Some(&[]), &[], &parameters),
            negative_evidence::Visibility::Unknown
        );
        parameters.visibility_uncertainty_scale = 0.0;
        assert_eq!(
            hypothesis_visibility(&hypothesis, &camera, radius, Some(&[]), &[], &parameters),
            negative_evidence::Visibility::Visible
        );
    }

    #[test]
    fn uncertainty_tie_break_does_not_let_a_diffuse_track_steal_a_precise_match() {
        for (weight, matched_index) in [(0.0, 0), (1.0, 1)] {
            let parameters = BallFilterParameters {
                maximum_matching_cost: 1.0,
                association_uncertainty_weight: weight,
                hidden_validity_exponential_decay_factor: 1.0,
                ..Default::default()
            };
            let make = |x, variance| {
                BallHypothesis::new(
                    MultivariateNormalDistribution {
                        mean: nalgebra::vector![x, 0.0, 0.0, 0.0],
                        covariance: Matrix4::identity() * variance,
                    },
                    Time::zero(),
                )
            };
            let mut filter = BallFilter {
                hypotheses: vec![make(0.0, 100.0), make(1.02, 0.01)],
            };
            let percept = BallPercept {
                percept_in_ground: MultivariateNormalDistribution {
                    mean: vector![1.0, 0.0],
                    covariance: Matrix2::identity() * 0.001,
                },
                image_location: Circle::new(point![0.0, 0.0], 1.0),
            };
            let time = Time::from_nanos(40_000_000);
            advance_all_hypotheses(
                &mut filter,
                &mut AssignmentSolver::default(),
                time,
                &[percept],
                None,
                None,
                &[],
                &parameters,
                &FieldDimensions::SPL_2025,
            )
            .unwrap();
            assert_eq!(filter.hypotheses.len(), 2);
            assert_eq!(filter.hypotheses[matched_index].last_seen, time);
            assert_eq!(filter.hypotheses[1 - matched_index].last_seen, Time::zero());
        }
    }

    #[test]
    fn distant_percept_cannot_capture_an_uncertain_track_with_distance_gate() {
        for (distance_gate, expected_tracks) in [(0.0, 1), (0.3, 2), (2.0, 1)] {
            let parameters = BallFilterParameters {
                hidden_validity_exponential_decay_factor: 1.0,
                maximum_matching_cost: 1.0,
                maximum_matching_distance: distance_gate,
                ..Default::default()
            };
            let mut parent = BallHypothesis::new(
                MultivariateNormalDistribution {
                    mean: nalgebra::Vector4::zeros(),
                    covariance: Matrix4::identity() * 100.0,
                },
                Time::from_nanos(40_000_000),
            );
            parent.validity = 10.0;
            let mut filter = BallFilter {
                hypotheses: vec![parent],
            };
            let percept = BallPercept {
                percept_in_ground: MultivariateNormalDistribution {
                    mean: vector![1.0, 0.0],
                    covariance: Matrix2::identity() * 0.001,
                },
                image_location: Circle::new(point![0.0, 0.0], 1.0),
            };
            advance_all_hypotheses(
                &mut filter,
                &mut AssignmentSolver::default(),
                Time::from_nanos(80_000_000),
                &[percept],
                None,
                None,
                &[],
                &parameters,
                &FieldDimensions::SPL_2025,
            )
            .unwrap();
            assert_eq!(filter.hypotheses.len(), expected_tracks);
            if expected_tracks == 2 {
                assert_eq!(filter.hypotheses[0].position().position, point![0.0, 0.0]);
                assert_eq!(filter.hypotheses[0].last_seen, Time::from_nanos(40_000_000));
                assert_eq!(filter.hypotheses[1].position().position, point![1.0, 0.0]);
            }
        }
    }

    #[test]
    fn rejected_pairs_cannot_steal_a_valid_assignment() {
        // A full assignment would prefer the two 0.2-cost pairs, then reject
        // both. The only feasible observation belongs to the first track.
        let scores = ndarray::array![[-0.1, -0.2], [-0.2, -100.0]];
        let scores = gated_assignment_scores(&scores, 0.15);
        let mut solver = AssignmentSolver::default();
        let assignment = solver.solve(scores.view(), Objective::Maximize).unwrap();
        assert_eq!(assignment[0], Some(0));
        assert!(assignment[1].unwrap() >= 2);
    }

    #[test]
    fn nearby_rejected_association_inherits_only_when_parent_is_unmatched() {
        for parent_seen in [false, true] {
            let parameters = BallFilterParameters {
                hidden_validity_exponential_decay_factor: 1.0,
                maximum_matching_cost: 0.25,
                nearby_spawn_validity_factor: Some(0.5),
                ..Default::default()
            };
            let mut parent = BallHypothesis::new(
                MultivariateNormalDistribution {
                    mean: nalgebra::Vector4::zeros(),
                    covariance: Matrix4::identity() * 0.0001,
                },
                Time::from_nanos(40_000_000),
            );
            parent.validity = 10.0;
            let percept = |x| BallPercept {
                percept_in_ground: MultivariateNormalDistribution {
                    mean: vector![x, 0.0],
                    covariance: Matrix2::identity() * 0.0001,
                },
                image_location: Circle::new(point![0.0, 0.0], 1.0),
            };
            let percepts = if parent_seen {
                vec![percept(0.0), percept(0.1)]
            } else {
                vec![percept(0.1)]
            };
            let mut filter = BallFilter {
                hypotheses: vec![parent],
            };
            advance_all_hypotheses(
                &mut filter,
                &mut AssignmentSolver::default(),
                Time::from_nanos(80_000_000),
                &percepts,
                None,
                None,
                &[],
                &parameters,
                &FieldDimensions::SPL_2025,
            )
            .unwrap();
            assert_eq!(filter.hypotheses.len(), 2);
            assert_eq!(
                filter.hypotheses[1].validity,
                if parent_seen { 1.0 } else { 2.0 }
            );
            assert_eq!(
                filter.hypotheses[0].validity,
                if parent_seen { 11.0 } else { 9.0 }
            );
            assert_eq!(filter.hypotheses[1].position().position, point![0.1, 0.0]);
        }
    }

    #[test]
    fn speed_transition_can_rest_a_quiet_uncertain_ball_without_stopping_fast_motion() {
        for (threshold, speed, resting) in [(0.0, 0.0, false), (0.1, 0.01, true), (0.1, 1.0, false)]
        {
            let parameters = BallFilterParameters {
                resting_velocity_threshold: threshold,
                log_likelihood_of_zero_velocity_threshold: f32::INFINITY,
                ..Default::default()
            };
            let mut filter = BallFilter {
                hypotheses: vec![BallHypothesis::new(
                    MultivariateNormalDistribution {
                        mean: nalgebra::vector![1.0, 0.0, speed, 0.0],
                        covariance: Matrix4::identity() * 50.0,
                    },
                    Time::zero(),
                )],
            };
            predict_hypotheses_from_odometry(
                &mut filter,
                Time::zero(),
                Pose2::new(point![0.0, 0.0], 0.0),
                &mut None,
                &mut None,
                &parameters,
            );
            assert_eq!(
                matches!(filter.hypotheses[0].mode, BallMode::Resting(_)),
                resting
            );
            assert_eq!(filter.hypotheses[0].position().position, point![1.0, 0.0]);
            assert_eq!(filter.hypotheses[0].validity, 1.0);
        }
    }

    #[test]
    fn reacquisition_gate_preserves_prior_and_requires_an_observation_gap() {
        for (gate, millis, should_branch) in [(0.0, 200, false), (0.1, 40, false), (0.1, 200, true)]
        {
            let mut parameters = BallFilterParameters::default();
            parameters.reacquisition_matching_distance = gate;
            parameters.maximum_matching_cost = 1.0;
            parameters.velocity_decay_factor = 0.998;
            parameters.hidden_validity_exponential_decay_factor = 1.0;
            parameters.validity_discard_threshold = 0.2;
            parameters.noise.initial_covariance.fill(1.0);
            let mut old = BallHypothesis::new(
                MultivariateNormalDistribution {
                    mean: nalgebra::vector![1.0, 0.0, 0.0, 0.0],
                    covariance: Matrix4::identity() * 10.0,
                },
                Time::zero(),
            );
            old.validity = 5.0;
            let mut filter = BallFilter {
                hypotheses: vec![old],
            };
            let percept = BallPercept {
                percept_in_ground: MultivariateNormalDistribution {
                    mean: nalgebra::vector![1.5, 0.0],
                    covariance: Matrix2::identity() * 0.01,
                },
                image_location: Circle::new(point![0.0, 0.0], 8.0),
            };
            advance_all_hypotheses(
                &mut filter,
                &mut AssignmentSolver::default(),
                Time::from_nanos(millis * 1_000_000),
                &[percept],
                None,
                Some(&[]),
                &[],
                &parameters,
                &FieldDimensions::SPL_2025,
            )
            .unwrap();
            assert_eq!(filter.hypotheses.len(), if should_branch { 2 } else { 1 });
            if should_branch {
                assert_eq!(filter.hypotheses[0].last_seen, Time::zero());
                assert_eq!(filter.hypotheses[0].position().position, point![1.0, 0.0]);
                assert_eq!(filter.hypotheses[1].validity, 1.0);
                assert_eq!(filter.hypotheses[1].position().position, point![1.5, 0.0]);
            } else {
                assert_eq!(
                    filter.hypotheses[0].last_seen,
                    Time::from_nanos(millis * 1_000_000)
                );
                assert!(filter.hypotheses[0].position().position.x() > 1.49);
            }
        }
    }

    #[test]
    fn unseen_kick_spawns_at_detection_without_destroying_the_old_track() {
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = BallFilterParameters::default();
        parameters.hidden_validity_exponential_decay_factor = 1.0;
        parameters.maximum_matching_cost = 0.25;
        parameters.maximum_matching_cost_validity_penalty_factor = 0.14;
        parameters.validity_discard_threshold = 0.2;
        parameters.validity_output_threshold = 0.5;
        parameters.maximum_number_of_hypotheses = 15;
        parameters.hypothesis_timeout = Duration::from_secs(20);
        parameters.noise.initial_covariance.fill(0.1);
        let old_track = BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::Vector4::zeros(),
                covariance: Matrix4::identity() * 0.01,
            }),
            last_seen: Time::zero(),
            validity: 2.0,
            motion_evidence: None,
            negative_evidence: None,
            validity_decay_evidence: None,
            leadership_evidence: None,
            last_observation_size_plausible: None,
            merge_observation_start: None,
        };
        let mut filter = BallFilter {
            hypotheses: vec![old_track],
        };
        let mut solver = AssignmentSolver::default();
        let percept = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: vector![3.0, 0.0],
                covariance: Matrix2::identity(),
            },
            image_location: Circle::new(point![0.0, 0.0], 1.0),
        };
        let time = Time::from_nanos(500_000_000);
        advance_all_hypotheses(
            &mut filter,
            &mut solver,
            time,
            &[percept],
            None,
            None,
            &[],
            &parameters,
            &dimensions,
        )
        .unwrap();
        remove_invalid_and_merge_hypotheses(&mut filter, time, &parameters, &dimensions);

        assert_eq!(filter.hypotheses.len(), 2);
        assert_eq!(filter.hypotheses[0].validity, 2.0);
        assert_eq!(filter.hypotheses[0].last_seen, Time::zero());
        assert_eq!(filter.hypotheses[1].position().position, point![3.0, 0.0]);
        assert!(
            filter
                .best_hypothesis(parameters.validity_output_threshold)
                .is_some()
        );

        // A second and third consistent observation promote the new location,
        // while a single unrelated false percept cannot erase the old model.
        for nanos in [540_000_000, 580_000_000] {
            let time = Time::from_nanos(nanos);
            advance_all_hypotheses(
                &mut filter,
                &mut solver,
                time,
                &[percept],
                None,
                None,
                &[],
                &parameters,
                &dimensions,
            )
            .unwrap();
            remove_invalid_and_merge_hypotheses(&mut filter, time, &parameters, &dimensions);
        }
        let best = filter
            .best_hypothesis(parameters.validity_output_threshold)
            .unwrap();
        assert_eq!(best.position().position, point![3.0, 0.0]);
        assert_eq!(best.last_seen, Time::from_nanos(580_000_000));
        assert_eq!(filter.hypotheses.len(), 2);
    }

    #[test]
    fn coherent_kick_observations_replace_high_validity_resting_track_promptly() {
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = BallFilterParameters::default();
        parameters.hidden_validity_exponential_decay_factor = 0.9997;
        parameters.maximum_matching_cost = 0.25;
        parameters.validity_discard_threshold = 0.2;
        parameters.validity_output_threshold = 0.5;
        parameters.maximum_number_of_hypotheses = 15;
        parameters.hypothesis_timeout = Duration::from_secs(20);
        parameters.noise.initial_covariance = vector![0.5, 0.5, 40.0, 40.0];
        let mut filter = BallFilter {
            hypotheses: vec![BallHypothesis {
                mode: BallMode::Resting(MultivariateNormalDistribution {
                    mean: vector![0.0, 0.0],
                    covariance: Matrix2::identity() * 0.01,
                }),
                last_seen: Time::zero(),
                validity: 25.0,
                motion_evidence: None,
                negative_evidence: None,
                validity_decay_evidence: None,
                leadership_evidence: None,
                last_observation_size_plausible: None,
                merge_observation_start: None,
            }],
        };
        let mut solver = AssignmentSolver::default();
        for index in 0..4 {
            let time = Time::from_nanos(500_000_000 + index * 40_000_000);
            if index > 0 {
                filter.predict(
                    Duration::from_millis(40),
                    Isometry2::identity(),
                    1.0,
                    Matrix4::identity() * 0.005,
                    Matrix2::identity() * 0.001,
                    f32::INFINITY,
                );
            }
            let percept = BallPercept {
                percept_in_ground: MultivariateNormalDistribution {
                    mean: vector![3.0 + index as f32 * 0.12, 0.0],
                    covariance: Matrix2::identity() * 0.01,
                },
                image_location: Circle::new(point![0.0, 0.0], 1.0),
            };
            advance_all_hypotheses(
                &mut filter,
                &mut solver,
                time,
                &[percept],
                None,
                None,
                &[],
                &parameters,
                &dimensions,
            )
            .unwrap();
            remove_invalid_and_merge_hypotheses(&mut filter, time, &parameters, &dimensions);
            let best = filter
                .best_hypothesis(parameters.validity_output_threshold)
                .unwrap();
            if index == 0 {
                assert_eq!(
                    best.position().position,
                    point![0.0, 0.0],
                    "one false percept must not steal selection"
                );
            }
        }
        let best = filter
            .best_hypothesis(parameters.validity_output_threshold)
            .unwrap();
        assert!(best.position().position.x() > 3.2);
        assert_eq!(best.last_seen, Time::from_nanos(620_000_000));
        let old = filter
            .hypotheses
            .iter()
            .find(|track| track.last_seen == Time::zero())
            .unwrap();
        assert!(
            old.validity > 24.9,
            "selection must preserve hidden-track confidence"
        );
        assert!(matches!(old.mode, BallMode::Resting(_)));
        assert_eq!(filter.hypotheses.len(), 2);

        // Capping rank must not shorten the unchanged time-based track lifetime.
        remove_invalid_and_merge_hypotheses(
            &mut filter,
            Time::from_nanos(19_999_999_999),
            &parameters,
            &dimensions,
        );
        assert!(
            filter
                .hypotheses
                .iter()
                .any(|track| track.last_seen == Time::zero())
        );
        remove_invalid_and_merge_hypotheses(
            &mut filter,
            Time::from_nanos(20_000_000_000),
            &parameters,
            &dimensions,
        );
        assert!(
            !filter
                .hypotheses
                .iter()
                .any(|track| track.last_seen == Time::zero())
        );
    }

    #[test]
    fn hypothesis_update_matching() {
        let hypothesis1 = BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::vector![0.0, 1.0, 0.0, 0.0],
                covariance: Matrix4::identity(),
            }),
            last_seen: Time::zero(),
            validity: 0.0,
            motion_evidence: None,
            negative_evidence: None,
            validity_decay_evidence: None,
            leadership_evidence: None,
            last_observation_size_plausible: None,
            merge_observation_start: None,
        };
        let hypothesis2 = BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::vector![0.0, -1.0, 0.0, 0.0],
                covariance: Matrix4::identity(),
            }),
            last_seen: Time::zero(),
            validity: 0.0,
            motion_evidence: None,
            negative_evidence: None,
            validity_decay_evidence: None,
            leadership_evidence: None,
            last_observation_size_plausible: None,
            merge_observation_start: None,
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
        let mut solver = AssignmentSolver::default();
        let assignment = solver.solve(costs.view(), Objective::Maximize).unwrap();

        let percept_of_hypothesis1 = assignment[0].unwrap();
        assert_eq!(percept_of_hypothesis1, 0);

        let percept_of_hypothesis2 = assignment[1].unwrap();
        assert_eq!(percept_of_hypothesis2, 1);

        assert_eq!(assignment.len(), 2);
        assert_eq!(assignment.iter().flatten().count(), 2);
    }
}

#[cfg(test)]
mod odometry_pose_tests {
    use coordinate_systems::Odometry;
    use linear_algebra::{Pose2, point};
    use ros_z::time::Time;
    use types::parameters::BallFilterParameters;

    use super::*;

    #[test]
    fn odometry_pose_prediction_updates_last_pose() {
        let mut ball_filter = BallFilter::default();
        let mut last_odometry = None;
        let mut last_prediction_time = None;
        let parameters = BallFilterParameters::default();

        predict_hypotheses_from_odometry(
            &mut ball_filter,
            Time::from_nanos(1_000_000_000),
            Pose2::<Odometry>::new(point![<Odometry>, 0.0, 0.0], 0.0),
            &mut last_odometry,
            &mut last_prediction_time,
            &parameters,
        );

        predict_hypotheses_from_odometry(
            &mut ball_filter,
            Time::from_nanos(1_010_000_000),
            Pose2::<Odometry>::new(point![<Odometry>, 1.0, 0.0], 0.0),
            &mut last_odometry,
            &mut last_prediction_time,
            &parameters,
        );

        assert!(last_odometry.is_some());
        assert_eq!(last_prediction_time, Some(Time::from_nanos(1_010_000_000)));
    }
}

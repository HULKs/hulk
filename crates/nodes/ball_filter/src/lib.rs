use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use ball_tracker::{FieldBounds, Measurement, TrackSnapshot, Tracker, TrackerParameters};
use color_eyre::Result;
use coordinate_systems::{Ground, Odometry, Pixel};
use geometry::circle::Circle;
use linear_algebra::{IntoFramed, Isometry2, Point2, Pose2};
use nalgebra::{Matrix2, Matrix4};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{context::Context, prelude::*, qos::QosDurability, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use tokio::task::block_in_place;
use types::{
    ball_detection::BallPercept,
    ball_position::BallPosition,
    ball_tracking::{BallTrack, TrackerDebugState},
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Object, RobocupObjectLabel},
    odometry,
    parameters::BallFilterParameters,
    time_wrapper::TimeWrapper,
};

struct BallFilterOutput {
    ball_percepts: Vec<BallPercept>,
    tracker_output: Option<TrackerDerivedOutput>,
}

struct TrackerDerivedOutput {
    tracks: Vec<BallTrack<Ground>>,
    primary_ball: Option<BallTrack<Ground>>,
    ball_position: Option<BallPosition<Ground>>,
    debug_state: TrackerDebugState,
    filtered_balls_in_image: Vec<Circle<Pixel>>,
}

struct TrackerPredictionUpdate {
    delta_time: Duration,
    last_to_current: Isometry2<Ground, Ground>,
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

    let tracks_pub = node
        .publisher::<Vec<BallTrack<Ground>>>("ball_filter/tracks")
        .build()
        .await?;
    let primary_ball_pub = node
        .publisher::<Option<BallTrack<Ground>>>("ball_filter/primary_ball")
        .build()
        .await?;
    let debug_state_pub = node
        .publisher::<TrackerDebugState>("ball_filter/debug_state")
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

    let mut tracker = Tracker::new();
    let mut last_odometry: Option<Pose2<Odometry>> = None;
    let mut last_prediction_time = None;

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let future_map_item = future_map.recv().await?;

        let Some(field_dimensions) = field_dimensions_sub.get_latest() else {
            continue;
        };

        let output = block_in_place(|| {
            let mut ball_percepts = Vec::new();
            let mut latest_output = None;

            for (time, (odometry_pose, detected_objects)) in future_map_item.persistent {
                if !has_detection_frame(detected_objects.as_ref()) {
                    continue;
                }

                let Some(prediction_update) = tracker_prediction_update(
                    time,
                    odometry_pose,
                    &mut last_odometry,
                    &mut last_prediction_time,
                ) else {
                    continue;
                };

                let timed_camera_matrix = camera_matrix_cache.get_nearest(time);
                let camera_matrix = timed_camera_matrix
                    .as_ref()
                    .map(|camera_matrix| &camera_matrix.inner);
                let projected_balls = project_detected_balls(
                    detected_objects
                        .as_ref()
                        .map(|detected_objects| detected_objects.inner.as_slice()),
                    camera_matrix,
                    parameters,
                    field_dimensions.ball_radius,
                )
                .unwrap_or_default();
                let projected_balls = deduplicate_measurements(
                    projected_balls,
                    parameters.duplicate_measurement_distance,
                );

                ball_percepts.extend(projected_balls.iter().map(|(percept, _)| *percept));
                let measurements: Vec<_> = projected_balls
                    .iter()
                    .map(|(percept, confidence)| Measurement {
                        position: Point2::from(percept.percept_in_ground.mean),
                        covariance: percept.percept_in_ground.covariance,
                        confidence: *confidence,
                        image_location: Some(percept.image_location),
                    })
                    .collect();

                latest_output = Some((
                    time,
                    tracker.update(
                        prediction_update.delta_time,
                        prediction_update.last_to_current,
                        &measurements,
                        &tracker_parameters_from_ros(parameters),
                        FieldBounds {
                            x_limit: field_dimensions.length / 2.0,
                            y_limit: field_dimensions.width / 2.0,
                        },
                        |position| {
                            camera_matrix.is_some_and(|camera_matrix| {
                                is_position_visible_to_camera(
                                    position,
                                    camera_matrix,
                                    field_dimensions.ball_radius,
                                )
                            })
                        },
                    ),
                ));
            }

            let tracker_output = if let Some((tracker_update_time, tracker_output)) = latest_output
            {
                let ball_radius = field_dimensions.ball_radius;
                let timed_camera_matrix = camera_matrix_cache.get_nearest(tracker_update_time);
                let camera_matrix = timed_camera_matrix
                    .as_ref()
                    .map(|timed_camera_matrix| &timed_camera_matrix.inner);

                Some(tracker_derived_output_from_update(
                    tracker_output,
                    tracker_update_time,
                    camera_matrix,
                    ball_radius,
                ))
            } else {
                None
            };

            ball_filter_output_from_parts(ball_percepts, tracker_output)
        });

        let Some(output) = output else {
            continue;
        };

        ball_percepts_pub.publish(&output.ball_percepts).await?;
        if let Some(tracker_output) = output.tracker_output {
            tracks_pub.publish(&tracker_output.tracks).await?;
            primary_ball_pub
                .publish(&tracker_output.primary_ball)
                .await?;
            debug_state_pub.publish(&tracker_output.debug_state).await?;
            filtered_balls_in_image_pub
                .publish(&tracker_output.filtered_balls_in_image)
                .await?;
            ball_position_pub
                .publish(&tracker_output.ball_position)
                .await?;
        }
    }
}

fn has_detection_frame<T>(detected_objects: Option<&TimeWrapper<T>>) -> bool {
    detected_objects.is_some()
}

fn ball_filter_output_from_parts(
    ball_percepts: Vec<BallPercept>,
    tracker_output: Option<TrackerDerivedOutput>,
) -> Option<BallFilterOutput> {
    if ball_percepts.is_empty() && tracker_output.is_none() {
        return None;
    }

    Some(BallFilterOutput {
        ball_percepts,
        tracker_output,
    })
}

fn tracker_prediction_update(
    time: Time,
    odometry_pose: Option<Pose2<Odometry>>,
    last_odometry: &mut Option<Pose2<Odometry>>,
    last_prediction_time: &mut Option<Time>,
) -> Option<TrackerPredictionUpdate> {
    let last_to_current = match (last_odometry.as_ref(), odometry_pose) {
        (Some(previous_odometry), Some(current_odometry)) => {
            odometry::previous_to_current(*previous_odometry, current_odometry)
        }
        _ => Isometry2::identity(),
    };
    let delta_time =
        last_prediction_time.map_or(Duration::ZERO, |last_time| time.duration_since(last_time));

    if let Some(current_odometry) = odometry_pose {
        *last_odometry = Some(current_odometry);
    }
    *last_prediction_time = Some(time);

    Some(TrackerPredictionUpdate {
        delta_time,
        last_to_current,
    })
}

fn tracker_parameters_from_ros(parameters: &BallFilterParameters) -> TrackerParameters {
    TrackerParameters {
        rolling_velocity_decay: parameters.rolling_velocity_decay,
        mode_transition_static_to_rolling: parameters.mode_transition_static_to_rolling,
        mode_transition_rolling_to_static: parameters.mode_transition_rolling_to_static,
        max_ball_speed: parameters.max_ball_speed,
        gating_chi_square: parameters.gating_chi_square,
        probability_of_detection_visible: parameters.probability_of_detection_visible,
        probability_of_detection_hidden: parameters.probability_of_detection_hidden,
        survival_probability: parameters.survival_probability,
        clutter_log_likelihood: parameters.clutter_log_likelihood,
        birth_log_likelihood: parameters.birth_log_likelihood,
        max_assignments_per_hypothesis: parameters.max_assignments_per_hypothesis,
        birth_existence_probability: parameters.birth_existence_probability,
        confirm_existence_threshold: parameters.confirm_existence_threshold,
        delete_existence_threshold: parameters.delete_existence_threshold,
        minimum_confirming_hits: parameters.minimum_confirming_hits,
        tentative_timeout: parameters.tentative_timeout,
        confirmed_timeout: parameters.confirmed_timeout,
        stale_timeout: parameters.stale_timeout,
        max_global_hypotheses: parameters.max_global_hypotheses,
        max_tracks_per_hypothesis: parameters.max_tracks_per_hypothesis,
        hypothesis_prune_log_weight: parameters.hypothesis_prune_log_weight,
        merge_distance: parameters.merge_distance,
        static_process_noise: Matrix2::from_diagonal(&parameters.noise.static_process_noise),
        rolling_process_noise: Matrix4::from_diagonal(&parameters.noise.rolling_process_noise),
        initial_position_covariance: parameters.noise.initial_position_covariance,
        initial_velocity_covariance: parameters.noise.initial_velocity_covariance,
    }
}

fn track_to_message(track: &TrackSnapshot, now: Time) -> BallTrack<Ground> {
    BallTrack {
        id: track.id,
        position: track.position,
        velocity: track.velocity,
        covariance: track.covariance,
        existence_probability: track.existence_probability,
        status: track.status,
        last_seen: now - track.last_seen_age,
    }
}

fn track_to_ball_position(track: &BallTrack<Ground>) -> BallPosition<Ground> {
    BallPosition {
        position: track.position,
        velocity: track.velocity,
        last_seen: track.last_seen,
    }
}

fn tracker_derived_output_from_update(
    tracker_output: ball_tracker::TrackerOutput,
    tracker_update_time: Time,
    camera_matrix: Option<&CameraMatrix>,
    ball_radius: f32,
) -> TrackerDerivedOutput {
    let tracks: Vec<_> = tracker_output
        .tracks
        .iter()
        .map(|track| track_to_message(track, tracker_update_time))
        .collect();
    let primary_ball = tracker_output
        .primary_track
        .as_ref()
        .map(|track| track_to_message(track, tracker_update_time));
    let ball_position = primary_ball.as_ref().map(track_to_ball_position);
    let filtered_balls_in_image = camera_matrix
        .map(|camera_matrix| project_to_image(&tracks, camera_matrix, ball_radius))
        .unwrap_or_default();

    TrackerDerivedOutput {
        tracks,
        primary_ball,
        ball_position,
        debug_state: tracker_output.debug,
        filtered_balls_in_image,
    }
}

fn deduplicate_measurements(
    percepts: Vec<(BallPercept, f32)>,
    duplicate_distance: f32,
) -> Vec<(BallPercept, f32)> {
    let mut deduplicated = Vec::<(BallPercept, f32)>::new();
    for (percept, confidence) in percepts {
        if deduplicated.iter().any(|(existing, _)| {
            (existing.percept_in_ground.mean - percept.percept_in_ground.mean).norm()
                < duplicate_distance
        }) {
            continue;
        }
        deduplicated.push((percept, confidence));
    }
    deduplicated
}

fn project_detected_balls(
    detections: Option<&[Object<RobocupObjectLabel>]>,
    camera_matrix: Option<&CameraMatrix>,
    parameters: &BallFilterParameters,
    ball_radius: f32,
) -> Option<Vec<(BallPercept, f32)>> {
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
                let confidence = detection.bounding_box.confidence;
                if confidence < parameters.ball_confidence_threshold {
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

                Some((
                    BallPercept {
                        percept_in_ground: MultivariateNormalDistribution {
                            mean: position.inner.coords,
                            covariance: projected_covariance,
                        },
                        image_location: circle,
                    },
                    confidence,
                ))
            })
            .collect(),
    )
}

fn project_to_image(
    filtered_balls: &[BallTrack<Ground>],
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

fn is_position_visible_to_camera(
    position: linear_algebra::Point2<Ground>,
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
) -> bool {
    let position_in_image = match camera_matrix.ground_with_z_to_pixel(position, ball_radius) {
        Ok(position_in_image) => position_in_image,
        Err(_) => return false,
    };
    (0.0..640.0).contains(&position_in_image.x()) && (0.0..480.0).contains(&position_in_image.y())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use linear_algebra::point;
    use nalgebra::{Vector2, Vector4};
    use types::parameters::{BallFilterNoise, BallFilterParameters};

    use super::*;

    fn parameters() -> BallFilterParameters {
        BallFilterParameters {
            ball_confidence_threshold: 0.5,
            duplicate_measurement_distance: 0.08,
            rolling_velocity_decay: 0.9,
            mode_transition_static_to_rolling: 0.1,
            mode_transition_rolling_to_static: 0.2,
            max_ball_speed: 6.0,
            gating_chi_square: 5.991,
            probability_of_detection_visible: 0.85,
            probability_of_detection_hidden: 0.05,
            survival_probability: 0.995,
            clutter_log_likelihood: -5.0,
            birth_log_likelihood: -2.0,
            max_assignments_per_hypothesis: 8,
            birth_existence_probability: 0.35,
            confirm_existence_threshold: 0.7,
            delete_existence_threshold: 0.1,
            minimum_confirming_hits: 2,
            tentative_timeout: Duration::from_secs(1),
            confirmed_timeout: Duration::from_secs(5),
            stale_timeout: Duration::from_secs(15),
            max_global_hypotheses: 16,
            max_tracks_per_hypothesis: 15,
            hypothesis_prune_log_weight: -8.0,
            merge_distance: 0.1,
            noise: BallFilterNoise {
                detection_noise: linear_algebra::vector![5.0, 5.0],
                static_process_noise: Vector2::new(0.001, 0.002),
                rolling_process_noise: Vector4::new(0.003, 0.004, 0.005, 0.006),
                initial_position_covariance: Vector2::new(0.5, 0.6),
                initial_velocity_covariance: Vector2::new(40.0, 41.0),
            },
        }
    }

    #[test]
    fn converts_ros_parameters_to_tracker_parameters() {
        let tracker_parameters = tracker_parameters_from_ros(&parameters());

        assert_eq!(tracker_parameters.rolling_velocity_decay, 0.9);
        assert_eq!(tracker_parameters.static_process_noise[(0, 0)], 0.001);
        assert_eq!(tracker_parameters.rolling_process_noise[(3, 3)], 0.006);
    }

    #[test]
    fn deduplicate_measurements_keeps_first_nearby_percept() {
        let first = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 2.0],
                covariance: Matrix2::identity(),
            },
            image_location: Circle::new(point![10.0, 20.0], 3.0),
        };
        let duplicate = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: nalgebra::vector![1.03, 2.0],
                covariance: Matrix2::identity(),
            },
            image_location: Circle::new(point![11.0, 20.0], 3.0),
        };
        let distant = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: nalgebra::vector![1.2, 2.0],
                covariance: Matrix2::identity(),
            },
            image_location: Circle::new(point![30.0, 20.0], 3.0),
        };

        let deduplicated =
            deduplicate_measurements(vec![(first, 0.7), (duplicate, 0.9), (distant, 0.8)], 0.08);

        assert_eq!(deduplicated.len(), 2);
        assert_eq!(deduplicated[0].1, 0.7);
        assert_eq!(deduplicated[1].1, 0.8);
    }
}

#[cfg(test)]
mod odometry_pose_tests {
    use coordinate_systems::Odometry;
    use linear_algebra::{Pose2, point};

    use super::*;

    #[test]
    fn tracker_prediction_update_advances_last_odometry_pose() {
        let mut last_odometry = None;
        let mut last_prediction_time = None;

        let first_update = tracker_prediction_update(
            Time::from_nanos(1_000_000_000),
            Some(Pose2::<Odometry>::new(point![<Odometry>, 0.0, 0.0], 0.0)),
            &mut last_odometry,
            &mut last_prediction_time,
        )
        .expect("first odometry pose should produce an update");

        let second_update = tracker_prediction_update(
            Time::from_nanos(1_010_000_000),
            Some(Pose2::<Odometry>::new(point![<Odometry>, 1.0, 0.0], 0.0)),
            &mut last_odometry,
            &mut last_prediction_time,
        )
        .expect("second odometry pose should produce an update");

        assert_eq!(first_update.delta_time, Duration::ZERO);
        assert_eq!(second_update.delta_time, Duration::from_millis(10));
        assert!(last_odometry.is_some());
        assert_eq!(last_prediction_time, Some(Time::from_nanos(1_010_000_000)));
    }

    #[test]
    fn output_keeps_percepts_without_tracker_update() {
        let percept = BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 2.0],
                covariance: Matrix2::identity(),
            },
            image_location: Circle::new(point![10.0, 20.0], 3.0),
        };

        let output = ball_filter_output_from_parts(vec![percept], None)
            .expect("percept-only output should still be published");

        assert_eq!(output.ball_percepts.len(), 1);
        assert_eq!(
            output.ball_percepts[0].percept_in_ground.mean,
            nalgebra::vector![1.0, 2.0]
        );
        assert!(output.tracker_output.is_none());
    }

    #[test]
    fn tracker_derived_output_uses_tracker_update_time() {
        let tracker_update_time = Time::from_nanos(1_000_000_000);
        let later_batch_time = Time::from_nanos(1_200_000_000);
        let track = TrackSnapshot {
            id: 1,
            position: point![1.0, 2.0],
            velocity: linear_algebra::vector![0.1, 0.2],
            covariance: Matrix2::identity(),
            existence_probability: 0.8,
            status: types::ball_tracking::TrackStatus::Confirmed,
            last_seen_age: Duration::from_millis(100),
        };

        let output = tracker_derived_output_from_update(
            ball_tracker::TrackerOutput {
                tracks: vec![track.clone()],
                primary_track: Some(track),
                debug: TrackerDebugState::default(),
            },
            tracker_update_time,
            None,
            0.05,
        );

        assert_eq!(output.tracks[0].last_seen, Time::from_nanos(900_000_000));
        assert_ne!(output.tracks[0].last_seen, later_batch_time);
        assert_eq!(
            output
                .ball_position
                .expect("primary ball should be converted")
                .last_seen,
            Time::from_nanos(900_000_000)
        );
    }
}

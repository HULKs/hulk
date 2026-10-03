use std::{boxed::Box, collections::BTreeMap, future::Future, pin::Pin};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use filtering::kalman_filter::KalmanFilter;
use geometry::rectangle::Rectangle;
use hsl_network_messages::PlayerNumber;
use itertools::{chain, iproduct};
use na::Matrix2;
use nalgebra as na;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{IntoFramed, Isometry2, Point2, Pose2, point};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use tokio::task::block_in_place;
use types::fall_detection::FallDetection;
use types::{
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Object, RobocupObjectLabel},
    obstacle_filter::Hypothesis,
    obstacles::{Obstacle, ObstacleKind},
    parameters::ObstacleFilterParameters,
    players::Players,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    world_state::PlayerState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObstacleFilter {
    hypotheses: Vec<Hypothesis>,
    last_primary_state: PrimaryState,
    ground_frame: Option<(Time, Pose2<Odometry>)>,
    last_detection_time: Option<Time>,
}

impl Default for ObstacleFilter {
    fn default() -> Self {
        Self {
            hypotheses: Vec::new(),
            last_primary_state: PrimaryState::Damping,
            ground_frame: None,
            last_detection_time: None,
        }
    }
}

struct ObstacleFilterOutput {
    time: Time,
    hypotheses: Vec<Hypothesis>,
    obstacles: Vec<Obstacle>,
}

#[derive(PartialEq)]
enum MeasurementKind {
    Own,
    NetworkRobot,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("obstacle_filter").build().await?;

    let parameters = node.bind_parameter_as::<ObstacleFilterParameters>("obstacle_filter")?;
    let field_dimensions_cache = node
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
        .with_stamp(|wrapper| wrapper.time)
        .build()
        .await?;
    let player_number_cache = node
        .subscriber::<PlayerNumber>("player_number")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let player_states_subscriber = node
        .subscriber::<Players<Option<TimeWrapper<PlayerState>>>>("player_states")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let primary_state_cache = node
        .subscriber::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let ground_to_field_cache = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .cache(200)
        .build()
        .await?;
    let fall_detection_cache = node
        .subscriber::<FallDetection>(types::fall_detection::FALL_DETECTION_TOPIC)
        .qos(ros_z::qos::QosProfile {
            reliability: ros_z::qos::QosReliability::BestEffort,
            ..Default::default()
        })
        .cache(10)
        .build()
        .await?;

    let mut detections = node
        .create_future_map_builder()
        .create_future_subscriber::<Pose2<Odometry>>("inputs/odometry", Duration::from_millis(1))
        .await?
        .create_future_subscriber::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>(
            "detected_objects",
            Duration::from_millis(50),
        )
        .await?
        .build();

    let obstacle_filter_hypotheses_pub = node
        .publisher::<Vec<Hypothesis>>("obstacle_filter_hypotheses")
        .build()
        .await?;
    let obstacles_pub = node.publisher::<Vec<Obstacle>>("obstacles").build().await?;

    let mut obstacle_filter = ObstacleFilter::default();
    let mut odometry_history = BTreeMap::new();
    let mut last_processed_player_state_times = Players::new(None);
    loop {
        tokio::select! {
            received_detections = detections.recv() => {
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let item = received_detections?;

                let outputs = block_in_place(|| {
                    let Some(field_dimensions) = field_dimensions_cache.get_latest() else {
                        return Vec::new();
                    };
                    let mut outputs = Vec::new();
                    for (detection_time, (odometry, detected_objects)) in item.persistent {
                        if let Some(pose) = odometry {
                            odometry_history.insert(detection_time, pose);
                            while odometry_history.len() > 1000 {
                                odometry_history.pop_first();
                            }
                        }
                        let camera_matrix = camera_matrix_cache.get_nearest(detection_time)
                            .filter(|camera| frame_stamp_is_recent(detection_time, camera.time));
                        let odometry = odometry_history.range(..=detection_time).next_back()
                            .filter(|(stamp, _)| detection_time.duration_since(**stamp) <= Duration::from_millis(2))
                            .map(|(_, pose)| *pose);
                        let ground_to_field = ground_to_field_cache.get_nearest_with_stamp(detection_time)
                            .filter(|(stamp, _)| frame_stamp_is_recent(detection_time, *stamp))
                            .map(|(_, pose)| pose);

                        let advanced = if let Some(detected_objects) = detected_objects {
                            obstacle_filter.process_detection(
                                detection_time,
                                parameters,
                                &detected_objects.inner,
                                camera_matrix.as_ref().map(|wrapper| &wrapper.inner),
                                odometry,
                            )
                        } else {
                            odometry.is_some_and(|pose| obstacle_filter.advance_ground_frame(
                                detection_time, pose, na::Vector2::zeros(),
                            ))
                        };
                        if !advanced { continue; }

                        let primary_state = primary_state_cache
                            .get_latest()
                            .map(|primary_state| *primary_state)
                            .unwrap_or_default();
                        let fall_detection = fall_detection_cache.get_latest();

                        let obstacles = obstacle_filter.compose_outputs(
                            detection_time,
                            parameters,
                            field_dimensions.as_ref(),
                            ground_to_field.as_ref().map(Arc::as_ref),
                            primary_state,
                            fall_detection.as_ref().map(Arc::as_ref),
                        );
                        outputs.push(ObstacleFilterOutput {
                            time: detection_time,
                            hypotheses: obstacle_filter.hypotheses.clone(),
                            obstacles,
                        });
                    }
                    outputs
                });

                for output in outputs {
                    obstacle_filter_hypotheses_pub
                        .publish_with_source_time(&output.hypotheses, output.time)
                        .await?;
                    obstacles_pub.publish_with_source_time(&output.obstacles, output.time).await?;
                }
            }
            received_player_states = player_states_subscriber.recv() => {
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let player_states = received_player_states?;
                let output = block_in_place(|| {
                    let (frame_time, _) = obstacle_filter.ground_frame?;
                    let ground_to_field = ground_to_field_cache.get_nearest_with_stamp(frame_time)
                        .filter(|(stamp, _)| frame_stamp_is_recent(frame_time, *stamp))?
                        .1;
                    let own_player_number = player_number_cache
                        .get_latest()
                        .map(|player_number| *player_number);
                    let network_player_states = new_network_player_states(
                        &player_states,
                        own_player_number,
                        &mut last_processed_player_state_times,
                    );
                    let mut processed_player_state = false;

                    for (player_state_time, player_state) in network_player_states {
                        obstacle_filter.process_network_player_state(
                            player_state_time,
                            parameters,
                            &player_state,
                            ground_to_field.as_ref(),
                        );
                        processed_player_state = true;
                    }

                    if !processed_player_state {
                        return None;
                    }

                    let field_dimensions = field_dimensions_cache.get_latest()?;
                    let primary_state = primary_state_cache
                        .get_latest()
                        .map(|primary_state| *primary_state)
                        .unwrap_or_default();
                    let fall_detection = fall_detection_cache.get_latest();

                    let obstacles = obstacle_filter.compose_outputs(
                        frame_time,
                        parameters,
                        field_dimensions.as_ref(),
                        Some(ground_to_field.as_ref()),
                        primary_state,
                        fall_detection.as_ref().map(Arc::as_ref),
                    );

                    Some(ObstacleFilterOutput {
                        time: frame_time,
                        hypotheses: obstacle_filter.hypotheses.clone(),
                        obstacles,
                    })
                });

                if let Some(output) = output {
                    obstacle_filter_hypotheses_pub
                        .publish_with_source_time(&output.hypotheses, output.time)
                        .await?;
                    obstacles_pub.publish_with_source_time(&output.obstacles, output.time).await?;
                }
            }
        }
    }
}

// This is a source-geometry tolerance, not message delivery latency. Never mark
// an old Ground frame as fresh simply because the filter processed it recently.
fn frame_stamp_is_recent(frame: Time, sample: Time) -> bool {
    frame.abs_diff(sample) <= Duration::from_millis(20)
}

impl ObstacleFilter {
    fn process_detection(
        &mut self,
        detection_time: Time,
        parameters: &ObstacleFilterParameters,
        detected_objects: &[Object<RobocupObjectLabel>],
        camera_matrix: Option<&CameraMatrix>,
        odometry: Option<Pose2<Odometry>>,
    ) -> bool {
        if self
            .last_detection_time
            .is_some_and(|previous| detection_time <= previous)
        {
            return false;
        }
        let Some(odometry) = odometry else {
            return false;
        };
        if !self.advance_ground_frame(detection_time, odometry, parameters.process_noise) {
            return false;
        }

        self.last_detection_time = Some(detection_time);
        if let Some(camera_matrix) = camera_matrix
            && parameters.use_detected_objects
        {
            let measured_object_positions =
                measured_object_positions(parameters, detected_objects, camera_matrix);

            for (kind, position, measurement_noise) in measured_object_positions {
                self.update_hypotheses_with_measurement(
                    position,
                    kind,
                    detection_time,
                    parameters.object_detection_measurement_matching_distance,
                    Matrix2::from_diagonal(&measurement_noise),
                    MeasurementKind::Own,
                );
            }
        }
        true
    }

    /// Follow every committed absolute-odometry sample, including camera silence.
    /// Fusion orders delayed images before newer persistent odometry; a detector
    /// at the current stamp is valid, independently of detector deduplication.
    /// Odometry-only calls supply zero noise: process noise is charged once per
    /// actual detector frame, preserving its existing calibration.
    /// This timestamp describes coordinates, not fresh visual confirmation;
    /// hypothesis.last_update still governs expiry during camera silence.
    fn advance_ground_frame(
        &mut self,
        time: Time,
        odometry: Pose2<Odometry>,
        process_noise: na::Vector2<f32>,
    ) -> bool {
        if let Some((previous_time, previous_pose)) = self.ground_frame {
            if time < previous_time {
                return false;
            }
            self.predict_hypotheses_with_odometry(
                types::odometry::previous_to_current(previous_pose, odometry).inner,
                Matrix2::from_diagonal(&process_noise),
            );
        }
        self.ground_frame = Some((time, odometry));
        true
    }

    fn process_network_player_state(
        &mut self,
        player_state_time: Time,
        parameters: &ObstacleFilterParameters,
        player_state: &PlayerState,
        ground_to_field: &Isometry2<Ground, Field>,
    ) {
        let player_position = measured_player_position(player_state, ground_to_field);
        self.update_hypotheses_with_measurement(
            player_position,
            ObstacleKind::Robot,
            player_state_time,
            parameters.network_robot_measurement_matching_distance,
            Matrix2::from_diagonal(&parameters.network_robot_measurement_noise),
            MeasurementKind::NetworkRobot,
        );
    }

    fn compose_outputs(
        &mut self,
        now: Time,
        parameters: &ObstacleFilterParameters,
        field_dimensions: &FieldDimensions,
        ground_to_field: Option<&Isometry2<Ground, Field>>,
        primary_state: PrimaryState,
        fall_detection: Option<&FallDetection>,
    ) -> Vec<Obstacle> {
        self.remove_hypotheses(
            now,
            parameters.hypothesis_timeout,
            parameters.hypothesis_merge_distance,
        );

        let became_unpenalized = self.last_primary_state == PrimaryState::Penalized
            && primary_state != PrimaryState::Penalized;

        let is_upright = fall_detection.is_some_and(|state| state.is_upright(now));

        self.last_primary_state = primary_state;

        if became_unpenalized {
            self.hypotheses.clear();
        }
        if !is_upright {
            self.hypotheses
                .retain(|obstacle| obstacle.obstacle_kind != ObstacleKind::Unknown);
        }

        let obstacles = self
            .hypotheses
            .iter()
            .filter(|hypothesis| {
                hypothesis.measurement_count > parameters.measurement_count_threshold
            })
            .map(|hypothesis| {
                let (radius_at_hip_height, radius_at_foot_height) = match hypothesis.obstacle_kind {
                    ObstacleKind::Robot => (
                        parameters.robot_obstacle_radius_at_hip_height,
                        parameters.robot_obstacle_radius_at_foot_height,
                    ),
                    ObstacleKind::Person => (
                        parameters.person_obstacle_radius,
                        parameters.person_obstacle_radius,
                    ),
                    ObstacleKind::GoalPost => (
                        parameters.goal_post_obstacle_radius,
                        parameters.goal_post_obstacle_radius,
                    ),
                    ObstacleKind::Unknown => (
                        parameters.unknown_obstacle_radius,
                        parameters.unknown_obstacle_radius,
                    ),
                    _ => panic!("Unexpected obstacle radius"),
                };
                Obstacle {
                    position: hypothesis.state.mean.framed().as_point(),
                    kind: hypothesis.obstacle_kind,
                    radius_at_hip_height,
                    radius_at_foot_height,
                }
            });
        let goal_posts = calculate_goal_post_positions(ground_to_field, field_dimensions);
        let goal_post_obstacles = goal_posts
            .into_iter()
            .map(|goal_post| Obstacle::goal_post(goal_post, parameters.goal_post_obstacle_radius));

        chain!(obstacles, goal_post_obstacles).collect()
    }

    fn predict_hypotheses_with_odometry(
        &mut self,
        last_odometry_to_current_odometry: na::Isometry2<f32>,
        process_noise: Matrix2<f32>,
    ) {
        for hypothesis in &mut self.hypotheses {
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
        detection_time: Time,
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
                ObstacleKind::Robot | ObstacleKind::GoalPost | ObstacleKind::Person => {
                    hypothesis.obstacle_kind
                }
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
        detection_time: Time,
        initial_covariance: Matrix2<f32>,
    ) {
        let new_hypothesis = Hypothesis {
            state: MultivariateNormalDistribution {
                mean: detected_position.inner.coords,
                covariance: initial_covariance,
            },
            obstacle_kind,
            measurement_count: 1,
            last_update: detection_time,
        };
        self.hypotheses.push(new_hypothesis);
    }

    fn remove_hypotheses(&mut self, now: Time, hypothesis_timeout: Duration, merge_distance: f32) {
        self.hypotheses
            .retain(|hypothesis| now.duration_since(hypothesis.last_update) < hypothesis_timeout);

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
                        ObstacleKind::Robot | ObstacleKind::GoalPost | ObstacleKind::Person => {
                            existing_hypothesis.obstacle_kind
                        }
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

fn measured_object_positions(
    parameters: &ObstacleFilterParameters,
    detected_objects: &[Object<RobocupObjectLabel>],
    camera_matrix: &CameraMatrix,
) -> impl Iterator<Item = (ObstacleKind, Point2<Ground>, na::Vector2<f32>)> {
    detected_objects.iter().filter_map(|detected_object| {
        let Object {
            label,
            bounding_box,
        } = detected_object;

        let (kind, measurement_noise, confidence_threshold) = match label {
            RobocupObjectLabel::GoalPost => (
                ObstacleKind::GoalPost,
                parameters.goal_post_measurement_noise,
                parameters.goal_post_confidence_threshold,
            ),
            RobocupObjectLabel::Robot => (
                ObstacleKind::Robot,
                parameters.robot_measurement_noise,
                parameters.robot_confidence_threshold,
            ),
            _ => return None,
        };

        if bounding_box.confidence < confidence_threshold {
            return None;
        }

        let bottom_center_position = {
            let Rectangle { min, max } = bounding_box.area;

            point![min.x() + (max.x() - min.x()) / 2.0, max.y()]
        };

        let obstacle_center: Point2<Ground> =
            camera_matrix.pixel_to_ground(bottom_center_position).ok()?;

        Some((kind, obstacle_center, measurement_noise))
    })
}

fn new_network_player_states(
    players: &Players<Option<TimeWrapper<PlayerState>>>,
    own_player_number: Option<PlayerNumber>,
    last_processed_player_state_times: &mut Players<Option<Time>>,
) -> Vec<(Time, PlayerState)> {
    let Some(own_player_number) = own_player_number else {
        return Vec::new();
    };

    players
        .iter()
        .filter_map(|(player_number, player_state)| {
            let player_state = player_state.as_ref()?;
            if last_processed_player_state_times[player_number]
                .is_some_and(|last_processed| player_state.time <= last_processed)
            {
                return None;
            }
            last_processed_player_state_times[player_number] = Some(player_state.time);

            if player_number == own_player_number {
                return None;
            }

            Some((player_state.time, player_state.inner))
        })
        .collect()
}

fn measured_player_position(
    player_state: &PlayerState,
    ground_to_field: &Isometry2<Ground, Field>,
) -> Point2<Ground> {
    let field_to_ground = ground_to_field.inverse();
    field_to_ground * player_state.pose.position()
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

#[cfg(test)]
mod tests {
    use super::*;
    use types::obstacles::ObstacleKind;

    #[test]
    fn odometry_only_updates_keep_current_coordinates_without_multiplying_noise() {
        let mut filter = ObstacleFilter::default();
        let start = Time::from_nanos(1_000_000_000);
        filter.advance_ground_frame(start, Pose2::default(), na::Vector2::zeros());
        filter.spawn_hypothesis(
            point![3.0, 1.0],
            ObstacleKind::Robot,
            start,
            Matrix2::identity(),
        );
        for tick in 1..=500 {
            let fraction = tick as f32 / 500.0;
            filter.advance_ground_frame(
                start + Duration::from_millis(tick * 2),
                Pose2::new(
                    point![fraction, 0.0],
                    fraction * std::f32::consts::FRAC_PI_2,
                ),
                na::Vector2::zeros(),
            );
        }
        assert!((filter.hypotheses[0].state.mean - na::vector![1.0, -2.0]).norm() < 1e-3);
        assert!((filter.hypotheses[0].state.covariance - Matrix2::identity()).norm() < 1e-3);
        assert_eq!(filter.hypotheses[0].last_update, start);
        let time = start + Duration::from_secs(1);
        let pose = filter.ground_frame.unwrap().1;
        let parameters = ObstacleFilterParameters {
            process_noise: na::vector![0.2, 0.2],
            ..Default::default()
        };
        assert!(filter.process_detection(time, &parameters, &[], None, Some(pose)));
        let covariance = filter.hypotheses[0].state.covariance;
        assert!((covariance - Matrix2::identity() * 1.2).norm() < 1e-3);
        assert!(!filter.process_detection(time, &parameters, &[], None, Some(pose)));
        assert_eq!(filter.hypotheses[0].state.covariance, covariance);
    }

    #[test]
    fn detector_at_current_odometry_stamp_associates_once_after_walking() {
        use linear_algebra::{Isometry3, vector};
        let camera = CameraMatrix::from_normalized_focal_and_center(
            na::vector![0.5, 0.5],
            na::point![0.5, 0.5],
            vector![640.0, 544.0],
            Isometry3::identity(),
            Isometry3::identity(),
            Isometry3::from_translation(0.0, 0.0, 1.0),
        );
        let original = camera.pixel_to_ground(point![320.0, 400.0]).unwrap();
        let object_at = |position| {
            let pixel = camera.ground_to_pixel(position).unwrap();
            Object::<RobocupObjectLabel>::from([
                pixel.x() - 10.0,
                pixel.y() - 40.0,
                pixel.x() + 10.0,
                pixel.y(),
                1.0,
                4.0,
            ])
        };
        let parameters = ObstacleFilterParameters {
            use_detected_objects: true,
            object_detection_measurement_matching_distance: 0.05,
            robot_measurement_noise: na::vector![0.1, 0.1],
            process_noise: na::vector![0.005, 0.005],
            ..Default::default()
        };
        let mut filter = ObstacleFilter::default();
        let start = Time::from_nanos(1_000_000_000);
        assert!(filter.process_detection(
            start,
            &parameters,
            &[object_at(original)],
            Some(&camera),
            Some(Pose2::default())
        ));
        let pose = Pose2::new(point![0.2, 0.0], 0.1);
        let time = start + Duration::from_millis(40);
        assert!(filter.advance_ground_frame(time, pose, na::Vector2::zeros()));
        let current = types::odometry::previous_to_current(Pose2::default(), pose) * original;
        assert!(filter.process_detection(
            time,
            &parameters,
            &[object_at(current)],
            Some(&camera),
            Some(pose)
        ));
        assert_eq!(filter.hypotheses.len(), 1);
        assert_eq!(filter.hypotheses[0].measurement_count, 2);
        assert!((filter.hypotheses[0].state.mean - current.inner.coords).norm() < 1e-5);
        assert!(!filter.process_detection(
            time,
            &parameters,
            &[object_at(current)],
            Some(&camera),
            Some(pose)
        ));
        assert_eq!(filter.hypotheses[0].measurement_count, 2);
    }

    #[test]
    fn camera_silence_still_expires_obstacles_as_odometry_advances() {
        let mut filter = ObstacleFilter::default();
        let start = Time::from_nanos(1_000_000_000);
        filter.advance_ground_frame(start, Pose2::default(), na::Vector2::zeros());
        filter.spawn_hypothesis(
            point![3.0, 1.0],
            ObstacleKind::Robot,
            start,
            Matrix2::identity(),
        );
        let parameters = ObstacleFilterParameters {
            hypothesis_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let time = start + Duration::from_millis(101);
        filter.advance_ground_frame(
            time,
            Pose2::new(point![0.1, 0.0], 0.0),
            na::Vector2::zeros(),
        );
        let obstacles = filter.compose_outputs(
            time,
            &parameters,
            &FieldDimensions::SPL_2025,
            None,
            PrimaryState::Damping,
            None,
        );
        assert!(obstacles.is_empty());
        assert!(filter.hypotheses.is_empty());
        assert_eq!(filter.ground_frame.unwrap().0, time);
    }

    #[test]
    fn ground_frame_integrates_the_entire_walk_and_turn_between_images() {
        let mut filter = ObstacleFilter::default();
        let start = Time::from_nanos(1_000_000_000);
        assert!(filter.advance_ground_frame(
            start,
            Pose2::new(point![0.0, 0.0], 0.0),
            na::Vector2::zeros(),
        ));
        filter.spawn_hypothesis(
            point![3.0, 1.0],
            ObstacleKind::Robot,
            start,
            Matrix2::identity(),
        );
        // Many unobserved odometry ticks occurred between these images. A full
        // metre of translation and a quarter turn must both be compensated.
        let next = start + Duration::from_millis(40);
        let pose = Pose2::new(point![1.0, 0.0], std::f32::consts::FRAC_PI_2);
        assert!(filter.advance_ground_frame(next, pose, na::Vector2::zeros()));
        let position = filter.hypotheses[0].state.mean;
        assert!((position - na::vector![1.0, -2.0]).norm() < 1e-5);
        assert!(!filter.advance_ground_frame(start, Pose2::default(), na::Vector2::zeros()));
        assert!(filter.advance_ground_frame(next, pose, na::Vector2::zeros()));
        assert_eq!(filter.hypotheses[0].state.mean, position);
        assert_eq!(filter.ground_frame.unwrap().0, next);
    }

    #[test]
    fn missing_odometry_cannot_relabel_an_old_ground_frame() {
        let mut filter = ObstacleFilter::default();
        let time = Time::from_nanos(1_000_000_000);
        let parameters = ObstacleFilterParameters::default();
        assert!(!filter.process_detection(time, &parameters, &[], None, None));
        assert!(filter.ground_frame.is_none());
        assert!(filter.process_detection(time, &parameters, &[], None, Some(Pose2::default())));
        assert!(!filter.process_detection(
            time + Duration::from_secs(1),
            &parameters,
            &[],
            None,
            None
        ));
        assert_eq!(filter.ground_frame.unwrap().0, time);
    }

    #[test]
    fn network_measurement_uses_existing_ground_frame_not_delivery_frame() {
        let mut filter = ObstacleFilter::default();
        let frame_time = Time::from_nanos(1_000_000_000);
        let pose = Pose2::new(point![1.0, 0.0], std::f32::consts::FRAC_PI_2);
        filter.advance_ground_frame(frame_time, pose, na::Vector2::zeros());
        let frame_to_field = Isometry2::from_parts(
            linear_algebra::vector![1.0, 0.0],
            std::f32::consts::FRAC_PI_2,
        );
        let teammate = PlayerState {
            pose: point![3.0, 1.0].into(),
            ball_position: None,
        };
        filter.process_network_player_state(
            frame_time + Duration::from_millis(50),
            &ObstacleFilterParameters::default(),
            &teammate,
            &frame_to_field,
        );
        assert!((filter.hypotheses[0].state.mean - na::vector![1.0, -2.0]).norm() < 1e-5);
        assert_eq!(filter.ground_frame.unwrap().0, frame_time);
        assert_eq!(
            filter.hypotheses[0].last_update,
            frame_time + Duration::from_millis(50)
        );
    }

    #[test]
    fn obstacle_filter_starts_without_hypotheses() {
        let filter = ObstacleFilter::default();
        assert!(filter.hypotheses.is_empty());
        assert_eq!(filter.last_primary_state, PrimaryState::Damping);
    }

    #[test]
    fn spawn_hypothesis_records_measurement_time_and_kind() {
        let mut filter = ObstacleFilter::default();
        let detection_time = ros_z::time::Time::from_nanos(1);
        let position = linear_algebra::point![1.0, 2.0];

        filter.spawn_hypothesis(
            position,
            ObstacleKind::Robot,
            detection_time,
            nalgebra::Matrix2::identity(),
        );

        assert_eq!(filter.hypotheses.len(), 1);
        assert_eq!(filter.hypotheses[0].obstacle_kind, ObstacleKind::Robot);
        assert_eq!(filter.hypotheses[0].measurement_count, 1);
        assert_eq!(filter.hypotheses[0].last_update, detection_time);
    }

    #[test]
    fn measured_player_position_is_transformed_from_field_to_ground() {
        use linear_algebra::IntoTransform;
        use types::world_state::PlayerState;

        let ground_to_field: Isometry2<Ground, Field> =
            na::Isometry2::translation(1.0, 2.0).framed_transform();
        let player_state = PlayerState {
            pose: linear_algebra::point![3.0, 5.0].into(),
            ball_position: None,
        };

        let position = measured_player_position(&player_state, &ground_to_field);

        assert_eq!(position, linear_algebra::point![2.0, 3.0]);
    }

    #[test]
    fn player_state_snapshots_extract_each_measurement_once() {
        use hsl_network_messages::PlayerNumber;

        let own_player_state = PlayerState {
            pose: linear_algebra::point![1.0, 2.0].into(),
            ball_position: None,
        };
        let teammate_state = PlayerState {
            pose: linear_algebra::point![3.0, 4.0].into(),
            ball_position: None,
        };
        let players = Players {
            two: Some(TimeWrapper {
                time: Time::from_nanos(2),
                inner: own_player_state,
            }),
            four: Some(TimeWrapper {
                time: Time::from_nanos(4),
                inner: teammate_state,
            }),
            ..Players::new(None)
        };
        let mut last_processed_player_state_times = Players::new(None);

        let entries = new_network_player_states(
            &players,
            Some(PlayerNumber::Two),
            &mut last_processed_player_state_times,
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, Time::from_nanos(4));
        assert_eq!(entries[0].1.pose.position(), teammate_state.pose.position());
        assert_eq!(
            last_processed_player_state_times[PlayerNumber::Two],
            Some(Time::from_nanos(2))
        );
        assert_eq!(
            last_processed_player_state_times[PlayerNumber::Four],
            Some(Time::from_nanos(4))
        );

        let repeated_entries = new_network_player_states(
            &players,
            Some(PlayerNumber::Two),
            &mut last_processed_player_state_times,
        );
        assert!(repeated_entries.is_empty());

        let newer_teammate_state = PlayerState {
            pose: linear_algebra::point![5.0, 6.0].into(),
            ball_position: None,
        };
        let updated_players = Players {
            four: Some(TimeWrapper {
                time: Time::from_nanos(6),
                inner: newer_teammate_state,
            }),
            ..players.clone()
        };

        let newer_entries = new_network_player_states(
            &updated_players,
            Some(PlayerNumber::Two),
            &mut last_processed_player_state_times,
        );

        assert_eq!(newer_entries.len(), 1);
        assert_eq!(newer_entries[0].0, Time::from_nanos(6));
        assert_eq!(
            newer_entries[0].1.pose.position(),
            newer_teammate_state.pose.position()
        );
    }
}

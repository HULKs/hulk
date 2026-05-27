use std::{boxed::Box, future::Future, pin::Pin};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use filtering::kalman_filter::KalmanFilter;
use geometry::rectangle::Rectangle;
use hsl_network_messages::PlayerNumber;
use itertools::{chain, iproduct};
use na::Matrix2;
use nalgebra as na;
use serde::{Deserialize, Serialize};

use booster::{FallDownState, FallDownStateType};
use coordinate_systems::{Field, Ground};
use linear_algebra::{IntoFramed, Isometry2, Point2, center, point};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use types::{
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Object, RobocupObjectLabel, YOLOObjectLabel},
    obstacle_filter::Hypothesis,
    obstacles::{Obstacle, ObstacleKind},
    parameters::ObstacleFilterParameters,
    players::Players,
    pose_detection::Pose,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    world_state::PlayerState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObstacleFilter {
    hypotheses: Vec<Hypothesis>,
    last_primary_state: PrimaryState,
}

impl Default for ObstacleFilter {
    fn default() -> Self {
        Self {
            hypotheses: Vec::new(),
            last_primary_state: PrimaryState::Damping,
        }
    }
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
        .cache(10)
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
    let current_odometry_to_last_odometry_cache = node
        .subscriber::<na::Isometry2<f32>>("current_odometry_to_last_odometry")
        .cache(10)
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
        .subscriber::<Option<Isometry2<Ground, Field>>>("ground_to_field")
        .cache(10)
        .build()
        .await?;
    let fall_down_state_cache = node
        .subscriber::<FallDownState>("inputs/fall_down_state")
        .cache(10)
        .build()
        .await?;

    let mut detections = node
        .create_future_map_builder()
        .create_future_subscriber::<Vec<Object<RobocupObjectLabel>>>(
            "detected_objects",
            Duration::from_millis(50),
        )
        .await?
        .create_future_subscriber::<Vec<Pose<YOLOObjectLabel>>>(
            "detected_poses",
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
    let mut last_processed_player_state_times = Players::new(None);
    loop {
        tokio::select! {
            received_detections = detections.recv() => {
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let item = received_detections?;

                let Some(field_dimensions) = field_dimensions_cache.get_latest() else {
                    continue;
                };
                for (detection_time, (detected_objects, detected_poses)) in item.persistent {
                    let detected_objects = detected_objects.unwrap_or_default();
                    let detected_poses = detected_poses.unwrap_or_default();
                    let camera_matrix = camera_matrix_cache.get_nearest(detection_time);
                    let current_odometry_to_last_odometry =
                        current_odometry_to_last_odometry_cache.get_nearest(detection_time);
                    let ground_to_field = ground_to_field_cache.get_nearest(detection_time);
                    let ground_to_field = ground_to_field.as_deref().and_then(Option::as_ref);

                    obstacle_filter.process_detection(
                        detection_time,
                        parameters,
                        &detected_objects,
                        &detected_poses,
                        camera_matrix.as_ref().map(|wrapper| &wrapper.inner),
                        current_odometry_to_last_odometry
                            .as_ref()
                            .map(Arc::as_ref),
                    );

                    let primary_state = primary_state_cache
                        .get_latest()
                        .map(|primary_state| *primary_state)
                        .unwrap_or_default();
                    let fall_down_state = fall_down_state_cache.get_latest();

                    let obstacles = obstacle_filter.compose_outputs(
                        node.clock().now(),
                        parameters,
                        field_dimensions.as_ref(),
                        ground_to_field,
                        primary_state,
                        fall_down_state.as_ref().map(Arc::as_ref),
                    );

                    obstacle_filter_hypotheses_pub
                        .publish(&obstacle_filter.hypotheses)
                        .await?;
                    obstacles_pub.publish(&obstacles).await?;
                }
            }
            received_player_states = player_states_subscriber.recv() => {
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let player_states = received_player_states?;
                let own_player_number = player_number_cache
                    .get_latest()
                    .map(|player_number| *player_number);
                let network_player_states = new_network_player_states(
                    &player_states,
                    own_player_number,
                    &mut last_processed_player_state_times,
                );
                let mut last_ground_to_field = None;
                let mut processed_player_state = false;

                for (player_state_time, player_state) in network_player_states {
                    let Some(ground_to_field) = ground_to_field_cache.get_nearest(player_state_time)
                    else {
                        continue;
                    };
                    let Some(ground_to_field_ref) = ground_to_field.as_ref() else {
                        continue;
                    };

                    obstacle_filter.process_network_player_state(
                        player_state_time,
                        parameters,
                        &player_state,
                        ground_to_field_ref,
                    );
                    last_ground_to_field = Some(ground_to_field);
                    processed_player_state = true;
                }

                if !processed_player_state {
                    continue;
                }

                let Some(field_dimensions) = field_dimensions_cache.get_latest() else {
                    continue;
                };
                let primary_state = primary_state_cache
                    .get_latest()
                    .map(|primary_state| *primary_state)
                    .unwrap_or_default();
                let fall_down_state = fall_down_state_cache.get_latest();

                let obstacles = obstacle_filter.compose_outputs(
                    node.clock().now(),
                    parameters,
                    field_dimensions.as_ref(),
                    last_ground_to_field.as_deref().and_then(Option::as_ref),
                    primary_state,
                    fall_down_state.as_ref().map(Arc::as_ref),
                );

                obstacle_filter_hypotheses_pub
                    .publish(&obstacle_filter.hypotheses)
                    .await?;
                obstacles_pub.publish(&obstacles).await?;
            }
        }
    }
}

impl ObstacleFilter {
    fn process_detection(
        &mut self,
        detection_time: Time,
        parameters: &ObstacleFilterParameters,
        detected_objects: &[Object<RobocupObjectLabel>],
        detected_poses: &[Pose<YOLOObjectLabel>],
        camera_matrix: Option<&CameraMatrix>,
        current_odometry_to_last_odometry: Option<&na::Isometry2<f32>>,
    ) {
        let current_odometry_to_last_odometry = current_odometry_to_last_odometry
            .copied()
            .unwrap_or_default();
        self.predict_hypotheses_with_odometry(
            current_odometry_to_last_odometry.inverse(),
            Matrix2::from_diagonal(&parameters.process_noise),
        );

        if let Some(camera_matrix) = camera_matrix
            && parameters.use_detected_objects
        {
            let measured_object_positions =
                measured_object_positions(parameters, detected_objects, camera_matrix);
            let measured_pose_positions =
                measured_pose_positions(parameters, detected_poses, camera_matrix);

            for (kind, position, measurement_noise) in
                measured_object_positions.chain(measured_pose_positions)
            {
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
        fall_down_state: Option<&FallDownState>,
    ) -> Vec<Obstacle> {
        self.remove_hypotheses(
            now,
            parameters.hypothesis_timeout,
            parameters.hypothesis_merge_distance,
        );

        let became_unpenalized = self.last_primary_state == PrimaryState::Penalized
            && primary_state != PrimaryState::Penalized;

        let is_upright = fall_down_state.is_none_or(|fall_down_state| {
            fall_down_state.fall_down_state != FallDownStateType::IsReady
        });

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

        let (kind, measurement_noise) = match label {
            RobocupObjectLabel::GoalPost => (
                ObstacleKind::GoalPost,
                parameters.goal_post_measurement_noise,
            ),
            RobocupObjectLabel::Robot => (ObstacleKind::Robot, parameters.robot_measurement_noise),
            _ => return None,
        };

        let bottom_center_position = {
            let Rectangle { min, max } = bounding_box.area;

            point![min.x() + (max.x() - min.x()) / 2.0, max.y()]
        };

        let obstacle_center: Point2<Ground> =
            camera_matrix.pixel_to_ground(bottom_center_position).ok()?;

        Some((kind, obstacle_center, measurement_noise))
    })
}

fn measured_pose_positions(
    parameters: &ObstacleFilterParameters,
    detected_poses: &[Pose<YOLOObjectLabel>],
    camera_matrix: &CameraMatrix,
) -> impl Iterator<Item = (ObstacleKind, Point2<Ground>, na::Vector2<f32>)> {
    detected_poses.iter().filter_map(|detected_pose| {
        let Object {
            label,
            bounding_box,
        } = detected_pose.object;

        let (kind, measurement_noise) = match label {
            YOLOObjectLabel::Person => (ObstacleKind::Person, parameters.person_measurement_noise),
            _ => return None,
        };

        let keypoints = detected_pose.keypoints;

        if keypoints.left_foot.confidence > parameters.person_feet_keypoints_confidence_threshold
            && keypoints.right_foot.confidence
                > parameters.person_feet_keypoints_confidence_threshold
        {
            let feet_center_point = center(keypoints.left_foot.point, keypoints.right_foot.point);

            let obstacle_center: Point2<Ground> =
                camera_matrix.pixel_to_ground(feet_center_point).ok()?;

            Some((kind, obstacle_center, measurement_noise))
        } else if bounding_box.confidence > parameters.person_object_confidence_threshold {
            let bottom_center_position = {
                let Rectangle { min, max } = bounding_box.area;

                point![min.x() + (max.x() - min.x()) / 2.0, max.y()]
            };

            let obstacle_center: Point2<Ground> =
                camera_matrix.pixel_to_ground(bottom_center_position).ok()?;

            Some((kind, obstacle_center, measurement_noise))
        } else {
            None
        }
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

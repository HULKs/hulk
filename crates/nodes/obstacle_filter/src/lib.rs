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

use booster::{FallDownState, FallDownStateType};
use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{IntoFramed, Isometry2, Point2, Pose2, point};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use tokio::task::block_in_place;
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
    merge_required: bool,
    last_merge_distance: Option<f32>,
    #[cfg(test)]
    merge_passes: usize,
}

impl Default for ObstacleFilter {
    fn default() -> Self {
        Self {
            hypotheses: Vec::new(),
            last_primary_state: PrimaryState::Damping,
            ground_frame: None,
            last_detection_time: None,
            merge_required: false,
            last_merge_distance: None,
            #[cfg(test)]
            merge_passes: 0,
        }
    }
}

struct ObstacleFilterOutput {
    time: Time,
    hypotheses: Option<Vec<Hypothesis>>,
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
    let ground_to_field_subscriber = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await?;
    let fall_down_state_cache = node
        .subscriber::<FallDownState>("inputs/fall_down_state")
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
    let mut field_poses = FieldPoseHistory::default();
    let mut last_processed_player_state_times = Players::new(None);
    let mut expiry_tick = node.create_timer(Duration::from_millis(20));
    loop {
        let parameters_snapshot = parameters.snapshot();
        let settings = parameters_snapshot.typed();
        let compose = |filter: &mut ObstacleFilter, now, poses: &FieldPoseHistory| {
            let ground_to_field = filter.ground_frame.and_then(|(time, _)| poses.at(time));
            let dimensions = field_dimensions_cache.get_latest();
            let primary_state = primary_state_cache
                .get_latest()
                .map(|state| *state)
                .unwrap_or_default();
            let fall_down_state = fall_down_state_cache.get_latest();
            filter.output(
                now,
                settings,
                dimensions.as_deref(),
                ground_to_field.as_ref(),
                primary_state,
                fall_down_state.as_deref(),
                obstacle_filter_hypotheses_pub.has_subscribers(),
            )
        };
        let outputs = tokio::select! {
            received = ground_to_field_subscriber.recv_with_metadata() => {
                field_poses.insert(received?);
                continue;
            }
            _ = expiry_tick.tick() => {
                // Expiry follows the node clock even when geometry/odometry and
                // camera streams stop. Empty outputs need no coordinate frame.
                // One current prune suffices after a pause; skip old deadlines.
                expiry_tick.reset();
                vec![compose(&mut obstacle_filter, node.clock().now(), &field_poses)]
            }
            received_detections = detections.recv() => {
                let item = received_detections?;
                block_in_place(|| {
                    let now = node.clock().now();
                    obstacle_filter.prune_hypotheses(now, settings.hypothesis_timeout);
                    let mut outputs = Vec::new();
                    for (detection_time, (odometry, detected_objects)) in item.persistent {
                        if !measurement_is_recent(now, detection_time, settings.hypothesis_timeout) {
                            continue;
                        }
                        if let Some(pose) = odometry {
                            odometry_history.insert(detection_time, pose);
                            while odometry_history.len() > 1000 {
                                odometry_history.pop_first();
                            }
                        }
                        let odometry = odometry_history.range(..=detection_time).next_back()
                            .filter(|(stamp, _)| detection_time.duration_since(**stamp) <= Duration::from_millis(2))
                            .map(|(_, pose)| *pose);
                        let advanced = if let Some(detected_objects) = detected_objects {
                            let camera_matrix = camera_matrix_cache.get_nearest(detection_time)
                                .filter(|camera| frame_stamp_is_recent(detection_time, camera.time));
                            obstacle_filter.process_detection(
                                detection_time, settings, &detected_objects.inner,
                                camera_matrix.as_ref().map(|wrapper| &wrapper.inner), odometry,
                            )
                        } else {
                            odometry.is_some_and(|pose| obstacle_filter.advance_ground_frame(
                                detection_time, pose, na::Vector2::zeros(),
                            ))
                        };
                        if advanced {
                            outputs.push(compose(&mut obstacle_filter, now, &field_poses));
                        }
                    }
                    // Failed geometry must not prevent clearing expired output.
                    if outputs.is_empty() {
                        outputs.push(compose(&mut obstacle_filter, now, &field_poses));
                    }
                    outputs
                })
            }
            received_player_states = player_states_subscriber.recv() => {
                let player_states = received_player_states?;
                block_in_place(|| {
                    let now = node.clock().now();
                    obstacle_filter.prune_hypotheses(now, settings.hypothesis_timeout);
                    // Network positions are projected into the CURRENT stored
                    // Ground frame, and may not revive a stale/future frame.
                    if let Some(frame_time) = obstacle_filter.current_frame_time(now)
                        && let Some(ground_to_field) = field_poses.at(frame_time)
                    {
                        let own_player_number = player_number_cache.get_latest().map(|number| *number);
                        for (time, state) in new_network_player_states(
                            &player_states, own_player_number, &mut last_processed_player_state_times,
                            now, settings.hypothesis_timeout,
                        ) {
                            obstacle_filter.process_network_player_state(
                                time, settings, &state, &ground_to_field,
                            );
                        }
                    }
                    vec![compose(&mut obstacle_filter, now, &field_poses)]
                })
            }
        };
        for output in outputs {
            if let Some(hypotheses) = output.hypotheses {
                obstacle_filter_hypotheses_pub
                    .publish_with_source_time(&hypotheses, output.time)
                    .await?;
            }
            obstacles_pub
                .publish_with_source_time(&output.obstacles, output.time)
                .await?;
        }
    }
}

// This is a source-geometry tolerance, not message delivery latency. Never mark
// an old Ground frame as fresh simply because the filter processed it recently.
fn frame_stamp_is_recent(frame: Time, sample: Time) -> bool {
    frame.abs_diff(sample) <= Duration::from_millis(20)
}

// Persistent fusion normally trails the clock by about 50 ms. A stopped
// odometry stream must not keep publishing that old Ground frame indefinitely.
const MAXIMUM_OUTPUT_FRAME_AGE: Duration = Duration::from_millis(100);

fn measurement_is_recent(now: Time, stamp: Time, maximum_age: Duration) -> bool {
    stamp <= now && now.duration_since(stamp) < maximum_age
}

#[derive(Default)]
struct FieldPoseHistory(BTreeMap<Time, Isometry2<Ground, Field>>);

impl FieldPoseHistory {
    fn insert(&mut self, received: ros_z::pubsub::Received<Isometry2<Ground, Field>>) {
        self.0.insert(received.source_time, received.message);
        while self.0.len() > 200 {
            self.0.pop_first();
        }
    }

    fn at(&self, time: Time) -> Option<Isometry2<Ground, Field>> {
        [
            self.0.range(..=time).next_back(),
            self.0.range(time..).next(),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(stamp, _)| time.abs_diff(**stamp))
        .filter(|(stamp, _)| frame_stamp_is_recent(time, **stamp))
        .map(|(_, pose)| *pose)
    }
}

impl ObstacleFilter {
    fn current_frame_time(&self, now: Time) -> Option<Time> {
        self.ground_frame
            .map(|(time, _)| time)
            .filter(|time| *time <= now && now.duration_since(*time) <= MAXIMUM_OUTPUT_FRAME_AGE)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Keep expiry time, geometry inputs and diagnostic demand explicit at publication"
    )]
    fn output(
        &mut self,
        now: Time,
        parameters: &ObstacleFilterParameters,
        field_dimensions: Option<&FieldDimensions>,
        ground_to_field: Option<&Isometry2<Ground, Field>>,
        primary_state: PrimaryState,
        fall_down_state: Option<&FallDownState>,
        include_hypotheses: bool,
    ) -> ObstacleFilterOutput {
        if let (Some(time), Some(dimensions)) = (self.current_frame_time(now), field_dimensions) {
            let obstacles = self.compose_outputs(
                now,
                parameters,
                dimensions,
                ground_to_field,
                primary_state,
                fall_down_state,
            );
            ObstacleFilterOutput {
                time,
                hypotheses: include_hypotheses.then(|| self.hypotheses.clone()),
                obstacles,
            }
        } else {
            self.maintain(now, parameters, primary_state, fall_down_state);
            ObstacleFilterOutput {
                // No stale coordinates are relabelled as current: both lists
                // are empty, while internal tracks can still age normally.
                time: now,
                hypotheses: include_hypotheses.then(Vec::new),
                obstacles: Vec::new(),
            }
        }
    }

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
        fall_down_state: Option<&FallDownState>,
    ) -> Vec<Obstacle> {
        self.maintain(now, parameters, primary_state, fall_down_state);

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
        self.merge_required = true;
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
        self.merge_required = true;
    }

    fn prune_hypotheses(&mut self, now: Time, hypothesis_timeout: Duration) {
        self.hypotheses.retain(|hypothesis| {
            measurement_is_recent(now, hypothesis.last_update, hypothesis_timeout)
        });
    }

    fn maintain(
        &mut self,
        now: Time,
        parameters: &ObstacleFilterParameters,
        primary_state: PrimaryState,
        fall_down_state: Option<&FallDownState>,
    ) {
        self.prune_hypotheses(now, parameters.hypothesis_timeout);
        let became_unpenalized = self.last_primary_state == PrimaryState::Penalized
            && primary_state != PrimaryState::Penalized;
        self.last_primary_state = primary_state;
        if became_unpenalized {
            self.hypotheses.clear();
        }
        // Retain main's existing SDK posture policy; timestamp-safe obstacle
        // tracking does not introduce the separate physical fall detector.
        let is_upright =
            fall_down_state.is_none_or(|state| state.fall_down_state != FallDownStateType::IsReady);
        if !is_upright {
            self.hypotheses
                .retain(|hypothesis| hypothesis.obstacle_kind != ObstacleKind::Unknown);
        }
        self.merge_hypotheses_if_needed(parameters.hypothesis_merge_distance);
    }

    fn merge_hypotheses_if_needed(&mut self, merge_distance: f32) {
        if !self.merge_required
            && self
                .last_merge_distance
                .is_some_and(|previous| previous.to_bits() == merge_distance.to_bits())
        {
            return;
        }
        self.merge_required = false;
        self.last_merge_distance = Some(merge_distance);
        #[cfg(test)]
        {
            self.merge_passes += 1;
        }

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
    now: Time,
    maximum_age: Duration,
) -> Vec<(Time, PlayerState)> {
    let Some(own_player_number) = own_player_number else {
        return Vec::new();
    };

    players
        .iter()
        .filter_map(|(player_number, player_state)| {
            let player_state = player_state.as_ref()?;
            if !measurement_is_recent(now, player_state.time, maximum_age) {
                return None;
            }
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
    fn field_pose_lookup_uses_source_metadata_despite_delayed_transport() {
        let source_time = Time::from_nanos(1_000_000_000);
        let transport_time = source_time + Duration::from_millis(80);
        let pose = Isometry2::from_parts(linear_algebra::vector![1.0, 2.0], 0.0);
        let mut history = FieldPoseHistory::default();
        history.insert(ros_z::pubsub::Received {
            message: pose,
            source_time,
            transport_time: Some(transport_time),
            sequence_number: 1,
            source_global_id: ros_z::attachment::EndpointGlobalId::ZERO,
        });
        assert_eq!(history.at(source_time), Some(pose));
        assert!(history.at(transport_time).is_none());
        // The same selected source frame serves both network robot positions
        // and static goal posts, despite late arrival of the localization.
        let selected = history.at(source_time).unwrap();
        let teammate = PlayerState {
            pose: point![3.0, 5.0].into(),
            ball_position: None,
        };
        assert_eq!(
            measured_player_position(&teammate, &selected),
            point![2.0, 3.0]
        );
        let dimensions = FieldDimensions::SPL_2025;
        let field_posts = calculate_goal_post_positions(Some(&Isometry2::identity()), &dimensions);
        let ground_posts = calculate_goal_post_positions(Some(&selected), &dimensions);
        for (field, ground) in field_posts.into_iter().zip(ground_posts) {
            let expected_field = point![<Field>, field.x(), field.y()];
            assert!((selected * ground - expected_field).norm() < 1e-5);
        }
    }

    #[test]
    fn frozen_ground_frame_expires_by_clock_and_clears_without_localization() {
        let mut filter = ObstacleFilter::default();
        let frame = Time::from_nanos(1_000_000_000);
        filter.advance_ground_frame(frame, Pose2::default(), na::Vector2::zeros());
        // Teammate input can legitimately be newer than our delayed Ground
        // frame. Expiry still uses the current clock, not that frame's time.
        filter.spawn_hypothesis(
            point![1.0, 0.0],
            ObstacleKind::Robot,
            frame + Duration::from_millis(50),
            Matrix2::identity(),
        );
        let settings = ObstacleFilterParameters {
            hypothesis_timeout: Duration::from_millis(40),
            ..Default::default()
        };
        let before = filter.output(
            frame + Duration::from_millis(70),
            &settings,
            Some(&FieldDimensions::SPL_2025),
            None,
            PrimaryState::Damping,
            None,
            true,
        );
        assert_eq!(before.obstacles.len(), 1);
        assert_eq!(before.time, frame);
        let expired = filter.output(
            frame + Duration::from_millis(91),
            &settings,
            Some(&FieldDimensions::SPL_2025),
            None,
            PrimaryState::Damping,
            None,
            true,
        );
        assert!(expired.obstacles.is_empty());
        assert!(expired.hypotheses.unwrap().is_empty());
        assert!(filter.hypotheses.is_empty());
        assert_eq!(filter.ground_frame.unwrap().0, frame);
    }

    #[test]
    fn stale_or_missing_geometry_cannot_publish_old_coordinates_as_current() {
        let mut filter = ObstacleFilter::default();
        let frame = Time::from_nanos(1_000_000_000);
        filter.advance_ground_frame(frame, Pose2::default(), na::Vector2::zeros());
        filter.spawn_hypothesis(
            point![1.0, 0.0],
            ObstacleKind::Robot,
            frame,
            Matrix2::identity(),
        );
        let settings = ObstacleFilterParameters {
            hypothesis_timeout: Duration::from_secs(1),
            ..Default::default()
        };
        let now = frame + Duration::from_millis(101);
        let cleared = filter.output(
            now,
            &settings,
            Some(&FieldDimensions::SPL_2025),
            Some(&Isometry2::identity()),
            PrimaryState::Damping,
            None,
            true,
        );
        assert!(cleared.obstacles.is_empty());
        assert!(cleared.hypotheses.unwrap().is_empty());
        assert_eq!(cleared.time, now);
        assert_eq!(filter.hypotheses.len(), 1); // retained internally in its actual frame
        assert!(filter.current_frame_time(now).is_none());
        let expired = filter.output(
            frame + Duration::from_secs(2),
            &settings,
            None,
            None,
            PrimaryState::Damping,
            None,
            false,
        );
        assert!(expired.obstacles.is_empty());
        assert!(expired.hypotheses.is_none());
        assert!(filter.hypotheses.is_empty());
    }

    #[test]
    fn future_network_measurement_is_rejected_without_poisoning_deduplication() {
        let now = Time::from_nanos(1_000_000_000);
        let state = PlayerState {
            pose: point![1.0, 2.0].into(),
            ball_position: None,
        };
        let mut players = Players::new(None);
        players.four = Some(TimeWrapper {
            time: now + Duration::from_secs(1),
            inner: state,
        });
        let mut processed = Players::new(None);
        assert!(
            new_network_player_states(
                &players,
                Some(PlayerNumber::Two),
                &mut processed,
                now,
                Duration::from_secs(1)
            )
            .is_empty()
        );
        assert_eq!(processed.four, None);
        players.four.as_mut().unwrap().time = now;
        assert_eq!(
            new_network_player_states(
                &players,
                Some(PlayerNumber::Two),
                &mut processed,
                now,
                Duration::from_secs(1)
            )
            .len(),
            1
        );
        let mut filter = ObstacleFilter::default();
        filter.spawn_hypothesis(
            point![1.0, 0.0],
            ObstacleKind::Robot,
            now + Duration::from_secs(1),
            Matrix2::identity(),
        );
        filter.prune_hypotheses(now, Duration::from_secs(10));
        assert!(filter.hypotheses.is_empty());
    }

    #[test]
    fn odometry_only_output_skips_merging_and_unobserved_diagnostics() {
        let start = Time::from_nanos(1_000_000_000);
        let mut filter = ObstacleFilter::default();
        filter.advance_ground_frame(start, Pose2::default(), na::Vector2::zeros());
        for x in [1.0, 1.2] {
            filter.spawn_hypothesis(
                point![x, 0.0],
                ObstacleKind::Robot,
                start,
                Matrix2::identity(),
            );
        }
        let mut settings = ObstacleFilterParameters {
            hypothesis_timeout: Duration::from_secs(2),
            hypothesis_merge_distance: 0.1,
            ..Default::default()
        };
        for tick in 0..=500 {
            let time = start + Duration::from_millis(tick * 2);
            filter.advance_ground_frame(
                time,
                Pose2::new(point![tick as f32 * 0.001, 0.0], 0.0),
                na::Vector2::zeros(),
            );
            let output = filter.output(
                time,
                &settings,
                Some(&FieldDimensions::SPL_2025),
                None,
                PrimaryState::Damping,
                None,
                false,
            );
            assert!(output.hypotheses.is_none());
            assert_eq!(output.obstacles.len(), 2);
        }
        assert_eq!(filter.merge_passes, 1);
        settings.hypothesis_merge_distance = 0.3;
        let time = start + Duration::from_secs(1);
        let output = filter.output(
            time,
            &settings,
            Some(&FieldDimensions::SPL_2025),
            None,
            PrimaryState::Damping,
            None,
            true,
        );
        assert_eq!(filter.merge_passes, 2);
        assert_eq!(output.hypotheses.unwrap().len(), 1);
        filter.update_hypotheses_with_measurement(
            point![3.0, 0.0],
            ObstacleKind::Robot,
            time,
            0.1,
            Matrix2::identity(),
            MeasurementKind::Own,
        );
        filter.output(
            time,
            &settings,
            Some(&FieldDimensions::SPL_2025),
            None,
            PrimaryState::Damping,
            None,
            false,
        );
        assert_eq!(filter.merge_passes, 3);
    }

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
            Time::from_nanos(10),
            Duration::from_secs(1),
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
            Time::from_nanos(10),
            Duration::from_secs(1),
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
            Time::from_nanos(10),
            Duration::from_secs(1),
        );

        assert_eq!(newer_entries.len(), 1);
        assert_eq!(newer_entries[0].0, Time::from_nanos(6));
        assert_eq!(
            newer_entries[0].1.pose.position(),
            newer_teammate_state.pose.position()
        );
    }
}

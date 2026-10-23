use std::{boxed::Box, collections::BTreeMap, future::Future, pin::Pin};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use filtering::kalman_filter::KalmanFilter;
use geometry::rectangle::Rectangle;
use hsl_network_messages::PlayerNumber;
use itertools::{chain, iproduct};
use na::{Matrix2, Matrix4, SMatrix};
use nalgebra as na;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{IntoFramed, Isometry2, Point2, Pose2, point};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use tokio::task::block_in_place;
use types::fall_detection::{FallDetection, Posture};
use types::{
    field_dimensions::FieldDimensions,
    game_controller_state::GameControllerState,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Object, RobocupObjectLabel},
    obstacle_filter::Hypothesis,
    obstacles::{Obstacle, ObstacleKind},
    parameters::ObstacleFilterParameters,
    players::Players,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    tracked_objects::{
        HumanoidRole, HumanoidTrack, StationaryClass, StationaryTrack, TeammateProbability,
        TrackStatus, TrackedObjects,
    },
    world_state::PlayerState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObstacleFilter {
    hypotheses: Vec<Hypothesis>,
    next_id: u64,
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
            next_id: 0,
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
    humanoid_tracks: TrackedObjects<HumanoidTrack>,
    stationary_tracks: TrackedObjects<StationaryTrack>,
}

struct Measurement {
    kind: ObstacleKind,
    stationary_class: Option<StationaryClass>,
    position: Point2<Ground>,
    noise: Matrix2<f32>,
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
    let fall_detection_cache = node
        .subscriber::<FallDetection>("fall_detection/status")
        .cache(10)
        .build()
        .await?;
    let game_controller_state_cache = node
        .subscriber::<Option<GameControllerState>>("game_controller_state")
        .cache(1)
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
    let humanoid_tracks_pub = node
        .publisher::<TrackedObjects<HumanoidTrack>>("obstacle_filter/humanoid_tracks")
        .build()
        .await?;
    let stationary_tracks_pub = node
        .publisher::<TrackedObjects<StationaryTrack>>("obstacle_filter/stationary_tracks")
        .build()
        .await?;

    let mut obstacle_filter = ObstacleFilter::default();
    let mut odometry_history = BTreeMap::new();
    let mut field_poses = FieldPoseHistory::default();
    let mut last_processed_player_state_times = Players::new(None);
    let mut expiry_tick = node.create_timer(Duration::from_millis(20));
    loop {
        let parameters_snapshot = parameters.snapshot();
        let settings = parameters_snapshot.typed();
        let compose = |filter: &mut ObstacleFilter, now, poses: &FieldPoseHistory| {
            if let Some(state) = game_controller_state_cache.get_latest()
                && let Some(state) = state.as_ref()
            {
                filter.hypotheses.retain(|hypothesis| {
                    hypothesis
                        .teammate
                        .is_none_or(|number| state.penalties[number].is_none())
                });
            }
            let ground_to_field = filter.ground_frame.and_then(|(time, _)| poses.at(time));
            let dimensions = field_dimensions_cache.get_latest();
            let primary_state = primary_state_cache
                .get_latest()
                .map(|state| *state)
                .unwrap_or(PrimaryState::Damping);
            let fall_detection = fall_detection_cache.get_latest();
            filter.output(
                now,
                settings,
                dimensions.as_deref(),
                ground_to_field.as_ref(),
                primary_state,
                fall_detection.as_deref(),
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
                        for (player_number, time, state) in new_network_player_states(
                            &player_states, own_player_number, &mut last_processed_player_state_times,
                            now, settings.hypothesis_timeout.min(TEAM_REPORT_TIMEOUT),
                        ) {
                            obstacle_filter.process_network_player_state(
                                time, settings, player_number, &state, &ground_to_field,
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
            humanoid_tracks_pub
                .publish_with_source_time(&output.humanoid_tracks, output.time)
                .await?;
            stationary_tracks_pub
                .publish_with_source_time(&output.stationary_tracks, output.time)
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
const MAXIMUM_TRACKS: usize = 64;
const TEAM_REPORT_TIMEOUT: Duration = Duration::from_secs(3);
const MAXIMUM_MOVING_OBSTACLE_AGE: Duration = Duration::from_millis(500);

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
        fall_detection: Option<&FallDetection>,
        include_hypotheses: bool,
    ) -> ObstacleFilterOutput {
        if let Some(time) = self.current_frame_time(now) {
            self.maintain(now, parameters, primary_state, fall_detection);
            let obstacles = field_dimensions.map_or_else(Vec::new, |dimensions| {
                self.compose_outputs(now, parameters, dimensions, ground_to_field)
            });
            let (humanoid_tracks, stationary_tracks) =
                self.tracked_objects(time, ground_to_field.copied(), parameters, true);
            ObstacleFilterOutput {
                time,
                hypotheses: include_hypotheses.then(|| self.hypotheses.clone()),
                obstacles,
                humanoid_tracks,
                stationary_tracks,
            }
        } else {
            self.maintain(now, parameters, primary_state, fall_detection);
            let (humanoid_tracks, stationary_tracks) =
                self.tracked_objects(now, None, parameters, false);
            ObstacleFilterOutput {
                // No stale coordinates are relabelled as current: both lists
                // are empty, while internal tracks can still age normally.
                time: now,
                hypotheses: include_hypotheses.then(Vec::new),
                obstacles: Vec::new(),
                humanoid_tracks,
                stationary_tracks,
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
            let measurements =
                measured_object_positions(parameters, detected_objects, camera_matrix)
                    .collect::<Vec<_>>();
            self.associate_measurements(
                detection_time,
                &measurements,
                parameters.object_detection_measurement_matching_distance,
            );
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
                time.duration_since(previous_time).as_secs_f32(),
            );
        }
        self.ground_frame = Some((time, odometry));
        true
    }

    fn process_network_player_state(
        &mut self,
        player_state_time: Time,
        parameters: &ObstacleFilterParameters,
        player_number: PlayerNumber,
        player_state: &PlayerState,
        ground_to_field: &Isometry2<Ground, Field>,
    ) {
        // Reports are correlated through localization. They identify seen robots,
        // but must not repeatedly shrink their camera position uncertainty.
        if player_state
            .pose_age
            .is_some_and(|age| age > Duration::from_secs(3))
        {
            return;
        }
        let player_position = measured_player_position(player_state, ground_to_field);
        if !player_position
            .inner
            .coords
            .iter()
            .all(|value| value.is_finite())
        {
            return;
        }
        let mut candidates: Vec<_> = self
            .hypotheses
            .iter()
            .enumerate()
            .filter(|(_, hypothesis)| {
                hypothesis.obstacle_kind == ObstacleKind::Robot
                    && hypothesis
                        .teammate
                        .is_none_or(|teammate| teammate == player_number)
            })
            .map(|(index, hypothesis)| {
                (
                    index,
                    (hypothesis.state.mean - player_position.inner.coords).norm(),
                )
            })
            .filter(|(_, distance)| {
                *distance < parameters.network_robot_measurement_matching_distance
            })
            .collect();
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
        // An ambiguous report cannot turn a nearby opponent into a teammate.
        if candidates.len() > 1 && candidates[1].1 - candidates[0].1 < 0.2 {
            return;
        }
        let index = if let Some((index, _)) = candidates.first() {
            *index
        } else {
            if self.hypotheses.len() >= MAXIMUM_TRACKS {
                return;
            }
            self.spawn_hypothesis(
                player_position,
                ObstacleKind::Robot,
                player_state_time,
                Matrix2::from_diagonal(
                    &parameters
                        .network_robot_measurement_noise
                        .map(|variance| variance.max(0.09)),
                ),
            );
            let index = self.hypotheses.len() - 1;
            self.hypotheses[index].last_detection = None;
            index
        };
        for (other, hypothesis) in self.hypotheses.iter_mut().enumerate() {
            if other != index && hypothesis.teammate == Some(player_number) {
                hypothesis.teammate = None;
                hypothesis.teammate_evidence = 0;
                hypothesis.last_team_report = None;
            }
        }
        let hypothesis = &mut self.hypotheses[index];
        if hypothesis
            .last_team_report
            .is_some_and(|last| player_state_time <= last)
        {
            return;
        }
        if hypothesis.last_detection.is_none() {
            hypothesis.measurement_count += usize::from(hypothesis.last_team_report.is_some());
            hypothesis.state.mean = player_position.inner.coords;
            // Preserve the localization uncertainty floor for remote-only tracks.
            hypothesis.state.covariance = Matrix2::from_diagonal(
                &parameters
                    .network_robot_measurement_noise
                    .map(|variance| variance.max(0.09)),
            );
        }
        hypothesis.teammate = Some(player_number);
        hypothesis.teammate_evidence += 1;
        hypothesis.last_team_report = Some(player_state_time);
        hypothesis.last_update = hypothesis.last_update.max(player_state_time);
    }

    fn compose_outputs(
        &self,
        now: Time,
        parameters: &ObstacleFilterParameters,
        field_dimensions: &FieldDimensions,
        ground_to_field: Option<&Isometry2<Ground, Field>>,
    ) -> Vec<Obstacle> {
        let goal_posts = calculate_goal_post_positions(ground_to_field, field_dimensions);
        let obstacles = self
            .hypotheses
            .iter()
            .filter(|hypothesis| {
                hypothesis.measurement_count > parameters.measurement_count_threshold
                    && hypothesis
                        .stationary_class
                        .is_none_or(StationaryClass::is_obstacle)
                    && (!hypothesis.is_moving()
                        || (hypothesis
                            .last_detection
                            .or(hypothesis.last_team_report)
                            .is_some_and(|stamp| {
                                measurement_is_recent(
                                    now,
                                    stamp,
                                    parameters
                                        .hypothesis_timeout
                                        .min(MAXIMUM_MOVING_OBSTACLE_AGE),
                                )
                            })
                            && hypothesis
                                .state
                                .covariance
                                .symmetric_eigen()
                                .eigenvalues
                                .max()
                                <= 1.0))
                    && !(hypothesis.obstacle_kind == ObstacleKind::GoalPost
                        && goal_posts.iter().any(|position| {
                            (hypothesis.state.mean - position.inner.coords).norm()
                                < parameters.object_detection_measurement_matching_distance
                        }))
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
        let goal_post_obstacles = goal_posts
            .iter()
            .copied()
            .map(|goal_post| Obstacle::goal_post(goal_post, parameters.goal_post_obstacle_radius));

        chain!(obstacles, goal_post_obstacles).collect()
    }

    fn tracked_objects(
        &self,
        time: Time,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        parameters: &ObstacleFilterParameters,
        include_tracks: bool,
    ) -> (
        TrackedObjects<HumanoidTrack>,
        TrackedObjects<StationaryTrack>,
    ) {
        let ground_to_odometry = self
            .ground_frame
            .map_or_else(Isometry2::identity, |(_, pose)| {
                pose.as_transform::<Ground>()
            });
        let rotation = *ground_to_odometry
            .inner
            .rotation
            .to_rotation_matrix()
            .matrix();
        let mut rotation4 = Matrix4::zeros();
        rotation4.fixed_view_mut::<2, 2>(0, 0).copy_from(&rotation);
        rotation4.fixed_view_mut::<2, 2>(2, 2).copy_from(&rotation);
        let mut humanoids = Vec::new();
        let mut stationary = Vec::new();
        for hypothesis in self.hypotheses.iter().filter(|_| include_tracks) {
            let position_in_ground = hypothesis.state.mean.framed().as_point();
            let position_in_odometry = ground_to_odometry * position_in_ground;
            let position_in_field = ground_to_field.map(|transform| transform * position_in_ground);
            let time_since_detection = hypothesis
                .last_detection
                .map(|last| time.duration_since(last));
            let confirmed = hypothesis.measurement_count > parameters.measurement_count_threshold;
            let existence = (hypothesis.measurement_count as f32
                / (parameters.measurement_count_threshold as f32 + 1.0))
                .min(1.0);
            let status = if !confirmed {
                TrackStatus::Tentative
            } else if time_since_detection.is_none_or(|age| age > Duration::from_secs(1)) {
                TrackStatus::Dormant
            } else {
                TrackStatus::Confirmed
            };
            let detection_probability = if hypothesis.last_detection.is_some()
                && hypothesis.last_detection == self.last_detection_time
            {
                1.0
            } else {
                0.0
            };
            if hypothesis.is_moving() {
                let mut state = hypothesis.state_with_velocity();
                state.mean = rotation4 * state.mean;
                state
                    .mean
                    .fixed_rows_mut::<2>(0)
                    .copy_from(&position_in_odometry.inner.coords);
                state.covariance = rotation4 * state.covariance * rotation4.transpose();
                let role = if hypothesis.obstacle_kind == ObstacleKind::Person {
                    HumanoidRole::Human
                } else {
                    hypothesis
                        .teammate
                        .filter(|_| hypothesis.teammate_evidence >= 2)
                        .map_or(HumanoidRole::Opponent, HumanoidRole::Teammate)
                };
                humanoids.push(HumanoidTrack {
                    id: hypothesis.id,
                    existence,
                    status,
                    robot_probability: if hypothesis.obstacle_kind == ObstacleKind::Robot {
                        1.0
                    } else {
                        0.0
                    },
                    human_probability: if hypothesis.obstacle_kind == ObstacleKind::Person {
                        1.0
                    } else {
                        0.0
                    },
                    role,
                    teammate_probabilities: hypothesis
                        .teammate
                        .into_iter()
                        .map(|player_number| TeammateProbability {
                            player_number,
                            probability: if hypothesis.teammate_evidence >= 2 {
                                1.0
                            } else {
                                0.5
                            },
                        })
                        .collect(),
                    remote_only: hypothesis.last_detection.is_none(),
                    detection_probability,
                    time_since_detection,
                    state,
                    position_in_ground,
                    velocity_in_ground: hypothesis.velocity.mean.framed(),
                    position_in_field,
                });
            } else if let Some(class) = hypothesis.stationary_class {
                stationary.push(StationaryTrack {
                    id: hypothesis.id,
                    existence,
                    status,
                    class,
                    class_probabilities: StationaryClass::ALL
                        .into_iter()
                        .map(|candidate| if candidate == class { 1.0 } else { 0.0 })
                        .collect(),
                    is_obstacle: class.is_obstacle(),
                    detection_probability,
                    time_since_detection,
                    state: MultivariateNormalDistribution {
                        mean: position_in_odometry.inner.coords,
                        covariance: rotation * hypothesis.state.covariance * rotation.transpose(),
                    },
                    position_in_ground,
                    position_in_field,
                });
            }
        }
        (
            TrackedObjects {
                time,
                ground_to_odometry,
                ground_to_field,
                tracks: humanoids,
            },
            TrackedObjects {
                time,
                ground_to_odometry,
                ground_to_field,
                tracks: stationary,
            },
        )
    }

    fn predict_hypotheses_with_odometry(
        &mut self,
        last_odometry_to_current_odometry: na::Isometry2<f32>,
        process_noise: Matrix2<f32>,
        delta_time: f32,
    ) {
        for hypothesis in &mut self.hypotheses {
            let rotation = *last_odometry_to_current_odometry
                .rotation
                .to_rotation_matrix()
                .matrix();
            let mut transition = Matrix4::zeros();
            transition.fixed_view_mut::<2, 2>(0, 0).copy_from(&rotation);
            transition.fixed_view_mut::<2, 2>(2, 2).copy_from(&rotation);
            if hypothesis.is_moving() {
                transition
                    .fixed_view_mut::<2, 2>(0, 2)
                    .copy_from(&(rotation * delta_time));
            }
            let mut noise = Matrix4::zeros();
            noise.fixed_view_mut::<2, 2>(0, 0).copy_from(&process_noise);
            if hypothesis.is_moving() {
                noise.fixed_view_mut::<2, 2>(2, 2).copy_from(&process_noise);
            }
            let mut state = hypothesis.state_with_velocity();
            state.predict(
                transition,
                SMatrix::<f32, 4, 2>::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0),
                last_odometry_to_current_odometry.translation.vector,
                noise,
            );
            hypothesis.set_state_with_velocity(state);
        }
    }

    fn associate_measurements(
        &mut self,
        time: Time,
        measurements: &[Measurement],
        matching_distance: f32,
    ) {
        // Resolve all pairs before updating: neither detector order nor an early
        // state update may cause several detections to update the same track.
        let mut pairs = Vec::new();
        for (measurement_index, measurement) in measurements.iter().enumerate() {
            for (hypothesis_index, hypothesis) in self.hypotheses.iter().enumerate() {
                if hypothesis.obstacle_kind != measurement.kind
                    || hypothesis.stationary_class != measurement.stationary_class
                {
                    continue;
                }
                let distance = (hypothesis.state.mean - measurement.position.inner.coords).norm();
                if distance < matching_distance {
                    pairs.push((distance, measurement_index, hypothesis_index));
                }
            }
        }
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut used_measurements = vec![false; measurements.len()];
        let mut used_hypotheses = vec![false; self.hypotheses.len()];
        for (_, measurement_index, hypothesis_index) in pairs {
            if used_measurements[measurement_index] || used_hypotheses[hypothesis_index] {
                continue;
            }
            used_measurements[measurement_index] = true;
            used_hypotheses[hypothesis_index] = true;
            self.update_hypothesis(hypothesis_index, &measurements[measurement_index], time);
        }
        for (index, measurement) in measurements.iter().enumerate() {
            if !used_measurements[index] && self.hypotheses.len() < MAXIMUM_TRACKS {
                self.spawn_hypothesis(
                    measurement.position,
                    measurement.kind,
                    time,
                    measurement.noise,
                );
                self.hypotheses
                    .last_mut()
                    .expect("just spawned")
                    .stationary_class = measurement.stationary_class;
            }
        }
    }

    fn update_hypothesis(&mut self, index: usize, measurement: &Measurement, time: Time) {
        let hypothesis = &mut self.hypotheses[index];
        let mut state = hypothesis.state_with_velocity();
        state.update(
            SMatrix::<f32, 2, 4>::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            measurement.position.inner.coords,
            measurement.noise,
        );
        hypothesis.set_state_with_velocity(state);
        hypothesis.measurement_count += 1;
        hypothesis.last_update = hypothesis.last_update.max(time);
        hypothesis.last_detection = Some(time);
        self.merge_required = true;
    }

    fn spawn_hypothesis(
        &mut self,
        detected_position: Point2<Ground>,
        obstacle_kind: ObstacleKind,
        detection_time: Time,
        initial_covariance: Matrix2<f32>,
    ) {
        let new_hypothesis = Hypothesis {
            id: self.next_id,
            state: MultivariateNormalDistribution {
                mean: detected_position.inner.coords,
                covariance: initial_covariance,
            },
            velocity: MultivariateNormalDistribution {
                mean: na::Vector2::zeros(),
                covariance: if matches!(obstacle_kind, ObstacleKind::Robot | ObstacleKind::Person) {
                    Matrix2::identity() * 0.35_f32.powi(2)
                } else {
                    Matrix2::zeros()
                },
            },
            position_velocity_covariance: Matrix2::zeros(),
            obstacle_kind,
            stationary_class: (obstacle_kind == ObstacleKind::GoalPost)
                .then_some(StationaryClass::GoalPost),
            measurement_count: 1,
            last_update: detection_time,
            last_detection: Some(detection_time),
            teammate: None,
            teammate_evidence: 0,
            last_team_report: None,
        };
        self.next_id += 1;
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
        fall_detection: Option<&FallDetection>,
    ) {
        self.prune_hypotheses(now, parameters.hypothesis_timeout);
        self.hypotheses.retain(|hypothesis| {
            hypothesis.last_detection.is_some()
                || hypothesis
                    .last_team_report
                    .is_some_and(|stamp| measurement_is_recent(now, stamp, TEAM_REPORT_TIMEOUT))
        });
        for hypothesis in &mut self.hypotheses {
            if hypothesis
                .last_team_report
                .is_some_and(|stamp| !measurement_is_recent(now, stamp, TEAM_REPORT_TIMEOUT))
            {
                hypothesis.teammate = None;
                hypothesis.teammate_evidence = 0;
                hypothesis.last_team_report = None;
            }
        }
        let became_unpenalized = self.last_primary_state == PrimaryState::Penalized
            && primary_state != PrimaryState::Penalized;
        self.last_primary_state = primary_state;
        if became_unpenalized {
            self.hypotheses.clear();
        }
        // Retain main's existing SDK posture policy; timestamp-safe obstacle
        // tracking does not introduce the separate physical fall detector.
        let is_upright = fall_detection
            .is_some_and(|state| state.is_valid(now) && state.posture == Posture::Upright);
        if !is_upright {
            self.hypotheses.retain(|hypothesis| {
                hypothesis.obstacle_kind != ObstacleKind::Unknown
                    || hypothesis.stationary_class.is_some()
            });
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
                        !hypothesis.is_moving()
                            && existing_hypothesis.obstacle_kind == hypothesis.obstacle_kind
                            && existing_hypothesis.stationary_class == hypothesis.stationary_class
                            && (existing_hypothesis.state.mean - hypothesis.state.mean).norm()
                                < merge_distance
                    });
            match hypothesis_in_merge_distance {
                Some(existing_hypothesis) => {
                    existing_hypothesis.state.update(
                        Matrix2::identity(),
                        hypothesis.state.mean,
                        hypothesis.state.covariance,
                    );
                    existing_hypothesis.measurement_count = existing_hypothesis
                        .measurement_count
                        .max(hypothesis.measurement_count);
                    existing_hypothesis.last_update =
                        existing_hypothesis.last_update.max(hypothesis.last_update);
                    existing_hypothesis.last_detection = existing_hypothesis
                        .last_detection
                        .max(hypothesis.last_detection);
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
) -> impl Iterator<Item = Measurement> {
    detected_objects.iter().filter_map(|detected_object| {
        let Object {
            label,
            bounding_box,
        } = detected_object;

        let stationary_class = match label {
            RobocupObjectLabel::GoalPost => Some(StationaryClass::GoalPost),
            RobocupObjectLabel::LSpot => Some(StationaryClass::LSpot),
            RobocupObjectLabel::TSpot => Some(StationaryClass::TSpot),
            RobocupObjectLabel::XSpot => Some(StationaryClass::XSpot),
            RobocupObjectLabel::PenaltySpot => Some(StationaryClass::PenaltySpot),
            _ => None,
        };
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
            RobocupObjectLabel::LSpot
            | RobocupObjectLabel::TSpot
            | RobocupObjectLabel::XSpot
            | RobocupObjectLabel::PenaltySpot => (
                ObstacleKind::Unknown,
                parameters.goal_post_measurement_noise,
                parameters.goal_post_confidence_threshold,
            ),
            RobocupObjectLabel::Ball => return None,
        };

        let Rectangle { min, max } = bounding_box.area;
        if ![min.x(), min.y(), max.x(), max.y(), bounding_box.confidence]
            .iter()
            .all(|value| value.is_finite())
            || bounding_box.confidence < confidence_threshold
            || min.x() >= max.x()
            || min.y() >= max.y()
            || min.x() < 0.0
            || max.x() > camera_matrix.image_size.x()
        {
            return None;
        }
        let is_marking = stationary_class.is_some_and(|class| !class.is_obstacle());
        // Clipped feet do not provide ground contact. Wait for a full detection
        // rather than creating a confidently wrong range from the image border.
        if !is_marking && max.y() >= camera_matrix.image_size.y() - 2.0 {
            return None;
        }
        let bottom_center_position = point![
            (min.x() + max.x()) / 2.0,
            if is_marking {
                (min.y() + max.y()) / 2.0
            } else {
                max.y()
            }
        ];
        if bottom_center_position.y() < 0.0
            || bottom_center_position.y() > camera_matrix.image_size.y()
        {
            return None;
        }
        let obstacle_center: Point2<Ground> =
            camera_matrix.pixel_to_ground(bottom_center_position).ok()?;
        let range = obstacle_center.coords().norm();
        if !range.is_finite() || range > if is_marking { 5.0 } else { 15.0 } {
            return None;
        }
        Some(Measurement {
            kind,
            stationary_class,
            position: obstacle_center,
            noise: Matrix2::from_diagonal(&measurement_noise.map(|variance| variance.max(0.0001)))
                * range.powi(2).max(0.01),
        })
    })
}

fn new_network_player_states(
    players: &Players<Option<TimeWrapper<PlayerState>>>,
    own_player_number: Option<PlayerNumber>,
    last_processed_player_state_times: &mut Players<Option<Time>>,
    now: Time,
    maximum_age: Duration,
) -> Vec<(PlayerNumber, Time, PlayerState)> {
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
            if player_state
                .inner
                .pose_age
                .unwrap_or(Duration::from_millis(300))
                .saturating_add(now.duration_since(player_state.time))
                > TEAM_REPORT_TIMEOUT
            {
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

            Some((player_number, player_state.time, player_state.inner))
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
    use linear_algebra::{IntoTransform, Isometry3, vector};
    use ros_z_streams::CreateAnnouncingPublisher;
    use types::obstacles::ObstacleKind;

    fn camera() -> CameraMatrix {
        let head_to_camera = (na::Isometry3::rotation(na::Vector3::x() * 0.4)
            * na::Isometry3::rotation(na::Vector3::y() * -std::f32::consts::FRAC_PI_2)
            * na::Isometry3::rotation(na::Vector3::x() * std::f32::consts::FRAC_PI_2))
        .framed_transform();
        CameraMatrix::from_normalized_focal_and_center(
            na::vector![0.5, 0.5 * 640.0 / 480.0],
            na::point![0.5, 0.5],
            vector![640.0, 480.0],
            na::Isometry3::translation(0.0, 0.0, -0.8).framed_transform(),
            Isometry3::identity(),
            head_to_camera,
        )
    }

    fn detection(
        camera: &CameraMatrix,
        label: RobocupObjectLabel,
        position: Point2<Ground>,
    ) -> Object<RobocupObjectLabel> {
        let pixel = camera.ground_to_pixel(position).unwrap();
        let marking = !matches!(
            label,
            RobocupObjectLabel::Robot | RobocupObjectLabel::GoalPost
        );
        Object {
            label,
            bounding_box: types::bounding_box::BoundingBox {
                area: Rectangle {
                    min: point![
                        pixel.x() - 3.0,
                        pixel.y() - if marking { 3.0 } else { 30.0 }
                    ],
                    max: point![pixel.x() + 3.0, pixel.y() + if marking { 3.0 } else { 0.0 }],
                },
                confidence: 0.9,
            },
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn live_node_publishes_confirmed_opponent_velocity() -> Result<()> {
        let context = Arc::new(
            ContextBuilder::default()
                .with_mode("peer")
                .disable_multicast_scouting()
                .with_connect_endpoints(std::iter::empty::<&str>())
                .with_listen_endpoints(["tcp/127.0.0.1:0"])
                .with_parameter_layers([std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../etc/parameters/base")])
                .build()
                .await?,
        );
        let fixture = context.create_node("opponent_track_test").build().await?;
        let retained = QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        };
        let field = fixture
            .publisher::<FieldDimensions>("field_dimensions")
            .qos(retained.clone())
            .build()
            .await?;
        let primary = fixture
            .publisher::<PrimaryState>("primary_state")
            .qos(retained)
            .build()
            .await?;
        let pose = fixture
            .publisher::<Isometry2<Ground, Field>>("ground_to_field")
            .build()
            .await?;
        let odometry = fixture
            .announcing_publisher::<Pose2<Odometry>>("inputs/odometry")
            .await?;
        let cameras = fixture
            .publisher::<TimeWrapper<CameraMatrix>>("camera_matrix")
            .build()
            .await?;
        let detections = fixture
            .announcing_publisher::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>(
                "detected_objects",
            )
            .await?;
        let tracks = fixture
            .subscriber::<TrackedObjects<HumanoidTrack>>("obstacle_filter/humanoid_tracks")
            .build()
            .await?;
        field.publish(&FieldDimensions::SPL_2025).await?;
        primary.publish(&PrimaryState::Playing).await?;
        let mut nodes = tokio::task::JoinSet::new();
        nodes.spawn(run(context));
        // A timer output confirms that all of the real filter's inputs are registered.
        tokio::time::timeout(Duration::from_secs(5), tracks.recv()).await??;
        let camera = camera();
        let start = fixture.clock().now();
        let mut tick = tokio::time::interval(Duration::from_millis(10));
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        let time = fixture.clock().now();
                        let seconds = time.duration_since(start).as_secs_f32();
                        pose.publish_with_source_time(&Isometry2::identity(), time).await?;
                        cameras.publish(&TimeWrapper { time, inner: camera.clone() }).await?;
                        odometry.announce(time).await?.publish(&Pose2::default()).await?;
                        detections.announce(time).await?.publish(&TimeWrapper {
                            time,
                            inner: vec![detection(&camera, RobocupObjectLabel::Robot,
                                point![3.0, 0.4 - seconds * 0.3])],
                        }).await?;
                    }
                    output = tracks.recv() => {
                        let output = output?;
                        if output.tracks.iter().any(|track| {
                            track.status == TrackStatus::Confirmed
                                && track.role == HumanoidRole::Opponent
                                && track.position_in_field.is_some()
                                && track.velocity_in_ground.y() < -0.05
                        }) {
                            break Ok::<(), color_eyre::Report>(());
                        }
                    }
                    exited = nodes.join_next() => panic!("obstacle filter exited: {exited:?}"),
                }
            }
        })
        .await??;
        nodes.abort_all();
        while nodes.join_next().await.is_some() {}
        Ok(())
    }

    #[test]
    fn mixed_detections_keep_ids_velocity_and_stationary_classes_while_robot_moves() {
        let mut filter = ObstacleFilter::default();
        let parameters = ObstacleFilterParameters {
            use_detected_objects: true,
            hypothesis_timeout: Duration::from_secs(30),
            object_detection_measurement_matching_distance: 1.0,
            robot_measurement_noise: na::vector![0.002, 0.002],
            goal_post_measurement_noise: na::vector![0.002, 0.002],
            process_noise: na::vector![0.0001, 0.0001],
            measurement_count_threshold: 2,
            ..Default::default()
        };
        let camera = camera();
        let start = Time::from_nanos(1_000_000_000);
        let mut first_ids = Vec::new();
        for frame in 0..90 {
            let seconds = frame as f32 / 30.0;
            let now = start + Duration::from_secs_f32(seconds);
            let pose = Pose2::new(point![seconds * 0.1, 0.0], seconds * 0.08);
            let odometry_to_ground = pose.as_transform::<Ground>().inverse();
            let measurements = [
                (RobocupObjectLabel::Robot, point![3.0, 0.4 - seconds * 0.3]),
                (RobocupObjectLabel::GoalPost, point![3.1, 0.4]),
                (RobocupObjectLabel::LSpot, point![3.0, -0.5]),
            ]
            .map(|(label, position)| detection(&camera, label, odometry_to_ground * position));
            assert!(filter.process_detection(
                now,
                &parameters,
                &measurements,
                Some(&camera),
                Some(pose)
            ));
            assert_eq!(filter.hypotheses.len(), 3);
            let ids: Vec<_> = filter.hypotheses.iter().map(|track| track.id).collect();
            if frame == 0 {
                first_ids = ids;
            } else {
                assert_eq!(ids, first_ids);
            }
        }
        let now = filter.ground_frame.unwrap().0;
        let pose = filter.ground_frame.unwrap().1;
        let (humanoids, stationary) = filter.tracked_objects(now, None, &parameters, true);
        assert_eq!(humanoids.time, now);
        assert_eq!(humanoids.tracks.len(), 1);
        assert_eq!(stationary.tracks.len(), 2);
        let robot = &humanoids.tracks[0];
        assert_eq!(robot.role, HumanoidRole::Opponent);
        assert_eq!(robot.status, TrackStatus::Confirmed);
        assert_eq!(robot.time_since_detection, Some(Duration::ZERO));
        assert!((robot.state.mean.fixed_rows::<2>(2) - na::vector![0.0, -0.3]).norm() < 0.05);
        assert!(
            (robot.state.mean.fixed_rows::<2>(0) - na::vector![3.0, 0.4 - 89.0 / 30.0 * 0.3])
                .norm()
                < 0.05
        );
        let velocity_in_odometry =
            humanoids.ground_to_odometry.inner.rotation * robot.velocity_in_ground.inner;
        assert!((velocity_in_odometry - na::vector![0.0, -0.3]).norm() < 0.05);
        let post = stationary
            .tracks
            .iter()
            .find(|track| track.class == StationaryClass::GoalPost)
            .unwrap();
        assert!((post.state.mean - na::vector![3.1, 0.4]).norm() < 0.001);
        assert!(post.is_obstacle);
        assert!(
            !stationary
                .tracks
                .iter()
                .find(|track| track.class == StationaryClass::LSpot)
                .unwrap()
                .is_obstacle
        );
        let obstacles = filter.compose_outputs(now, &parameters, &FieldDimensions::SPL_2025, None);
        assert_eq!(obstacles.len(), 2);
        // Predictions in a fresh frame carry physical velocity, without claiming new evidence.
        let later = now + Duration::from_millis(200);
        filter.advance_ground_frame(later, pose, na::Vector2::zeros());
        let (predicted, _) = filter.tracked_objects(later, None, &parameters, true);
        assert_eq!(predicted.tracks[0].id, robot.id);
        assert_eq!(
            predicted.tracks[0].time_since_detection,
            Some(Duration::from_millis(200))
        );
        assert!((predicted.tracks[0].state.mean.y - (robot.state.mean.y - 0.06)).abs() < 0.02);
    }

    #[test]
    fn nearby_robots_receive_one_measurement_each_and_keep_distinct_ids() {
        let mut filter = ObstacleFilter::default();
        let time = Time::from_nanos(1_000_000_000);
        for x in [2.0, 2.4] {
            filter.spawn_hypothesis(
                point![x, 0.0],
                ObstacleKind::Robot,
                time,
                Matrix2::identity(),
            );
        }
        let measurements = [2.35, 2.05].map(|x| Measurement {
            kind: ObstacleKind::Robot,
            stationary_class: None,
            position: point![x, 0.0],
            noise: Matrix2::identity(),
        });
        filter.associate_measurements(time + Duration::from_millis(30), &measurements, 1.0);
        filter.merge_hypotheses_if_needed(1.0);
        assert_eq!(filter.hypotheses.len(), 2);
        assert_ne!(filter.hypotheses[0].id, filter.hypotheses[1].id);
        assert!(
            filter
                .hypotheses
                .iter()
                .all(|track| track.measurement_count == 2)
        );
        assert!(filter.hypotheses[0].state.mean.x < filter.hypotheses[1].state.mean.x);
    }

    #[test]
    fn dormant_moving_track_keeps_identity_without_remaining_a_blocking_obstacle() {
        let mut filter = ObstacleFilter::default();
        let start = Time::from_nanos(1_000_000_000);
        let parameters = ObstacleFilterParameters {
            hypothesis_timeout: Duration::from_secs(30),
            ..Default::default()
        };
        filter.advance_ground_frame(start, Pose2::zero(), na::Vector2::zeros());
        for kind in [ObstacleKind::Robot, ObstacleKind::GoalPost] {
            filter.spawn_hypothesis(point![3.0, 0.0], kind, start, Matrix2::identity() * 0.01);
        }
        let robot_id = filter.hypotheses[0].id;
        for (age, expected_obstacles) in [(499, 2), (501, 1), (1100, 1)] {
            let now = start + Duration::from_millis(age);
            filter.advance_ground_frame(now, Pose2::zero(), na::Vector2::zeros());
            let output = filter.output(
                now,
                &parameters,
                Some(&FieldDimensions::SPL_2025),
                None,
                PrimaryState::Playing,
                None,
                false,
            );
            assert_eq!(output.obstacles.len(), expected_obstacles);
            assert_eq!(output.humanoid_tracks.tracks[0].id, robot_id);
            if age > 1000 {
                assert_eq!(
                    output.humanoid_tracks.tracks[0].status,
                    TrackStatus::Dormant
                );
            }
        }
        // A team association may renew identity, but not stale visual coordinates.
        filter.hypotheses[0].last_team_report = Some(start + Duration::from_millis(1100));
        let obstacles = filter.compose_outputs(
            start + Duration::from_millis(1100),
            &parameters,
            &FieldDimensions::SPL_2025,
            None,
        );
        assert_eq!(obstacles.len(), 1);
        let now = start + Duration::from_millis(1200);
        filter.advance_ground_frame(now, Pose2::zero(), na::Vector2::zeros());
        filter.associate_measurements(
            now,
            &[Measurement {
                kind: ObstacleKind::Robot,
                stationary_class: None,
                position: point![3.0, 0.0],
                noise: Matrix2::identity() * 0.01,
            }],
            1.0,
        );
        let output = filter.output(
            now,
            &parameters,
            Some(&FieldDimensions::SPL_2025),
            None,
            PrimaryState::Playing,
            None,
            false,
        );
        assert_eq!(output.obstacles.len(), 2);
        assert_eq!(output.humanoid_tracks.tracks[0].id, robot_id);
        assert_eq!(
            output.humanoid_tracks.tracks[0].status,
            TrackStatus::Confirmed
        );
    }

    #[test]
    fn team_reports_need_unique_repeated_evidence_and_expire_without_reclassifying_remote_tracks() {
        let mut filter = ObstacleFilter::default();
        let time = Time::from_nanos(1_000_000_000);
        let parameters = ObstacleFilterParameters {
            network_robot_measurement_matching_distance: 0.75,
            hypothesis_timeout: Duration::from_secs(30),
            ..Default::default()
        };
        filter.advance_ground_frame(time, Pose2::zero(), na::Vector2::zeros());
        for y in [-0.3, 0.3] {
            filter.spawn_hypothesis(
                point![3.0, y],
                ObstacleKind::Robot,
                time,
                Matrix2::identity(),
            );
        }
        let mut report = PlayerState {
            pose: point![3.0, 0.0].into(),
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
        };
        filter.process_network_player_state(
            time,
            &parameters,
            PlayerNumber::Three,
            &report,
            &Isometry2::identity(),
        );
        assert!(
            filter
                .hypotheses
                .iter()
                .all(|track| track.teammate.is_none())
        );
        report.pose = point![3.0, 0.3].into();
        for tick in 0..2 {
            filter.process_network_player_state(
                time + Duration::from_millis(tick * 20),
                &parameters,
                PlayerNumber::Three,
                &report,
                &Isometry2::identity(),
            );
            let (tracks, _) = filter.tracked_objects(time, None, &parameters, true);
            assert_eq!(tracks.tracks[0].role, HumanoidRole::Opponent);
            assert_eq!(
                tracks.tracks[1].role,
                if tick == 0 {
                    HumanoidRole::Opponent
                } else {
                    HumanoidRole::Teammate(PlayerNumber::Three)
                }
            );
        }
        // Replayed reports cannot promote a track or collapse camera covariance.
        assert_eq!(filter.hypotheses[1].state.covariance, Matrix2::identity());
        filter.process_network_player_state(
            time + Duration::from_millis(20),
            &parameters,
            PlayerNumber::Three,
            &report,
            &Isometry2::identity(),
        );
        assert_eq!(filter.hypotheses[1].teammate_evidence, 2);
        report.pose = point![-3.0, 0.0].into();
        report.pose_age = Some(Duration::MAX);
        filter.process_network_player_state(
            time,
            &parameters,
            PlayerNumber::Four,
            &report,
            &Isometry2::identity(),
        );
        assert_eq!(filter.hypotheses.len(), 2);
        report.pose_age = None;
        filter.process_network_player_state(
            time,
            &parameters,
            PlayerNumber::Four,
            &report,
            &Isometry2::identity(),
        );
        assert_eq!(filter.hypotheses.len(), 3);
        assert!(filter.hypotheses[2].last_detection.is_none());
        filter.maintain(
            time + Duration::from_secs(4),
            &parameters,
            PrimaryState::Playing,
            None,
        );
        assert_eq!(filter.hypotheses.len(), 2);
        assert!(
            filter
                .hypotheses
                .iter()
                .all(|track| track.teammate.is_none())
        );
    }

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
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
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
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
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
                ObstacleKind::GoalPost,
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
        filter.associate_measurements(
            time,
            &[Measurement {
                position: point![3.0, 0.0],
                kind: ObstacleKind::GoalPost,
                stationary_class: None,
                noise: Matrix2::identity(),
            }],
            0.1,
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
            ObstacleKind::GoalPost,
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
        filter.maintain(time, &parameters, PrimaryState::Damping, None);
        let obstacles = filter.compose_outputs(time, &parameters, &FieldDimensions::SPL_2025, None);
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
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
        };
        filter.process_network_player_state(
            frame_time + Duration::from_millis(50),
            &ObstacleFilterParameters::default(),
            PlayerNumber::Four,
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
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
        };

        let position = measured_player_position(&player_state, &ground_to_field);

        assert_eq!(position, linear_algebra::point![2.0, 3.0]);
    }

    #[test]
    fn player_state_snapshots_extract_each_measurement_once() {
        use hsl_network_messages::PlayerNumber;

        let own_player_state = PlayerState {
            pose: linear_algebra::point![1.0, 2.0].into(),
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
        };
        let teammate_state = PlayerState {
            pose: linear_algebra::point![3.0, 4.0].into(),
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
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
        assert_eq!(entries[0].1, Time::from_nanos(4));
        assert_eq!(entries[0].2.pose.position(), teammate_state.pose.position());
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
            head_yaw: 0.0,
            pose_age: None,
            ball_position: None,
            pass: None,
            replacement_goalkeeper_claim: false,
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
        assert_eq!(newer_entries[0].1, Time::from_nanos(6));
        assert_eq!(
            newer_entries[0].2.pose.position(),
            newer_teammate_state.pose.position()
        );
    }
}

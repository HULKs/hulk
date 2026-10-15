use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Isometry2, Point2};
use ros_z::{
    prelude::*,
    qos::{QosDurability, QosHistory},
    time::Time,
};
use search_heatmap::{
    Heatmap, SearchArea, SearchOccluder, SearchVoronoiSelection, ball_position_near_field,
};
use std::{boxed::Box, future::Future, num::NonZeroUsize, pin::Pin, sync::Arc, time::Duration};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    obstacles::{Obstacle, ObstacleKind},
    parameters::SearchSuggestorParameters,
    players::Players,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    world_state::PlayerState,
};

const MAXIMUM_MEAN_PRIORITY_AT_PLAYING_START: f32 = 0.5;

#[derive(Clone, Copy)]
struct TeammateFreshness {
    now: Time,
    window: Option<Duration>,
}

impl TeammateFreshness {
    fn contains(self, state_time: Time) -> bool {
        self.now >= state_time
            && self
                .window
                .is_some_and(|window| self.now.duration_since(state_time) <= window)
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("search_suggestor").build().await?;

    let parameters = node.bind_parameter_as::<SearchSuggestorParameters>("search_suggestor")?;
    parameters.add_validation_hook(validate_search_suggestor_parameters)?;
    let field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let ball_position_sub = node
        .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
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
    let goalkeeper_player_number_cache = node
        .subscriber::<PlayerNumber>("behavior/goalkeeper_player_number")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(NonZeroUsize::MIN),
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let hsl_state_message_send_interval_cache = node
        .subscriber::<Option<Duration>>("hsl_state_message_send_interval")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(NonZeroUsize::MIN),
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let hypothetical_ball_positions_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>(
            "ball_filter/hypothetical_ball_positions",
        )
        .build()
        .await?;
    let ground_to_field_cache = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
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
    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await?;
    let obstacles_cache = node
        .subscriber::<Vec<Obstacle>>("obstacles")
        .cache(1)
        .build()
        .await?;
    let head_joints_command_cache = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")
        .cache(1)
        .build()
        .await?;
    let last_received_player_states_sub = node
        .subscriber::<Players<Option<TimeWrapper<PlayerState>>>>("last_received_player_states")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(NonZeroUsize::MIN),
            ..Default::default()
        })
        .build()
        .await?;
    let additional_heatmap_pub = node
        .publisher::<types::heatmap::Heatmap>("ball_search_heatmap")
        .build()
        .await?;
    let suggested_search_position_pub = node
        .publisher::<Option<Point2<Field>>>("suggested_search_position")
        .build()
        .await?;

    let field_dimensions = field_dimensions_sub.recv().await?;
    let mut heatmap = Heatmap::new(field_dimensions);
    let mut last_ball_position = None;
    let mut current_ball_position = None;
    let mut latest_filtered_game_controller_state = None;
    let mut last_teammate_states = Players::new(None);
    let mut no_regular_target_since = None;
    let mut was_playing = false;
    let mut was_heatmap_search_enabled = true;
    let mut last_priority_update = node.clock().now();
    let mut tick = node.create_timer(Duration::from_millis(20));

    loop {
        tick.tick().await;
        let now = node.clock().now();
        let elapsed_since_last_priority_update = now.duration_since(last_priority_update);
        last_priority_update = now;

        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        if parameters.disable_heatmap_search {
            if was_heatmap_search_enabled {
                last_ball_position = None;
                current_ball_position = None;
                latest_filtered_game_controller_state = None;
                last_teammate_states = Players::new(None);
                no_regular_target_since = None;
                was_playing = false;
            }
            was_heatmap_search_enabled = false;
            suggested_search_position_pub.publish(&None).await?;

            while ball_position_sub.is_ready() {
                ball_position_sub.recv().await?;
            }
            while hypothetical_ball_positions_sub.is_ready() {
                hypothetical_ball_positions_sub.recv().await?;
            }
            while last_received_player_states_sub.is_ready() {
                last_received_player_states_sub.recv().await?;
            }
            while filtered_game_controller_state_sub.is_ready() {
                filtered_game_controller_state_sub.recv().await?;
            }
            continue;
        }
        if !was_heatmap_search_enabled {
            heatmap = Heatmap::new(field_dimensions);
        }
        was_heatmap_search_enabled = true;
        let teammate_freshness = TeammateFreshness {
            now,
            window: hsl_state_message_send_interval_cache
                .get_latest()
                .and_then(|interval| *interval)
                .map(|interval| interval.mul_f32(1.2)),
        };

        let ground_to_field = ground_to_field_cache
            .get_latest()
            .map(|ground_to_field| *ground_to_field)
            .filter(|ground_to_field| ground_to_field_is_finite(*ground_to_field));
        let primary_state = primary_state_cache.get_latest();
        let primary_state = primary_state.as_deref();
        let is_playing = primary_state == Some(&PrimaryState::Playing);
        let entered_playing = is_playing && !was_playing;
        if entered_playing {
            heatmap.limit_mean_priority(MAXIMUM_MEAN_PRIORITY_AT_PLAYING_START);
        }
        was_playing = is_playing;
        let mut ball_was_seen = false;
        let mut received_filtered_game_controller_state = false;

        while ball_position_sub.is_ready() {
            match (ball_position_sub.recv().await?, ground_to_field) {
                (Some(ball_position), Some(ground_to_field)) => {
                    if !ball_position.position.x().is_finite()
                        || !ball_position.position.y().is_finite()
                    {
                        continue;
                    }
                    let ball_position = ground_to_field * ball_position.position;
                    if ball_position_near_field(field_dimensions, ball_position) {
                        ball_was_seen = true;
                        last_ball_position = Some(ball_position);
                        current_ball_position = Some(ball_position);
                    } else {
                        current_ball_position = None;
                    }
                }
                (None, _) | (Some(_), None) => {
                    current_ball_position = None;
                }
            }
        }
        while hypothetical_ball_positions_sub.is_ready() {
            if let Some(ground_to_field) = ground_to_field {
                let hypothetical_ball_positions = hypothetical_ball_positions_sub.recv().await?;
                heatmap.update_with_hypothetical_ball_positions(
                    field_dimensions,
                    &hypothetical_ball_positions,
                    ground_to_field,
                    parameters,
                );
            } else {
                hypothetical_ball_positions_sub.recv().await?;
            }
        }
        while last_received_player_states_sub.is_ready() {
            let player_states = last_received_player_states_sub.recv().await?;
            update_from_player_states(
                &mut heatmap,
                field_dimensions,
                &player_states,
                &mut last_teammate_states,
                parameters,
                teammate_freshness,
                elapsed_since_last_priority_update,
            );
        }
        while filtered_game_controller_state_sub.is_ready() {
            let filtered_game_controller_state = filtered_game_controller_state_sub.recv().await?;
            received_filtered_game_controller_state = true;
            latest_filtered_game_controller_state = Some(filtered_game_controller_state);
        }

        let restart_regenerated = latest_filtered_game_controller_state.as_ref().is_some_and(
            |filtered_game_controller_state| {
                heatmap.regenerate_restart_hypotheses(
                    filtered_game_controller_state,
                    field_dimensions,
                    parameters.rule_ball_weight_increment,
                )
            },
        );
        if !restart_regenerated
            && received_filtered_game_controller_state
            && let (Some(filtered_game_controller_state), Some(primary_state)) =
                (&latest_filtered_game_controller_state, primary_state)
        {
            heatmap.update_with_rule_ball(
                filtered_game_controller_state,
                field_dimensions,
                *primary_state,
                parameters,
            );
        }

        if !restart_regenerated
            && is_playing
            && current_ball_position.is_none()
            && let Some(last_ball_position) = last_ball_position
        {
            heatmap.increase_around_last_ball(
                field_dimensions,
                last_ball_position,
                if entered_playing {
                    Duration::ZERO
                } else {
                    elapsed_since_last_priority_update
                },
                parameters,
            );
        }

        if current_ball_position.is_none()
            && !ball_was_seen
            && let Some(ground_to_field) = ground_to_field
        {
            let occluders = obstacles_cache
                .get_latest()
                .map(|obstacles| {
                    search_occluders_from_obstacles(obstacles.as_ref(), ground_to_field)
                })
                .unwrap_or_default();
            let head_yaw = head_joints_command_cache
                .get_latest()
                .map_or(0.0, |head_joints| head_joints.yaw);
            if head_yaw.is_finite() {
                heatmap.decay_tiles_in_robot_fov_with_occluders(
                    field_dimensions,
                    ground_to_field,
                    head_yaw,
                    parameters,
                    &occluders,
                );
            }
        }

        if let Some(current_ball_position) = current_ball_position {
            heatmap.set_known_ball_position(field_dimensions, current_ball_position);
        }

        heatmap.clamp_values();

        let own_voronoi_site = ground_to_field.zip(
            player_number_cache
                .get_latest()
                .map(|player_number| *player_number),
        );
        let search_area = own_voronoi_site.as_ref().and_then(|(_, player_number)| {
            goalkeeper_player_number_cache
                .get_latest()
                .map(|goalkeeper_player_number| {
                    search_area_for_player(*player_number, *goalkeeper_player_number)
                })
        });
        if last_ball_position.is_none() {
            no_regular_target_since = None;
            heatmap.clear_suggested_search_position();
        } else if let Some((voronoi_selection, search_area)) =
            search_voronoi_selection_from_teammates(
                field_dimensions,
                own_voronoi_site,
                &last_teammate_states,
                teammate_freshness,
            )
            .zip(search_area)
        {
            let has_regular_target = heatmap.has_regular_search_target(
                field_dimensions,
                parameters,
                &voronoi_selection,
                search_area,
            );
            let minimum_validity = search_minimum_validity(
                &mut no_regular_target_since,
                now,
                current_ball_position.is_none() && is_playing,
                has_regular_target,
                parameters,
            );
            heatmap.update_suggested_search_position_with_voronoi(
                field_dimensions,
                parameters,
                ground_to_field,
                &voronoi_selection,
                minimum_validity,
                search_area,
            );
        } else {
            no_regular_target_since = None;
            heatmap.clear_suggested_search_position();
        }

        let suggested_search_position =
            ground_to_field
                .zip(search_area)
                .and_then(|(ground_to_field, search_area)| {
                    heatmap.selected_position(
                        field_dimensions,
                        ground_to_field.translation(),
                        search_area,
                    )
                });
        suggested_search_position_pub
            .publish(&suggested_search_position)
            .await?;

        additional_heatmap_pub
            .publish_if_subscribed(|| async { heatmap.to_message() })
            .await?;
    }
}

fn search_area_for_player(
    player_number: PlayerNumber,
    goalkeeper_player_number: PlayerNumber,
) -> SearchArea {
    if player_number == goalkeeper_player_number {
        SearchArea::OwnHalf
    } else {
        SearchArea::WholeField
    }
}

fn validate_search_suggestor_parameters(
    parameters: &SearchSuggestorParameters,
) -> std::result::Result<(), String> {
    if !parameters.minimum_validity.is_finite()
        || !(0.0..=1.0).contains(&parameters.minimum_validity)
    {
        return Err(format!(
            "minimum_validity must be finite and between 0 and 1 (got {})",
            parameters.minimum_validity
        ));
    }
    if !parameters.exhausted_search_minimum_validity.is_finite()
        || !(0.0..=parameters.minimum_validity)
            .contains(&parameters.exhausted_search_minimum_validity)
    {
        return Err(format!(
            "exhausted_search_minimum_validity must be finite and between 0 and minimum_validity (got {})",
            parameters.exhausted_search_minimum_validity
        ));
    }
    Ok(())
}

fn ground_to_field_is_finite(ground_to_field: Isometry2<Ground, Field>) -> bool {
    ground_to_field.translation().x().is_finite()
        && ground_to_field.translation().y().is_finite()
        && ground_to_field.orientation().angle().is_finite()
}

fn search_minimum_validity(
    no_regular_target_since: &mut Option<Time>,
    now: Time,
    can_use_fallback: bool,
    has_regular_target: bool,
    parameters: &SearchSuggestorParameters,
) -> f32 {
    if !can_use_fallback || has_regular_target {
        *no_regular_target_since = None;
        return parameters.minimum_validity;
    }

    let since = no_regular_target_since.get_or_insert(now);
    if now < *since {
        *since = now;
    }
    if now.duration_since(*since) >= parameters.exhausted_search_delay {
        parameters.exhausted_search_minimum_validity
    } else {
        parameters.minimum_validity
    }
}

fn search_occluders_from_obstacles(
    obstacles: &[Obstacle],
    ground_to_field: Isometry2<Ground, Field>,
) -> Vec<SearchOccluder> {
    obstacles
        .iter()
        .filter(|obstacle| obstacle.kind != ObstacleKind::Ball)
        .map(|obstacle| SearchOccluder {
            center: ground_to_field * obstacle.position,
            radius: obstacle
                .radius_at_foot_height
                .max(obstacle.radius_at_hip_height),
        })
        .filter(|occluder| {
            occluder.center.x().is_finite()
                && occluder.center.y().is_finite()
                && occluder.radius.is_finite()
                && occluder.radius > 0.0
        })
        .collect()
}

fn search_voronoi_selection_from_teammates(
    field_dimensions: FieldDimensions,
    own_voronoi_site: Option<(Isometry2<Ground, Field>, PlayerNumber)>,
    last_teammate_states: &Players<Option<TimeWrapper<PlayerState>>>,
    teammate_freshness: TeammateFreshness,
) -> Option<SearchVoronoiSelection> {
    let (ground_to_field, own_player_number) = own_voronoi_site?;
    let mut sites = vec![(ground_to_field.as_pose(), own_player_number)];

    for (player_number, teammate_state) in last_teammate_states.iter() {
        let Some(teammate_state) = teammate_state else {
            continue;
        };
        if player_number == own_player_number {
            continue;
        }
        if teammate_freshness.contains(teammate_state.time) {
            sites.push((teammate_state.inner.pose, player_number));
        }
    }

    Some(SearchVoronoiSelection::new(
        field_dimensions,
        own_player_number,
        sites,
    ))
}

fn update_from_player_states(
    heatmap: &mut Heatmap,
    field_dimensions: FieldDimensions,
    player_states: &Players<Option<TimeWrapper<PlayerState>>>,
    last_teammate_states: &mut Players<Option<TimeWrapper<PlayerState>>>,
    parameters: &SearchSuggestorParameters,
    teammate_freshness: TeammateFreshness,
    local_tick_duration: Duration,
) {
    for (player_number, current) in player_states.iter() {
        let Some(current) = current.as_ref().filter(|state| state.inner.is_finite()) else {
            last_teammate_states[player_number] = None;
            continue;
        };
        let previous = last_teammate_states[player_number].as_ref();
        if previous.is_some_and(|previous| current.time <= previous.time) {
            continue;
        }

        let is_fresh = teammate_freshness.contains(current.time);
        if is_fresh && let Some(ball) = current.inner.ball_position {
            heatmap.update_with_team_ball(field_dimensions, ball.position, parameters);
        }
        if is_fresh
            && let Some(freshness_window) = teammate_freshness.window
            && let Some(previous) = previous
            && let elapsed_since_previous_message = current.time.duration_since(previous.time)
            && elapsed_since_previous_message <= freshness_window
        {
            let tick_count = replay_tick_count(elapsed_since_previous_message, local_tick_duration);
            heatmap.decay_tiles_from_teammate_motion(
                field_dimensions,
                &previous.inner,
                &current.inner,
                tick_count,
                parameters,
            );
        }
        last_teammate_states[player_number] = Some(current.clone());
    }
}

fn replay_tick_count(message_elapsed: Duration, local_tick_duration: Duration) -> usize {
    let local_tick_duration = local_tick_duration.max(Duration::from_millis(1));
    (message_elapsed.as_secs_f32() / local_tick_duration.as_secs_f32())
        .ceil()
        .max(1.0) as usize
}

#[cfg(test)]
mod tests;

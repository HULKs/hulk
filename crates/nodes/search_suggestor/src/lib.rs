use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::{HulkMessage, PlayerNumber, StateMessage};
use linear_algebra::{Isometry2, Point2};
use ros_z::{prelude::*, qos::QosDurability};
use search_heatmap::{Heatmap, SearchOccluder, SearchVoronoiSelection};
use std::{boxed::Box, future::Future, pin::Pin, sync::Arc, time::Duration};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage,
    obstacles::{Obstacle, ObstacleKind},
    parameters::{BehaviorParameters, HslNetworkParameters, SearchSuggestorParameters},
    players::Players,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    time_wrapper::TimeWrapper,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("search_suggestor").build().await?;

    let behavior_parameters = node.bind_parameter_as::<BehaviorParameters>("behavior_node")?;
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
    let rule_obstacles_cache = node
        .subscriber::<Vec<RuleObstacle>>("rule_obstacles")
        .cache(1)
        .build()
        .await?;
    let network_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")
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
    let mut last_teammate_messages = Players::new(None);
    let mut last_priority_update = node.clock().now();
    let mut tick = node.create_timer(Duration::from_millis(20));

    loop {
        tick.tick().await;
        let now = node.clock().now();
        let elapsed_since_last_priority_update = now.duration_since(last_priority_update);
        last_priority_update = now;

        let behavior_parameters_snapshot = behavior_parameters.snapshot();
        let behavior_parameters = behavior_parameters_snapshot.typed();
        let parameters = &behavior_parameters.search_suggestor;
        let hsl_network_parameters = &behavior_parameters.hsl_network;

        let ground_to_field = ground_to_field_cache
            .get_latest()
            .map(|ground_to_field| *ground_to_field);
        let primary_state = primary_state_cache.get_latest();
        let primary_state = primary_state.as_deref();
        let mut ball_was_seen = false;
        let mut received_filtered_game_controller_state = None;

        while ball_position_sub.is_ready() {
            match (ball_position_sub.recv().await?, ground_to_field) {
                (Some(ball_position), Some(ground_to_field)) => {
                    ball_was_seen = true;
                    let ball_position = ground_to_field * ball_position.position;
                    last_ball_position = Some(ball_position);
                    current_ball_position = Some(ball_position);
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
        while network_message_sub.is_ready() {
            let network_message = network_message_sub.recv().await?;
            heatmap.update_with_team_ball(field_dimensions, network_message.clone(), parameters);
            decay_with_teammate_message(
                &mut heatmap,
                field_dimensions,
                network_message,
                &mut last_teammate_messages,
                parameters,
                hsl_network_parameters,
                elapsed_since_last_priority_update,
            );
        }
        while filtered_game_controller_state_sub.is_ready() {
            let filtered_game_controller_state = filtered_game_controller_state_sub.recv().await?;
            received_filtered_game_controller_state = Some(filtered_game_controller_state.clone());
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
            && let (Some(filtered_game_controller_state), Some(primary_state)) =
                (&received_filtered_game_controller_state, primary_state)
        {
            heatmap.update_with_rule_ball(
                filtered_game_controller_state,
                field_dimensions,
                *primary_state,
                parameters,
            );
        }

        if !restart_regenerated
            && current_ball_position.is_none()
            && let Some(last_ball_position) = last_ball_position
        {
            heatmap.increase_around_last_ball(
                field_dimensions,
                last_ball_position,
                elapsed_since_last_priority_update,
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
            heatmap.decay_tiles_in_robot_fov_with_occluders(
                field_dimensions,
                ground_to_field,
                parameters,
                &occluders,
            );
        }

        if let Some(current_ball_position) = current_ball_position {
            heatmap.set_known_ball_position(field_dimensions, current_ball_position);
        }

        heatmap.clamp_values();

        let obstacle_snapshot = obstacles_cache.get_latest();
        let obstacles = obstacle_snapshot
            .as_deref()
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let rule_obstacle_snapshot = rule_obstacles_cache.get_latest();
        let rule_obstacles = rule_obstacle_snapshot
            .as_deref()
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let own_voronoi_site = ground_to_field.zip(
            player_number_cache
                .get_latest()
                .map(|player_number| *player_number),
        );
        if let Some(voronoi_selection) = search_voronoi_selection_from_teammates(
            field_dimensions,
            own_voronoi_site,
            &last_teammate_messages,
            obstacles,
            rule_obstacles,
            now,
            hsl_network_parameters
                .hsl_state_message_send_interval
                .mul_f32(1.2),
        ) {
            heatmap.update_suggested_search_position_with_voronoi(
                field_dimensions,
                parameters,
                ground_to_field,
                &voronoi_selection,
            );
        } else {
            heatmap.clear_suggested_search_position();
        }

        suggested_search_position_pub
            .publish(&heatmap.selected_position(field_dimensions))
            .await?;

        additional_heatmap_pub
            .publish_if_subscribed(|| async { heatmap.to_message() })
            .await?;
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
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct TeammateStateMessage {
    received_at: ros_z::time::Time,
    state: StateMessage,
}

fn search_voronoi_selection_from_teammates(
    field_dimensions: FieldDimensions,
    own_voronoi_site: Option<(Isometry2<Ground, Field>, PlayerNumber)>,
    last_teammate_messages: &Players<Option<TeammateStateMessage>>,
    obstacles: &[Obstacle],
    rule_obstacles: &[RuleObstacle],
    now: ros_z::time::Time,
    freshness_window: Duration,
) -> Option<SearchVoronoiSelection> {
    let (ground_to_field, own_player_number) = own_voronoi_site?;
    let mut sites = vec![(ground_to_field.as_pose(), own_player_number)];

    for (player_number, teammate_message) in last_teammate_messages.iter() {
        let Some(teammate_message) = teammate_message else {
            continue;
        };
        if player_number == own_player_number || now < teammate_message.received_at {
            continue;
        }
        if now.duration_since(teammate_message.received_at) <= freshness_window {
            sites.push((teammate_message.state.pose, player_number));
        }
    }

    Some(SearchVoronoiSelection::new_with_obstacles(
        field_dimensions,
        own_player_number,
        sites,
        obstacles,
        rule_obstacles,
        ground_to_field,
    ))
}

fn decay_with_teammate_message(
    heatmap: &mut Heatmap,
    field_dimensions: FieldDimensions,
    message: TimeWrapper<IncomingMessage>,
    last_teammate_messages: &mut Players<Option<TeammateStateMessage>>,
    parameters: &SearchSuggestorParameters,
    hsl_network_parameters: &HslNetworkParameters,
    local_tick_duration: Duration,
) {
    let TimeWrapper {
        time,
        inner: IncomingMessage::Hsl(HulkMessage::State(state)),
    } = message
    else {
        return;
    };

    if let Some(previous) = last_teammate_messages[state.player_number]
        && time >= previous.received_at
        && let elapsed_since_previous_message = time.duration_since(previous.received_at)
        && elapsed_since_previous_message
            <= hsl_network_parameters
                .hsl_state_message_send_interval
                .mul_f32(1.2)
    {
        let tick_count = replay_tick_count(elapsed_since_previous_message, local_tick_duration);
        heatmap.decay_tiles_from_teammate_motion(
            field_dimensions,
            &previous.state,
            &state,
            tick_count,
            parameters,
        );
    }

    last_teammate_messages[state.player_number] = Some(TeammateStateMessage {
        received_at: time,
        state,
    });
}

fn replay_tick_count(message_elapsed: Duration, local_tick_duration: Duration) -> usize {
    let local_tick_duration = local_tick_duration.max(Duration::from_millis(1));
    (message_elapsed.as_secs_f32() / local_tick_duration.as_secs_f32())
        .ceil()
        .max(1.0) as usize
}

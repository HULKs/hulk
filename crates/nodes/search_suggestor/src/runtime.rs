mod topics;

use color_eyre::Result;
use ros_z::prelude::*;
use search_heatmap::{Heatmap, SearchVoronoiCache};
use std::{sync::Arc, time::Duration};
use topics::Topics;
use types::{
    heatmap::grid_cell_size, parameters::SearchSuggestorParameters, players::Players,
    primary_state::PrimaryState,
};

use crate::{
    priority::{PriorityUpdate, observe_local_ball, update_priorities},
    selection::{TargetSelection, ground_to_field_is_finite, select_search_position},
    team::{TeammateFreshness, update_from_player_states},
    validation::validate_search_suggestor_parameters,
};

const MAXIMUM_MEAN_PRIORITY_AT_PLAYING_START: f32 = 0.5;

pub(crate) async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("search_suggestor").build().await?;

    let parameters = node.bind_parameter_as::<SearchSuggestorParameters>("search_suggestor")?;
    parameters.add_validation_hook(validate_search_suggestor_parameters)?;
    let topics = Topics::build(&node).await?;

    let field_dimensions = topics.field_dimensions_sub.recv().await?;
    let mut heatmap = Heatmap::new(
        field_dimensions,
        parameters.snapshot().typed().cells_along_field_length,
    );
    let mut published_cell_size = grid_cell_size(
        field_dimensions,
        parameters.snapshot().typed().cells_along_field_length,
    );
    topics.cell_size_pub.publish(&published_cell_size).await?;
    let mut last_ball_position = None;
    let mut current_ball_position = None;
    let mut latest_filtered_game_controller_state = None;
    let mut last_teammate_states = Players::new(None);
    let mut voronoi_cache = SearchVoronoiCache::default();
    let mut no_regular_target_since = None;
    let mut was_playing = false;
    let mut was_heatmap_search_enabled = true;
    let mut last_priority_update = node.clock().now();
    let mut tick = node.create_timer(Duration::from_millis(50));

    loop {
        tick.tick().await;
        let now = node.clock().now();
        let elapsed_since_last_priority_update = now.duration_since(last_priority_update);
        last_priority_update = now;

        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        let cell_size = grid_cell_size(field_dimensions, parameters.cells_along_field_length);
        if cell_size != published_cell_size {
            topics.cell_size_pub.publish(&cell_size).await?;
            published_cell_size = cell_size;
        }
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
            topics.suggested_search_position_pub.publish(&None).await?;

            topics.drain_disabled_inputs().await?;
            continue;
        }
        if !was_heatmap_search_enabled {
            heatmap = Heatmap::new(field_dimensions, parameters.cells_along_field_length);
        }
        heatmap.resize(field_dimensions, parameters.cells_along_field_length);
        was_heatmap_search_enabled = true;
        let teammate_freshness = TeammateFreshness {
            now,
            window: topics
                .hsl_state_message_send_interval_cache
                .get_latest()
                .and_then(|interval| *interval)
                .map(|interval| interval.mul_f32(1.2)),
        };

        let ground_to_field = topics
            .ground_to_field_cache
            .get_latest()
            .map(|ground_to_field| *ground_to_field)
            .filter(|ground_to_field| ground_to_field_is_finite(*ground_to_field));
        let primary_state = topics.primary_state_cache.get_latest();
        let primary_state = primary_state.as_deref();
        let is_playing = primary_state == Some(&PrimaryState::Playing);
        let entered_playing = is_playing && !was_playing;
        if entered_playing {
            heatmap.limit_mean_priority(MAXIMUM_MEAN_PRIORITY_AT_PLAYING_START);
        }
        was_playing = is_playing;
        let mut ball_was_seen = false;

        while topics.ball_position_sub.is_ready() {
            ball_was_seen |= observe_local_ball(
                topics.ball_position_sub.recv().await?,
                ground_to_field,
                field_dimensions,
                &mut last_ball_position,
                &mut current_ball_position,
            );
        }
        while topics.hypothetical_ball_positions_sub.is_ready() {
            if let Some(ground_to_field) = ground_to_field {
                let hypothetical_ball_positions =
                    topics.hypothetical_ball_positions_sub.recv().await?;
                heatmap.update_with_hypothetical_ball_positions(
                    field_dimensions,
                    &hypothetical_ball_positions,
                    ground_to_field,
                    parameters,
                );
            } else {
                topics.hypothetical_ball_positions_sub.recv().await?;
            }
        }
        while topics.last_received_player_states_sub.is_ready() {
            let player_states = topics.last_received_player_states_sub.recv().await?;
            update_from_player_states(
                &mut heatmap,
                field_dimensions,
                &player_states,
                &mut last_teammate_states,
                parameters,
                teammate_freshness,
            );
        }
        while topics.filtered_game_controller_state_sub.is_ready() {
            let filtered_game_controller_state =
                topics.filtered_game_controller_state_sub.recv().await?;
            latest_filtered_game_controller_state = Some(filtered_game_controller_state);
        }

        let obstacles = topics.obstacles_cache.get_latest();
        let head_yaw = topics
            .head_joints_command_cache
            .get_latest()
            .map_or(0.0, |head_joints| head_joints.yaw);
        update_priorities(PriorityUpdate {
            heatmap: &mut heatmap,
            field_dimensions,
            parameters,
            latest_filtered_game_controller_state: &latest_filtered_game_controller_state,
            primary_state,
            last_ball_position,
            current_ball_position,
            ball_was_seen,
            is_playing,
            entered_playing,
            ground_to_field,
            obstacles: obstacles.as_deref().map(Vec::as_slice),
            head_yaw,
            now,
            elapsed_since_last_priority_update,
        });

        let suggested_search_position = select_search_position(TargetSelection {
            heatmap: &mut heatmap,
            voronoi_cache: &mut voronoi_cache,
            field_dimensions,
            parameters,
            ground_to_field,
            player_number: topics
                .player_number_cache
                .get_latest()
                .map(|number| *number),
            goalkeeper_player_number: topics
                .goalkeeper_player_number_cache
                .get_latest()
                .map(|number| *number),
            last_teammate_states: &last_teammate_states,
            teammate_freshness,
            last_ball_position,
            current_ball_position,
            is_playing,
            no_regular_target_since: &mut no_regular_target_since,
            now,
        });
        topics
            .suggested_search_position_pub
            .publish(&suggested_search_position)
            .await?;

        topics
            .additional_heatmap_pub
            .publish_if_subscribed(|| async { heatmap.to_message() })
            .await?;
    }
}

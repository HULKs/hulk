use color_eyre::{
    Result,
    eyre::{WrapErr as _, ensure},
};
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2, Vector2, point, vector};
use ndarray::{Array2, array};
use ndarray_conv::{ConvExt, ConvMode, PaddingMode};
use ros_z::{prelude::*, qos::QosDurability};
use std::{boxed::Box, f32::consts, future::Future, pin::Pin, sync::Arc, time::Duration};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
};
mod heatmap;
use heatmap::Heatmap;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("search_suggestor").build().await?;

    let parameters = node.bind_parameter_as::<SearchSuggestorParameters>("search_suggestor")?;
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
    let hypothetical_ball_positions_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>(
            "ball_filter/hypothetical_ball_positions",
        )
        .build()
        .await?;
    let ground_to_field_cache = node
        .subscriber::<Option<Isometry2<Ground, Field>>>("ground_to_field")
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
    let network_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")
        .build()
        .await?;
    let additional_heatmap_pub = node
        .publisher::<types::heatmap::Heatmap>("ball_search_heatmap")
        .build()
        .await?;
    let suggested_search_position_pub = node
        .publisher::<Point2<Field>>("suggested_search_position")
        .build()
        .await?;

    let field_dimensions = field_dimensions_sub.recv().await?;
    let initial_parameters_snapshot = parameters.snapshot();
    let initial_parameters = initial_parameters_snapshot.typed();
    let (heatmap_length, heatmap_width) = (
        (field_dimensions.length * initial_parameters.cells_per_meter).round() as usize,
        (field_dimensions.width * initial_parameters.cells_per_meter).round() as usize,
    );

    ensure!(
        heatmap_length > 0,
        "heatmap_length must at least be 1 - current value is {heatmap_length}"
    );
    ensure!(
        heatmap_width > 0,
        format!("heatmap_width must at least be 1 - current value is {heatmap_width}")
    );

    let mut heatmap = Heatmap {
        map: Array2::ones((heatmap_length, heatmap_width))
            / (heatmap_length * heatmap_width) as f32,
        cells_per_meter: initial_parameters.cells_per_meter,
        last_maximum_heatmap_position: None,
        has_decided_for_heatmap_tile: false,
    };

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let ground_to_field = ground_to_field_cache
            .get_latest()
            .and_then(|ground_to_field| *ground_to_field);
        let primary_state = primary_state_cache.get_latest();
        let primary_state = primary_state.as_deref();
        let mut ball_was_seen = false;

        while ball_position_sub.is_ready() {
            let ball_position = ball_position_sub.recv().await?;
            if let (Some(ball_position), Some(ground_to_field)) = (ball_position, ground_to_field) {
                ball_was_seen = true;
                heatmap.update_with_ball_position(field_dimensions, ball_position, ground_to_field);
            }
        }
        while hypothetical_ball_positions_sub.is_ready() {
            if let Some(ground_to_field) = ground_to_field {
                heatmap.update_with_hypothetical_ball_positions(
                    field_dimensions,
                    hypothetical_ball_positions_sub.recv().await?,
                    ground_to_field,
                    parameters,
                );
            } else {
                hypothetical_ball_positions_sub.recv().await?;
            }
        }
        while network_message_sub.is_ready() {
            heatmap.update_with_team_ball(
                field_dimensions,
                network_message_sub.recv().await?,
                parameters,
            );
        }
        while filtered_game_controller_state_sub.is_ready() {
            if let Some(primary_state) = primary_state {
                heatmap.update_with_rule_ball(
                    &filtered_game_controller_state_sub.recv().await?,
                    &field_dimensions,
                    primary_state,
                    parameters,
                );
            } else {
                filtered_game_controller_state_sub.recv().await?;
            }
        }

        if !ball_was_seen && let Some(ground_to_field) = ground_to_field {
            let robot_position = ground_to_field.as_pose().position().coords();
            let body_orientation = ground_to_field.orientation().angle();
            let fov_angle_offset = 45.0 * consts::PI / 180.0;
            let left_angle = body_orientation - fov_angle_offset;
            let right_angle = body_orientation + fov_angle_offset;
            let left_edge: Vector2<Field> = vector!(left_angle.cos(), left_angle.sin());
            let right_edge: Vector2<Field> = vector!(right_angle.cos(), right_angle.sin());

            heatmap.decay_tiles_in_fov(
                field_dimensions,
                robot_position,
                left_edge,
                right_edge,
                parameters.decay_distance_factor,
                parameters.heatmap_decay_range.clone(),
            );
        }

        let kernel = create_kernel(parameters.heatmap_convolution_kernel_weight);
        heatmap.map = heatmap
            .map
            .conv(&kernel, ConvMode::Same, PaddingMode::Replicate)
            .wrap_err("heatmap convolution failed")?;
        heatmap.map /= heatmap.map.sum();

        if !heatmap.has_decided_for_heatmap_tile {
            let suggested_search_index = heatmap.get_maximum_position(parameters.minimum_validity);
            if suggested_search_index.is_some() {
                heatmap.has_decided_for_heatmap_tile = true;
            }
            heatmap.last_maximum_heatmap_position = suggested_search_index;
        } else if let Some(last_maximum_heatmap_index) = heatmap.last_maximum_heatmap_position {
            let global_max_value = heatmap
                .get_maximum_position(0.0)
                .map_or(0.0, |idx| heatmap.map[idx]);
            let current_tile_value = heatmap.map[last_maximum_heatmap_index];

            if current_tile_value < global_max_value * parameters.tile_switch_hysteresis {
                heatmap.has_decided_for_heatmap_tile = false;
            }
        }

        if let Some((x, y)) = heatmap.last_maximum_heatmap_position {
            let suggested_search_position = point![
                ((x as f32 + 1.0 / 2.0) / heatmap.cells_per_meter - field_dimensions.length / 2.0),
                ((y as f32 + 1.0 / 2.0) / heatmap.cells_per_meter - field_dimensions.width / 2.0)
            ];
            suggested_search_position_pub
                .publish(&suggested_search_position)
                .await?;
        }

        additional_heatmap_pub
            .publish_if_subscribed(|| async { heatmap.to_message() })
            .await?;

        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

fn create_kernel(alpha: f32) -> Array2<f32> {
    array![
        [alpha, alpha, alpha],
        [alpha, 1.0 - alpha, alpha],
        [alpha, alpha, alpha]
    ] / (1.0 + 7.0 * alpha)
}

use std::{
    boxed::Box,
    f32::consts,
    future::Future,
    ops::Range,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::Result;

use coordinate_systems::{Field, Ground};
use geometry::direction::{Direction, Rotate90Degrees};
use hsl_network_messages::{HulkMessage, StateMessage, SubState, Team};
use itertools::Itertools;
use linear_algebra::{Isometry2, Point2, Vector2, point, vector};
use nalgebra::clamp;
use ndarray::{Array2, array};
use ndarray_conv::{ConvExt, ConvMode, PaddingMode};
use ros_z::{prelude::*, qos::QosDurability};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::{FieldDimensions, Half, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("search_suggestor").build().await?;

    let parameters = node.bind_parameter_as::<SearchSuggestorParameters>("search_suggestor")?;
    let field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _ball_position_sub = node
        .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")?
        .build()
        .await?;
    let hypothetical_ball_positions_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")?
        .build()
        .await?;
    let ground_to_field_cache = node
        .create_cache::<Isometry2<Ground, Field>>("ground_to_field", 10)?
        .build()
        .await?;
    let primary_state_cache = node
        .create_cache::<PrimaryState>("primary_state", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<TimeWrapper<FilteredGameControllerState>>("filtered_game_controller_state")?
        .build()
        .await?;
    let network_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")?
        .build()
        .await?;
    // TODO: do we need to directly publish ndarray here? Choose another type or manually implement
    // support for `Array: Message` in ros-z
    // let _heatmap_pub = node
    //     .publisher::<Array2<f32>>("ball_search_heatmap")
    //     .build()
    //     .await?;
    let suggested_search_position_pub = node
        .publisher::<Point2<Field>>("suggested_search_position")?
        .build()
        .await?;

    let field_dimensions = field_dimensions_sub.recv().await?;
    let initial_parameters_snapshot = parameters.snapshot();
    let initial_parameters = initial_parameters_snapshot.typed();
    let (heatmap_length, heatmap_width) = (
        (field_dimensions.length * initial_parameters.cells_per_meter).round() as usize,
        (field_dimensions.width * initial_parameters.cells_per_meter).round() as usize,
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
            .map(|ground_to_field| *ground_to_field);
        let primary_state = primary_state_cache.get_latest();
        let primary_state = primary_state.as_deref();
        let mut ball_was_seen = false;

        while ball_position_sub.is_ready() {
            ball_was_seen = true;
            if let Some(ground_to_field) = ground_to_field {
                heatmap.update_with_ball_position(
                    field_dimensions,
                    ball_position_sub.recv().await?,
                    ground_to_field,
                );
            } else {
                ball_position_sub.recv().await?;
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
                    &filtered_game_controller_state_sub.recv().await?.inner,
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

#[derive(Deserialize, Serialize)]
struct Heatmap {
    map: Array2<f32>,
    cells_per_meter: f32,
    last_maximum_heatmap_position: Option<(usize, usize)>,
    has_decided_for_heatmap_tile: bool,
}

impl Heatmap {
    fn update_with_ball_position(
        &mut self,
        field_dimensions: FieldDimensions,
        ball_position: BallPosition<Ground>,
        ground_to_field: Isometry2<Ground, Field>,
    ) {
        let heatmap_point =
            self.field_to_heatmap(field_dimensions, ground_to_field * ball_position.position);
        self.map[heatmap_point] = 1.0;
    }

    fn update_with_hypothetical_ball_positions(
        &mut self,
        field_dimensions: FieldDimensions,
        hypothetical_ball_positions: Vec<HypotheticalBallPosition<Ground>>,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: &SearchSuggestorParameters,
    ) {
        for ball_hypothesis in hypothetical_ball_positions {
            let ball_hypothesis_position = ground_to_field * ball_hypothesis.position;
            let heatmap_point = self.field_to_heatmap(field_dimensions, ball_hypothesis_position);
            self.map[heatmap_point] = (self.map[heatmap_point]
                + ball_hypothesis.validity * parameters.own_ball_weight)
                / 2.0;
        }
    }

    fn update_with_rule_ball(
        &mut self,
        filtered_game_controller_state: &FilteredGameControllerState,
        field_dimensions: &FieldDimensions,
        primary_state: &PrimaryState,
        parameters: &SearchSuggestorParameters,
    ) {
        for rule_ball_hypothesis in get_rule_hypotheses(
            *primary_state,
            filtered_game_controller_state,
            *field_dimensions,
        ) {
            let heatmap_point = self.field_to_heatmap(*field_dimensions, rule_ball_hypothesis);
            self.map[heatmap_point] += parameters.rule_ball_weight_increment;
        }
    }

    fn update_with_team_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        network_message: TimeWrapper<IncomingMessage>,
        parameters: &SearchSuggestorParameters,
    ) {
        let IncomingMessage::Hsl(message) = network_message.inner else {
            return;
        };
        self.add_teamballs(
            field_dimensions,
            network_message.time.to_wallclock(),
            message,
            parameters.team_ball_weight,
        );
    }

    fn field_to_heatmap(
        &self,
        field_dimensions: FieldDimensions,
        field_point: Point2<Field>,
    ) -> (usize, usize) {
        let heatmap_point = (
            ((field_point.x() + field_dimensions.length / 2.0) * self.cells_per_meter) as usize,
            ((field_point.y() + field_dimensions.width / 2.0) * self.cells_per_meter) as usize,
        );
        (
            clamp(heatmap_point.0, 0, self.map.dim().0 - 1),
            clamp(heatmap_point.1, 0, self.map.dim().1 - 1),
        )
    }

    fn get_maximum_position(&self, minimum_validity: f32) -> Option<(usize, usize)> {
        let linear_maximum_heat_heatmap_position =
            self.map.iter().position_max_by(|a, b| a.total_cmp(b))?;
        let maximum_heat_heatmap_position = (
            linear_maximum_heat_heatmap_position / self.map.dim().1,
            linear_maximum_heat_heatmap_position % self.map.dim().1,
        );
        if self.map[maximum_heat_heatmap_position] > minimum_validity {
            return Some(maximum_heat_heatmap_position);
        }
        None
    }

    fn add_teamballs(
        &mut self,
        field_dimensions: FieldDimensions,
        time: SystemTime,
        message: HulkMessage,
        team_ball_weight: f32,
    ) {
        let ball = match message {
            HulkMessage::State(StateMessage { ball_position, .. }) => {
                ball_position.map(|ball| BallPosition {
                    position: ball.position,
                    velocity: Vector2::zeros(),
                    last_seen: time - ball.age,
                })
            }
        };
        if let Some(ball_position) = ball {
            let heatmap_point = self.field_to_heatmap(field_dimensions, ball_position.position);
            self.map[heatmap_point] = team_ball_weight;
        }
    }

    fn decay_tiles_in_fov(
        &mut self,
        field_dimensions: FieldDimensions,
        robot_position: Vector2<Field>,
        left_edge: Vector2<Field>,
        right_edge: Vector2<Field>,
        decay_distance_factor: f32,
        heatmap_decay_range: Range<f32>,
    ) {
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field: Vector2<Field> = vector![
                ((x as f32 + 1.0 / 2.0) / self.cells_per_meter - field_dimensions.length / 2.0),
                ((y as f32 + 1.0 / 2.0) / self.cells_per_meter - field_dimensions.width / 2.0)
            ];
            let robot_to_tile = tile_center_in_field - robot_position;
            let is_inside_sight = get_direction(left_edge, robot_to_tile)
                == Direction::Counterclockwise
                && get_direction(right_edge, robot_to_tile) == Direction::Clockwise;
            let distance_to_tile = robot_to_tile.norm();
            let relative_distance_to_tile =
                clamp(distance_to_tile / heatmap_decay_range.end, 0.0, 1.0);
            if is_inside_sight && heatmap_decay_range.contains(&distance_to_tile) {
                *value *= 1.0 - decay_distance_factor * (1.0 - relative_distance_to_tile);
            }
        });
    }
}

fn get_rule_hypotheses(
    primary_state: PrimaryState,
    filtered_game_controller_state: &FilteredGameControllerState,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    let kicking_team_half = kicking_team_half(filtered_game_controller_state.kicking_team);

    match (primary_state, filtered_game_controller_state.sub_state) {
        (PrimaryState::Ready, Some(SubState::PenaltyKick)) => {
            let kicking_team_half = kicking_team_half.unwrap_or(Half::Own).mirror();
            vec![field_dimensions.penalty_spot(kicking_team_half)]
        }
        // Kick-off
        (PrimaryState::Ready, None) => vec![field_dimensions.center()],
        (PrimaryState::Playing, Some(SubState::CornerKick)) => {
            if let Some(kicking_team_half) = kicking_team_half {
                let kicking_team_half = kicking_team_half.mirror();
                vec![
                    field_dimensions.corner(kicking_team_half, Side::Left),
                    field_dimensions.corner(kicking_team_half, Side::Right),
                ]
            } else {
                vec![
                    field_dimensions.corner(Half::Own, Side::Left),
                    field_dimensions.corner(Half::Opponent, Side::Left),
                    field_dimensions.corner(Half::Own, Side::Right),
                    field_dimensions.corner(Half::Opponent, Side::Right),
                ]
            }
        }
        (PrimaryState::Playing, Some(SubState::GoalKick)) => {
            if let Some(kicking_team_half) = kicking_team_half {
                vec![
                    field_dimensions.goal_box_corner(kicking_team_half, Side::Left),
                    field_dimensions.goal_box_corner(kicking_team_half, Side::Right),
                ]
            } else {
                vec![
                    field_dimensions.goal_box_corner(Half::Own, Side::Left),
                    field_dimensions.goal_box_corner(Half::Opponent, Side::Left),
                    field_dimensions.goal_box_corner(Half::Own, Side::Right),
                    field_dimensions.goal_box_corner(Half::Opponent, Side::Right),
                ]
            }
        }
        (_, _) => Vec::new(),
    }
}

fn kicking_team_half(kicking_team: Option<Team>) -> Option<Half> {
    match kicking_team {
        Some(Team::Opponent) => Some(Half::Opponent),
        Some(Team::Hulks) => Some(Half::Own),
        None => None,
    }
}

fn get_direction(base_vector: Vector2<Field>, vector_to_test: Vector2<Field>) -> Direction {
    let clockwise_normal_vector = base_vector.rotate_90_degrees(Direction::Clockwise);
    let directed_cathetus = clockwise_normal_vector.dot(&vector_to_test);

    match directed_cathetus {
        0.0 => Direction::Collinear,
        f if f > 0.0 => Direction::Clockwise,
        f if f < 0.0 => Direction::Counterclockwise,
        f => panic!("directed cathetus was not a real number: {f}"),
    }
}

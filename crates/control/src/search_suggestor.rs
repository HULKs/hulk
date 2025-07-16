use std::{
    f32::consts,
    ops::{Index, IndexMut},
    time::SystemTime,
};

use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use geometry::direction::{Direction, Rotate90Degrees};
use itertools::Itertools;
use linear_algebra::{point, vector, Isometry2, Point2, Vector2};
use nalgebra::clamp;
use ndarray::{array, Array2};
use ndarray_conv::{ConvExt, ConvMode, PaddingMode};
use serde::{Deserialize, Serialize};
use spl_network_messages::{HulkMessage, SubState, Team};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::{FieldDimensions, Half, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
    sensor_data::SensorData,
};

use crate::team_ball_receiver::get_spl_messages;

#[derive(Deserialize, Serialize)]
pub struct SearchSuggestor {
    heatmap: Heatmap,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
}

#[context]
pub struct CycleContext {
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    hypothetical_ball_positions:
        Input<Vec<HypotheticalBallPosition<Ground>>, "hypothetical_ball_positions">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    sensor_data: Input<SensorData, "sensor_data">,
    primary_state: Input<PrimaryState, "primary_state">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,

    heatmap: AdditionalOutput<Array2<f32>, "ball_search_heatmap">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub suggested_search_position: MainOutput<Option<Point2<Field>>>,
}

impl SearchSuggestor {
    pub fn new(context: CreationContext) -> Result<Self> {
        let (heatmap_length, heatmap_width) = (
            (context.field_dimensions.length
                * context.search_suggestor_configuration.cells_per_meter)
                .round() as usize,
            (context.field_dimensions.width
                * context.search_suggestor_configuration.cells_per_meter)
                .round() as usize,
        );
        let heatmap = Heatmap {
            map: Array2::ones((heatmap_length, heatmap_width))
                / (heatmap_length * heatmap_width) as f32,
            field_dimensions: *context.field_dimensions,
            cells_per_meter: context.search_suggestor_configuration.cells_per_meter,
        };
        Ok(Self { heatmap })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.update_heatmap(&context)?;
        let suggested_search_position = self
            .heatmap
            .get_maximum_position(context.search_suggestor_configuration.minimum_validity);

        context
            .heatmap
            .fill_if_subscribed(|| self.heatmap.map.clone());

        Ok(MainOutputs {
            suggested_search_position: suggested_search_position.into(),
        })
    }

    fn update_heatmap(&mut self, context: &CycleContext) -> Result<()> {
        if let Some(ball_position) = context.ball_position {
            if let Some(ground_to_field) = context.ground_to_field {
                self.heatmap[ground_to_field * ball_position.position] = 1.0;
            }
        }
        for ball_hypothesis in context.hypothetical_ball_positions {
            if let Some(ground_to_field) = context.ground_to_field {
                let ball_hypothesis_position = ground_to_field * ball_hypothesis.position;
                self.heatmap[ball_hypothesis_position] = (self.heatmap[ball_hypothesis_position]
                    + ball_hypothesis.validity
                        * context.search_suggestor_configuration.own_ball_weight)
                    / 2.0;
            }
        }
        if let Some(filtered_game_controller_state) = context.filtered_game_controller_state {
            for rule_ball_hypothesis in get_rule_hypotheses(
                *context.primary_state,
                filtered_game_controller_state,
                *context.field_dimensions,
            ) {
                self.heatmap[rule_ball_hypothesis] =
                    context.search_suggestor_configuration.rule_ball_weight;
            }
        }

        let messages = get_spl_messages(&context.network_message.persistent);
        for (time, message) in messages {
            self.heatmap.add_teamballs(
                time,
                message,
                context.search_suggestor_configuration.team_ball_weight,
            );
        }

        if context.ball_position.is_none() {
            if let Some(ground_to_field) = context.ground_to_field {
                let robot_position = ground_to_field.as_pose().position().coords();
                let head_orientation =
                    ground_to_field.orientation().angle() + context.sensor_data.positions.head.yaw;
                let fov_angle_offset = 25.0 * consts::PI / 180.0;
                let left_angle = head_orientation - fov_angle_offset;
                let right_angle = head_orientation + fov_angle_offset;
                let left_edge: Vector2<Field> = vector!(left_angle.cos(), left_angle.sin());
                let right_edge: Vector2<Field> = vector!(right_angle.cos(), right_angle.sin());

                let tile_width = 1.0 / self.heatmap.cells_per_meter;
                let tile_center_offset = tile_width / 2.0;
                let bottom_left_corner_in_field: Vector2<Field> = vector!(
                    -self.heatmap.field_dimensions.length / 2.0,
                    -self.heatmap.field_dimensions.width / 2.0
                );
                self.heatmap
                    .map
                    .indexed_iter_mut()
                    .for_each(|((x, y), value)| {
                        let tile_center_in_field: Vector2<Field> = vector!(
                            (x as f32) * tile_width + tile_center_offset,
                            (y as f32) * tile_width + tile_center_offset,
                        ) + bottom_left_corner_in_field;
                        let robot_to_tile = tile_center_in_field - robot_position;
                        let is_inside_sight = get_direction(left_edge, robot_to_tile)
                            == Direction::Counterclockwise
                            && get_direction(right_edge, robot_to_tile) == Direction::Clockwise;
                        let distancse_to_tile = robot_to_tile.norm();
                        let relative_distance_to_tile = clamp(
                            distancse_to_tile / self.heatmap.field_dimensions.length,
                            0.0,
                            1.0,
                        );
                        if is_inside_sight && distancse_to_tile > 0.25 {
                            *value *= 1.0 - 0.05 * relative_distance_to_tile;
                        }
                    });
            }
        }

        let kernel = create_kernel(
            context
                .search_suggestor_configuration
                .heatmap_convolution_kernel_weight,
        );
        self.heatmap.map = self
            .heatmap
            .map
            .conv(&kernel, ConvMode::Same, PaddingMode::Replicate)
            .wrap_err("heatmap convolution failed")?;
        self.heatmap.map /= self.heatmap.map.sum();
        Ok(())
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
    field_dimensions: FieldDimensions,
    cells_per_meter: f32,
}

impl Heatmap {
    fn field_to_heatmap(&self, field_point: Point2<Field>) -> (usize, usize) {
        let heatmap_point = (
            ((field_point.x() + self.field_dimensions.length / 2.0) * self.cells_per_meter)
                as usize,
            ((field_point.y() + self.field_dimensions.width / 2.0) * self.cells_per_meter) as usize,
        );
        (
            clamp(heatmap_point.0, 0, self.map.dim().0 - 1),
            clamp(heatmap_point.1, 0, self.map.dim().1 - 1),
        )
    }

    fn get_maximum_position(&self, minimum_validity: f32) -> Option<Point2<Field>> {
        let linear_maximum_heat_heatmap_position =
            self.map.iter().position_max_by(|a, b| a.total_cmp(b))?;
        let maximum_heat_heatmap_position = (
            linear_maximum_heat_heatmap_position / self.map.dim().1,
            linear_maximum_heat_heatmap_position % self.map.dim().1,
        );
        if self.map[maximum_heat_heatmap_position] > minimum_validity {
            let search_suggestion = point![
                ((maximum_heat_heatmap_position.0 as f32 + 1.0 / 2.0) / self.cells_per_meter
                    - self.field_dimensions.length / 2.0),
                ((maximum_heat_heatmap_position.1 as f32 + 1.0 / 2.0) / self.cells_per_meter
                    - self.field_dimensions.width / 2.0)
            ];
            return Some(search_suggestion);
        }
        None
    }

    fn add_teamballs(&mut self, time: SystemTime, message: HulkMessage, team_ball_weight: f32) {
        let (_, ball) = match message {
            HulkMessage::Striker(striker_message) => (
                striker_message.player_number,
                Some(BallPosition {
                    position: striker_message.ball_position.position,
                    velocity: Vector2::zeros(),
                    last_seen: time - striker_message.ball_position.age,
                }),
            ),
            HulkMessage::Loser(_) | HulkMessage::VisualReferee(_) => return,
        };
        if let Some(ball_position) = ball {
            self[ball_position.position] = team_ball_weight;
        }
    }
}

impl Index<Point2<Field>> for Heatmap {
    type Output = f32;
    fn index(&self, field_point: Point2<Field>) -> &Self::Output {
        let heatmap_point = self.field_to_heatmap(field_point);
        &self.map[heatmap_point]
    }
}

impl IndexMut<Point2<Field>> for Heatmap {
    fn index_mut(&mut self, field_point: Point2<Field>) -> &mut Self::Output {
        let heatmap_point = self.field_to_heatmap(field_point);
        &mut self.map[heatmap_point]
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

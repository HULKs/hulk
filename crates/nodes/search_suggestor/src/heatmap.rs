use std::{ops::Range, time::SystemTime};

use coordinate_systems::{Field, Ground};
use geometry::direction::{Direction, Rotate90Degrees};
use hsl_network_messages::{HulkMessage, StateMessage, SubState, Team};
use itertools::Itertools;
use linear_algebra::{Isometry2, Point2, Vector2, vector};
use nalgebra::clamp;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::{FieldDimensions, Half, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    heatmap::Heatmap as HeatmapMessage,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
};

#[derive(Deserialize, Serialize)]
pub(crate) struct Heatmap {
    pub(crate) map: Array2<f32>,
    pub(crate) cells_per_meter: f32,
    pub(crate) last_maximum_heatmap_position: Option<(usize, usize)>,
    pub(crate) has_decided_for_heatmap_tile: bool,
}

impl Heatmap {
    pub(crate) fn to_message(&self) -> HeatmapMessage {
        let (length, width) = self.map.dim();
        HeatmapMessage {
            length: length as u32,
            width: width as u32,
            values: self.map.iter().copied().collect(),
        }
    }

    pub(crate) fn update_with_ball_position(
        &mut self,
        field_dimensions: FieldDimensions,
        ball_position: BallPosition<Ground>,
        ground_to_field: Isometry2<Ground, Field>,
    ) {
        let heatmap_point =
            self.field_to_heatmap(field_dimensions, ground_to_field * ball_position.position);
        self.map[heatmap_point] = 1.0;
    }

    pub(crate) fn update_with_hypothetical_ball_positions(
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

    pub(crate) fn update_with_rule_ball(
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

    pub(crate) fn update_with_team_ball(
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

    pub(crate) fn get_maximum_position(&self, minimum_validity: f32) -> Option<(usize, usize)> {
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

    pub(crate) fn decay_tiles_in_fov(
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

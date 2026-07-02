use std::{f32::consts::LN_2, ops::Range, time::Duration, time::SystemTime};

use coordinate_systems::{Field, Ground};
use geometry::direction::{Direction, Rotate90Degrees};
use hsl_network_messages::{HulkMessage, PlayerNumber, StateMessage, SubState, Team};
use itertools::Itertools;
use linear_algebra::{Isometry2, Point2, Pose2, Vector2, point, vector};
use nalgebra::clamp;
use ndarray::Array2;
use ros_z::time::Time;
use serde::{Deserialize, Serialize};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::{FieldDimensions, Half, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    heatmap::Heatmap as HeatmapMessage,
    messages::IncomingMessage,
    obstacles::Obstacle,
    parameters::{SearchSuggestorParameters, VoronoiParameters},
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    time_wrapper::TimeWrapper,
};
use voronoi::{Ownership, VoronoiBounds, VoronoiGrid};

#[derive(Clone, Copy, Debug)]
pub struct SearchOccluder {
    pub center: Point2<Field>,
    pub radius: f32,
}

struct FieldOfViewDecay<'a> {
    distance_factor: f32,
    range: Range<f32>,
    sampled_tick_count: usize,
    occluders: &'a [SearchOccluder],
    occluded_factor: f32,
}

#[derive(Clone, Debug)]
pub struct SearchVoronoiSelection {
    pub owner: PlayerNumber,
    pub grid: VoronoiGrid,
}

impl SearchVoronoiSelection {
    pub fn new_with_obstacles(
        field_dimensions: FieldDimensions,
        owner: PlayerNumber,
        sites: impl IntoIterator<Item = (Pose2<Field>, PlayerNumber)>,
        obstacles: &[Obstacle],
        rule_obstacles: &[RuleObstacle],
        ground_to_field: Isometry2<Ground, Field>,
    ) -> Self {
        let mut grid = search_voronoi_grid(field_dimensions);
        grid.initialize_obstacles(obstacles, rule_obstacles, ground_to_field);
        grid.multi_source_dijkstra(&sites.into_iter().collect_vec(), 0.0);
        Self { owner, grid }
    }

    fn owns_index(&self, field_dimensions: FieldDimensions, index: (usize, usize)) -> bool {
        self.grid
            .ownership_at(heatmap_tile_center(field_dimensions, index))
            .is_some_and(|ownership| ownership == Ownership::Robot(self.owner))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Heatmap {
    map: Array2<f32>,
    last_maximum_heatmap_position: Option<(usize, usize)>,
    has_decided_for_heatmap_tile: bool,
}

impl Heatmap {
    pub fn new(field_dimensions: FieldDimensions) -> Self {
        let (length, width) = heatmap_dimensions(field_dimensions);
        Self {
            map: Array2::zeros((length, width)),
            last_maximum_heatmap_position: None,
            has_decided_for_heatmap_tile: false,
        }
    }

    pub fn to_message(&self) -> HeatmapMessage {
        let (length, width) = self.map.dim();
        HeatmapMessage {
            length: length as u32,
            width: width as u32,
            values: self.map.iter().copied().collect(),
        }
    }

    pub fn selected_position(&self, field_dimensions: FieldDimensions) -> Option<Point2<Field>> {
        self.last_maximum_heatmap_position
            .map(|index| heatmap_tile_center(field_dimensions, index))
    }

    pub fn set_known_ball_position(
        &mut self,
        field_dimensions: FieldDimensions,
        ball_position: Point2<Field>,
    ) {
        let heatmap_point = self.field_to_heatmap(field_dimensions, ball_position);
        let (length, width) = self.map.dim();
        self.map.fill(0.0);
        for x in
            heatmap_point.0.saturating_sub(1)..=heatmap_point.0.saturating_add(1).min(length - 1)
        {
            for y in
                heatmap_point.1.saturating_sub(1)..=heatmap_point.1.saturating_add(1).min(width - 1)
            {
                if (x, y) != heatmap_point {
                    self.map[(x, y)] = 0.5;
                }
            }
        }
        self.map[heatmap_point] = 1.0;
        self.last_maximum_heatmap_position = Some(heatmap_point);
        self.has_decided_for_heatmap_tile = true;
    }

    pub fn update_with_hypothetical_ball_positions<'a>(
        &mut self,
        field_dimensions: FieldDimensions,
        hypothetical_ball_positions: impl IntoIterator<Item = &'a HypotheticalBallPosition<Ground>>,
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

    pub fn update_with_rule_ball(
        &mut self,
        filtered_game_controller_state: &FilteredGameControllerState,
        field_dimensions: FieldDimensions,
        primary_state: PrimaryState,
        parameters: &SearchSuggestorParameters,
    ) {
        if self.regenerate_restart_hypotheses(
            filtered_game_controller_state,
            field_dimensions,
            parameters.rule_ball_weight_increment,
        ) {
            return;
        }

        for rule_ball_hypothesis in get_rule_hypotheses(
            primary_state,
            filtered_game_controller_state,
            field_dimensions,
        ) {
            let heatmap_point = self.field_to_heatmap(field_dimensions, rule_ball_hypothesis);
            self.map[heatmap_point] += parameters.rule_ball_weight_increment;
        }
    }

    pub fn update_with_team_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        network_message: TimeWrapper<IncomingMessage>,
        parameters: &SearchSuggestorParameters,
    ) {
        let IncomingMessage::Hsl(message) = network_message.inner else {
            return;
        };
        self.add_team_ball(
            field_dimensions,
            network_message.time.to_wallclock(),
            message,
            parameters.team_ball_weight,
        );
    }

    fn add_team_ball(
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
                    last_seen: Time::from_wallclock(time) - ball.age,
                })
            }
        };
        if let Some(ball_position) = ball {
            let heatmap_point = self.field_to_heatmap(field_dimensions, ball_position.position);
            self.map[heatmap_point] = team_ball_weight;
        }
    }

    pub fn clamp_values(&mut self) {
        self.map.mapv_inplace(|value| {
            if value.is_nan() {
                0.0
            } else {
                value.clamp(0.0, 1.0)
            }
        });
    }

    pub fn increase_around_last_ball(
        &mut self,
        field_dimensions: FieldDimensions,
        last_ball_position: Point2<Field>,
        elapsed: Duration,
        parameters: &SearchSuggestorParameters,
    ) {
        let rise_time = parameters.last_ball_priority_rise_time.as_secs_f32();
        let rise_fraction = if rise_time <= f32::EPSILON {
            1.0
        } else {
            elapsed.as_secs_f32() / rise_time
        };
        if rise_fraction <= 0.0 {
            return;
        }

        let half_distance = parameters
            .last_ball_priority_half_distance
            .max(f32::EPSILON);
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field = heatmap_tile_center(field_dimensions, (x, y));
            let distance_to_last_ball = (tile_center_in_field - last_ball_position).norm();
            let gaussian = (-LN_2 * (distance_to_last_ball / half_distance).powi(2)).exp();
            let multiplier = parameters.last_ball_priority_minimum
                + (parameters.last_ball_priority_maximum - parameters.last_ball_priority_minimum)
                    * gaussian;
            *value += multiplier * rise_fraction;
        });
    }

    fn get_maximum_position_matching(
        &self,
        minimum_validity: f32,
        matches_selection: impl Fn((usize, usize)) -> bool,
    ) -> Option<(usize, usize)> {
        self.map
            .indexed_iter()
            .filter(|(index, value)| **value > minimum_validity && matches_selection(*index))
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
    }

    fn get_maximum_position_with_turn_preference_matching(
        &self,
        minimum_validity: f32,
        turn_preference_priority_margin: f32,
        field_dimensions: FieldDimensions,
        robot_position: Vector2<Field>,
        robot_heading: Vector2<Field>,
        matches_selection: impl Fn((usize, usize)) -> bool,
    ) -> Option<(usize, usize)> {
        let maximum_position =
            self.get_maximum_position_matching(minimum_validity, &matches_selection)?;
        let maximum_value = self.map[maximum_position];
        self.map
            .indexed_iter()
            .filter(|(index, value)| {
                matches_selection(*index)
                    && **value > minimum_validity
                    && **value >= maximum_value - turn_preference_priority_margin.max(0.0)
            })
            .max_by(|(a_index, a_value), (b_index, b_value)| {
                let a_turn_score =
                    turn_score(*a_index, field_dimensions, robot_position, robot_heading);
                let b_turn_score =
                    turn_score(*b_index, field_dimensions, robot_position, robot_heading);
                a_turn_score
                    .total_cmp(&b_turn_score)
                    .then_with(|| a_value.total_cmp(b_value))
            })
            .map(|(index, _)| index)
    }

    fn get_suggested_search_index(
        &self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        voronoi_selection: Option<&SearchVoronoiSelection>,
    ) -> Option<(usize, usize)> {
        let matches_selection = |index| {
            voronoi_selection
                .map(|selection| selection.owns_index(field_dimensions, index))
                .unwrap_or(true)
        };
        ground_to_field.map_or_else(
            || self.get_maximum_position_matching(parameters.minimum_validity, matches_selection),
            |ground_to_field| {
                let robot_position = ground_to_field.as_pose().position().coords();
                let robot_heading_angle = ground_to_field.orientation().angle();
                let robot_heading = vector![robot_heading_angle.cos(), robot_heading_angle.sin()];
                self.get_maximum_position_with_turn_preference_matching(
                    parameters.minimum_validity,
                    parameters.turn_preference_priority_margin,
                    field_dimensions,
                    robot_position,
                    robot_heading,
                    matches_selection,
                )
            },
        )
    }

    pub fn update_suggested_search_position_with_voronoi(
        &mut self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        voronoi_selection: &SearchVoronoiSelection,
    ) {
        self.update_suggested_search_position_with_optional_voronoi(
            field_dimensions,
            parameters,
            ground_to_field,
            Some(voronoi_selection),
        );
    }

    pub fn clear_suggested_search_position(&mut self) {
        self.last_maximum_heatmap_position = None;
        self.has_decided_for_heatmap_tile = false;
    }

    fn update_suggested_search_position_with_optional_voronoi(
        &mut self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        voronoi_selection: Option<&SearchVoronoiSelection>,
    ) {
        if !self.has_decided_for_heatmap_tile {
            let suggested_search_index = self.get_suggested_search_index(
                field_dimensions,
                parameters,
                ground_to_field,
                voronoi_selection,
            );
            if suggested_search_index.is_some() {
                self.has_decided_for_heatmap_tile = true;
            }
            self.last_maximum_heatmap_position = suggested_search_index;
        } else if let Some(last_maximum_heatmap_index) = self.last_maximum_heatmap_position {
            let current_tile_is_selectable = voronoi_selection
                .map(|selection| selection.owns_index(field_dimensions, last_maximum_heatmap_index))
                .unwrap_or(true);
            let global_max_value = self
                .get_maximum_position_matching(0.0, |index| {
                    voronoi_selection
                        .map(|selection| selection.owns_index(field_dimensions, index))
                        .unwrap_or(true)
                })
                .map_or(0.0, |idx| self.map[idx]);
            let current_tile_value = self.map[last_maximum_heatmap_index];

            if !current_tile_is_selectable
                || current_tile_value <= parameters.minimum_validity
                || current_tile_value < global_max_value * parameters.tile_switch_hysteresis
            {
                let suggested_search_index = self.get_suggested_search_index(
                    field_dimensions,
                    parameters,
                    ground_to_field,
                    voronoi_selection,
                );
                self.has_decided_for_heatmap_tile = suggested_search_index.is_some();
                self.last_maximum_heatmap_position = suggested_search_index;
            }
        }
    }

    fn decay_tiles_in_fov_for_sampled_ticks(
        &mut self,
        field_dimensions: FieldDimensions,
        robot_position: Vector2<Field>,
        left_edge: Vector2<Field>,
        right_edge: Vector2<Field>,
        decay: FieldOfViewDecay<'_>,
    ) {
        let sampled_tick_count = decay.sampled_tick_count.max(1).min(i32::MAX as usize) as i32;
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field: Vector2<Field> = vector![
                (x as f32 + 0.5 - field_dimensions.length / 2.0),
                (y as f32 + 0.5 - field_dimensions.width / 2.0)
            ];
            let robot_to_tile = tile_center_in_field - robot_position;
            let is_inside_sight = get_direction(left_edge, robot_to_tile)
                == Direction::Counterclockwise
                && get_direction(right_edge, robot_to_tile) == Direction::Clockwise;
            let distance_to_tile = robot_to_tile.norm();
            let relative_distance_to_tile = clamp(distance_to_tile / decay.range.end, 0.0, 1.0);
            if is_inside_sight && decay.range.contains(&distance_to_tile) {
                let occlusion_factor =
                    if is_occluded(robot_position, tile_center_in_field, decay.occluders) {
                        decay.occluded_factor.clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                let per_tick_decay =
                    decay.distance_factor * occlusion_factor * (1.0 - relative_distance_to_tile);
                let effective_decay = 1.0 - (1.0 - per_tick_decay).powi(sampled_tick_count);
                *value *= 1.0 - effective_decay;
            }
        });
    }

    fn decay_tiles_from_field_pose_and_heading_for_sampled_ticks(
        &mut self,
        field_dimensions: FieldDimensions,
        pose: Pose2<Field>,
        heading_direction: Vector2<Field>,
        decay: FieldOfViewDecay<'_>,
    ) {
        if heading_direction.norm() <= f32::EPSILON {
            return;
        }

        let robot_position = pose.position().coords();
        let heading_angle = heading_direction.y().atan2(heading_direction.x());
        let fov_angle_offset = 45.0_f32.to_radians();
        let left_angle = heading_angle - fov_angle_offset;
        let right_angle = heading_angle + fov_angle_offset;
        let left_edge: Vector2<Field> = vector![left_angle.cos(), left_angle.sin()];
        let right_edge: Vector2<Field> = vector![right_angle.cos(), right_angle.sin()];

        self.decay_tiles_in_fov_for_sampled_ticks(
            field_dimensions,
            robot_position,
            left_edge,
            right_edge,
            decay,
        );
    }

    pub fn decay_tiles_from_teammate_motion(
        &mut self,
        field_dimensions: FieldDimensions,
        previous: &StateMessage,
        current: &StateMessage,
        tick_count: usize,
        parameters: &SearchSuggestorParameters,
    ) {
        let tick_count = tick_count.max(1);
        let replay_stride = parameters.teammate_replay_stride.max(1);
        for chunk_start in (0..tick_count).step_by(replay_stride) {
            let sampled_tick_count = (tick_count - chunk_start).min(replay_stride);
            let alpha = (chunk_start as f32 + sampled_tick_count as f32 * 0.5) / tick_count as f32;
            let (pose, heading_direction) =
                interpolate_teammate_pose_and_heading(previous, current, alpha);
            self.decay_tiles_from_field_pose_and_heading_for_sampled_ticks(
                field_dimensions,
                pose,
                heading_direction,
                FieldOfViewDecay {
                    distance_factor: teammate_decay_factor(parameters),
                    range: parameters.heatmap_decay_range.clone(),
                    sampled_tick_count,
                    occluders: &[],
                    occluded_factor: 1.0,
                },
            );
        }
    }

    pub fn decay_tiles_in_robot_fov_with_occluders(
        &mut self,
        field_dimensions: FieldDimensions,
        ground_to_field: Isometry2<Ground, Field>,
        parameters: &SearchSuggestorParameters,
        occluders: &[SearchOccluder],
    ) {
        let body_orientation = ground_to_field.orientation().angle();
        let heading_direction: Vector2<Field> =
            vector![body_orientation.cos(), body_orientation.sin()];
        self.decay_tiles_from_field_pose_and_heading_for_sampled_ticks(
            field_dimensions,
            ground_to_field.as_pose(),
            heading_direction,
            FieldOfViewDecay {
                distance_factor: parameters.decay_distance_factor,
                range: parameters.heatmap_decay_range.clone(),
                sampled_tick_count: 1,
                occluders,
                occluded_factor: parameters.occluded_decay_factor,
            },
        );
    }

    fn field_to_heatmap(
        &self,
        field_dimensions: FieldDimensions,
        field_point: Point2<Field>,
    ) -> (usize, usize) {
        let heatmap_point = (
            (field_point.x() + field_dimensions.length / 2.0).floor(),
            (field_point.y() + field_dimensions.width / 2.0).floor(),
        );
        (
            clamp(heatmap_point.0, 0.0, (self.map.dim().0 - 1) as f32) as usize,
            clamp(heatmap_point.1, 0.0, (self.map.dim().1 - 1) as f32) as usize,
        )
    }

    pub fn regenerate_restart_hypotheses(
        &mut self,
        filtered_game_controller_state: &FilteredGameControllerState,
        field_dimensions: FieldDimensions,
        increment: f32,
    ) -> bool {
        let Some(restart_indices) =
            self.restart_hypothesis_indices(filtered_game_controller_state, field_dimensions)
        else {
            return false;
        };
        self.map.indexed_iter_mut().for_each(|(index, value)| {
            if !restart_indices.contains(&index) {
                *value = 0.0;
            }
        });
        for index in restart_indices {
            self.map[index] += increment;
        }
        true
    }

    fn restart_hypothesis_indices(
        &self,
        filtered_game_controller_state: &FilteredGameControllerState,
        field_dimensions: FieldDimensions,
    ) -> Option<Vec<(usize, usize)>> {
        let mut indices: Vec<_> = match filtered_game_controller_state.sub_state {
            Some(SubState::CornerKick) => corner_kick_hypotheses(
                filtered_game_controller_state.kicking_team,
                field_dimensions,
            )
            .into_iter()
            .map(|hypothesis| self.field_to_heatmap(field_dimensions, hypothesis))
            .collect(),
            Some(SubState::GoalKick) => goal_kick_hypotheses(
                filtered_game_controller_state.kicking_team,
                field_dimensions,
            )
            .into_iter()
            .map(|hypothesis| self.field_to_heatmap(field_dimensions, hypothesis))
            .collect(),
            Some(SubState::ThrowIn) => {
                let last_y = self.map.dim().1 - 1;
                (0..self.map.dim().0)
                    .flat_map(|x| [(x, 0), (x, last_y)])
                    .collect()
            }
            _ => return None,
        };

        indices.sort_unstable();
        indices.dedup();
        Some(indices)
    }
}

fn teammate_decay_factor(parameters: &SearchSuggestorParameters) -> f32 {
    if parameters.teammate_decay_factor > 0.0 {
        parameters.teammate_decay_factor
    } else {
        parameters.decay_distance_factor * 0.25
    }
}

fn interpolate_teammate_pose_and_heading(
    previous: &StateMessage,
    current: &StateMessage,
    alpha: f32,
) -> (Pose2<Field>, Vector2<Field>) {
    let alpha = alpha.clamp(0.0, 1.0);
    let previous_position = previous.pose.position();
    let current_position = current.pose.position();
    let interpolated_position = point![
        previous_position.x() + (current_position.x() - previous_position.x()) * alpha,
        previous_position.y() + (current_position.y() - previous_position.y()) * alpha
    ];
    let previous_heading = previous.pose.orientation().angle() + previous.head_yaw;
    let current_heading = current.pose.orientation().angle() + current.head_yaw;
    let interpolated_heading =
        previous_heading + normalize_angle(current_heading - previous_heading) * alpha;
    let heading_direction = vector![interpolated_heading.cos(), interpolated_heading.sin()];

    (
        Pose2::new(interpolated_position, interpolated_heading),
        heading_direction,
    )
}

fn normalize_angle(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

fn heatmap_dimensions(field_dimensions: FieldDimensions) -> (usize, usize) {
    (
        field_dimensions.length.ceil().max(1.0) as usize,
        field_dimensions.width.ceil().max(1.0) as usize,
    )
}

fn search_voronoi_grid(field_dimensions: FieldDimensions) -> VoronoiGrid {
    let (length, width) = heatmap_dimensions(field_dimensions);
    let grid_min = point![
        -field_dimensions.length / 2.0,
        -field_dimensions.width / 2.0
    ];
    let grid_max = point![grid_min.x() + length as f32, grid_min.y() + width as f32];
    VoronoiGrid::new(
        VoronoiBounds {
            grid_min,
            grid_max,
            centroid_min: grid_min,
            centroid_max: grid_max,
        },
        VoronoiParameters {
            grid_resolution: 1.0,
            ..Default::default()
        },
    )
}

fn heatmap_tile_center(field_dimensions: FieldDimensions, (x, y): (usize, usize)) -> Point2<Field> {
    point![
        x as f32 + 0.5 - field_dimensions.length / 2.0,
        y as f32 + 0.5 - field_dimensions.width / 2.0
    ]
}

fn get_rule_hypotheses(
    primary_state: PrimaryState,
    filtered_game_controller_state: &FilteredGameControllerState,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    match (primary_state, filtered_game_controller_state.sub_state) {
        (PrimaryState::Ready, Some(SubState::PenaltyKick)) => {
            let kicking_team_half = kicking_team_half(filtered_game_controller_state.kicking_team)
                .unwrap_or(Half::Own)
                .mirror();
            vec![field_dimensions.penalty_spot(kicking_team_half)]
        }
        (PrimaryState::Ready, None) => vec![field_dimensions.center()],
        (PrimaryState::Playing, Some(SubState::CornerKick)) => corner_kick_hypotheses(
            filtered_game_controller_state.kicking_team,
            field_dimensions,
        ),
        (PrimaryState::Playing, Some(SubState::GoalKick)) => goal_kick_hypotheses(
            filtered_game_controller_state.kicking_team,
            field_dimensions,
        ),
        (_, _) => Vec::new(),
    }
}

fn corner_kick_hypotheses(
    kicking_team: Option<Team>,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    if let Some(kicking_team_half) = kicking_team_half(kicking_team) {
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

fn goal_kick_hypotheses(
    kicking_team: Option<Team>,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    if let Some(kicking_team_half) = kicking_team_half(kicking_team) {
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

fn kicking_team_half(kicking_team: Option<Team>) -> Option<Half> {
    match kicking_team {
        Some(Team::Opponent) => Some(Half::Opponent),
        Some(Team::Hulks) => Some(Half::Own),
        None => None,
    }
}

fn turn_score(
    (x, y): (usize, usize),
    field_dimensions: FieldDimensions,
    robot_position: Vector2<Field>,
    robot_heading: Vector2<Field>,
) -> f32 {
    let tile_center: Vector2<Field> = vector![
        x as f32 + 0.5 - field_dimensions.length / 2.0,
        y as f32 + 0.5 - field_dimensions.width / 2.0
    ];
    let robot_to_tile = tile_center - robot_position;
    let robot_to_tile_norm = robot_to_tile.norm();
    let robot_heading_norm = robot_heading.norm();
    if robot_to_tile_norm <= f32::EPSILON || robot_heading_norm <= f32::EPSILON {
        return 1.0;
    }
    robot_heading.dot(&robot_to_tile) / (robot_heading_norm * robot_to_tile_norm)
}

fn is_occluded(
    observer: Vector2<Field>,
    tile_center: Vector2<Field>,
    occluders: &[SearchOccluder],
) -> bool {
    let observer_to_tile = tile_center - observer;
    let tile_distance_squared = observer_to_tile.norm_squared();
    if tile_distance_squared <= f32::EPSILON {
        return false;
    }

    occluders.iter().any(|occluder| {
        if occluder.radius <= 0.0 {
            return false;
        }
        let observer_to_occluder = occluder.center.coords() - observer;
        let projection = observer_to_occluder.dot(&observer_to_tile) / tile_distance_squared;
        if !(0.0..1.0).contains(&projection) {
            return false;
        }
        let closest_point = observer + observer_to_tile * projection;
        (occluder.center.coords() - closest_point).norm() <= occluder.radius
    })
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

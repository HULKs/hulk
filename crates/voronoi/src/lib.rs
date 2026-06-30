use std::{cmp::Reverse, collections::BinaryHeap, f32::consts::SQRT_2};

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Pose2, point};
use ordered_float::NotNan;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{obstacles::Obstacle, parameters::VoronoiParameters, rule_obstacles::RuleObstacle};

type QueueItem = Reverse<(NotNan<f32>, usize, usize)>;
type Queue = BinaryHeap<QueueItem>;

const STRAIGHT_COST: f32 = 1.0;
const DIAGONAL_COST: f32 = SQRT_2;
const INV_SQRT_2: f32 = 1.0 / DIAGONAL_COST;

struct NearestCell {
    pub index: usize,
    pub cost: f32,
}

#[derive(Copy, Clone, Debug)]
struct Neighbor {
    pub dx: isize,
    pub dy: isize,
    pub step_cost: f32,
    pub inv_norm: f32,
}

impl Neighbor {
    const fn new(dx: isize, dy: isize, step_cost: f32, inv_norm: f32) -> Self {
        Self {
            dx,
            dy,
            step_cost,
            inv_norm,
        }
    }
}

const NEIGHBORS: [Neighbor; 8] = [
    Neighbor::new(1, 0, STRAIGHT_COST, 1.0),
    Neighbor::new(1, 1, DIAGONAL_COST, INV_SQRT_2),
    Neighbor::new(0, 1, STRAIGHT_COST, 1.0),
    Neighbor::new(-1, 1, DIAGONAL_COST, INV_SQRT_2),
    Neighbor::new(-1, 0, STRAIGHT_COST, 1.0),
    Neighbor::new(-1, -1, DIAGONAL_COST, INV_SQRT_2),
    Neighbor::new(0, -1, STRAIGHT_COST, 1.0),
    Neighbor::new(1, -1, DIAGONAL_COST, INV_SQRT_2),
];

#[derive(
    PartialEq,
    Clone,
    Copy,
    Default,
    Debug,
    Deserialize,
    Serialize,
    PathIntrospect,
    PathSerialize,
    Message,
)]
pub enum Ownership {
    Blocked,
    Robot(PlayerNumber),
    #[default]
    Free,
}

#[derive(
    Clone,
    Debug,
    Deserialize,
    Serialize,
    Default,
    PartialEq,
    PathIntrospect,
    PathDeserialize,
    PathSerialize,
    Message,
)]

pub struct VoronoiBounds {
    pub grid_min: Point2<Field>,
    pub grid_max: Point2<Field>,
    pub centroid_min: Point2<Field>,
    pub centroid_max: Point2<Field>,
}

#[derive(
    PartialEq,
    Clone,
    Debug,
    Deserialize,
    Serialize,
    Default,
    PathIntrospect,
    PathDeserialize,
    PathSerialize,
    Message,
)]
pub struct VoronoiGrid {
    pub tiles: Vec<Ownership>,
    width_tiles: usize,
    height_tiles: usize,
    parameters: VoronoiParameters,
    bounds: VoronoiBounds,
}

impl VoronoiGrid {
    pub fn new(bounds: VoronoiBounds, parameters: VoronoiParameters) -> Self {
        let resolution = parameters.grid_resolution;
        let width_tiles =
            ((bounds.grid_max.x() - bounds.grid_min.x()) / resolution).round() as usize;
        let height_tiles =
            ((bounds.grid_max.y() - bounds.grid_min.y()) / resolution).round() as usize;
        let tile_count = (width_tiles) * (height_tiles);

        Self {
            tiles: vec![Ownership::Free; tile_count],
            width_tiles,
            height_tiles,
            parameters,
            bounds,
        }
    }

    pub fn initialize_obstacles(
        &mut self,
        obstacles: &[Obstacle],
        rule_obstacles: &[RuleObstacle],
        ground_to_field: Isometry2<Ground, Field>,
    ) {
        for obstacle in obstacles.iter() {
            let radius = obstacle
                .radius_at_hip_height
                .max(obstacle.radius_at_foot_height);
            let center = ground_to_field * obstacle.position;

            let radius_squared = radius * radius;
            self.rasterize_bounds(
                center.x() - radius,
                center.x() + radius,
                center.y() - radius,
                center.y() + radius,
                |grid_point| (grid_point - center).norm_squared() <= radius_squared,
            );
        }

        for rule_obstacle in rule_obstacles.iter() {
            match rule_obstacle {
                RuleObstacle::Circle(circle) => {
                    let radius_squared = circle.radius * circle.radius;
                    self.rasterize_bounds(
                        circle.center.x() - circle.radius,
                        circle.center.x() + circle.radius,
                        circle.center.y() - circle.radius,
                        circle.center.y() + circle.radius,
                        |grid_point| (grid_point - circle.center).norm_squared() <= radius_squared,
                    );
                }
                RuleObstacle::Rectangle(rectangle) => {
                    self.rasterize_bounds(
                        rectangle.min.x(),
                        rectangle.max.x(),
                        rectangle.min.y(),
                        rectangle.max.y(),
                        |grid_point| rectangle.contains(grid_point),
                    );
                }
            }
        }
    }

    pub fn multi_source_dijkstra(
        &mut self,
        robots: &[(Pose2<Field>, PlayerNumber)],
        orientation_bias: f32,
    ) {
        if !self.is_valid_grid() {
            return;
        }
        let (mut distance, mut queue, robot_headings) = self.prepare_dijkstra(robots);

        while let Some(Reverse((current_cost, current_index, robot_index))) = queue.pop() {
            let current_cost = current_cost.into_inner();
            if current_cost > distance[current_index] {
                continue;
            }

            let player_number = robots[robot_index].1;
            let (sin_h, cos_h) = robot_headings[robot_index];

            for (neighbor_index, neighbor) in self.neighbor_indices(current_index) {
                if self.tiles[neighbor_index] == Ownership::Blocked {
                    continue;
                }

                let rotation_cost = rotation_cost(sin_h, cos_h, neighbor, orientation_bias);
                let new_cost = current_cost + neighbor.step_cost + rotation_cost;

                if new_cost < distance[neighbor_index] {
                    distance[neighbor_index] = new_cost;
                    self.tiles[neighbor_index] = Ownership::Robot(player_number);
                    queue.push(Reverse((
                        NotNan::new(new_cost).unwrap(),
                        neighbor_index,
                        robot_index,
                    )));
                }
            }
        }
    }

    fn is_valid_grid(&self) -> bool {
        self.width_tiles > 0
            && self.height_tiles > 0
            && self.width_tiles * self.height_tiles == self.tiles.len()
    }

    fn prepare_dijkstra(
        &mut self,
        robots: &[(Pose2<Field>, PlayerNumber)],
    ) -> (Vec<f32>, Queue, Vec<(f32, f32)>) {
        let mut distance = vec![f32::INFINITY; self.tiles.len()];
        let mut queue = Queue::new();

        let robot_headings: Vec<(f32, f32)> = robots
            .iter()
            .map(|(pose, _)| pose.orientation().angle().sin_cos())
            .collect();

        self.seed_sources(robots, &mut distance, &mut queue);

        (distance, queue, robot_headings)
    }

    fn seed_sources(
        &mut self,
        robots: &[(Pose2<Field>, PlayerNumber)],
        distance: &mut [f32],
        queue: &mut Queue,
    ) {
        let mut seed_distance = vec![f32::INFINITY; self.tiles.len()];
        let mut seed_queue = BinaryHeap::new();

        for (robot_index, (robot_pose, player_number)) in robots.iter().enumerate() {
            if let Some(seed_cell) = self.nearest_matching_cell(
                robot_pose.position(),
                |ownership| ownership == Ownership::Free,
                &mut seed_distance,
                &mut seed_queue,
            ) && seed_cell.cost < distance[seed_cell.index]
            {
                distance[seed_cell.index] = seed_cell.cost;
                self.tiles[seed_cell.index] = Ownership::Robot(*player_number);
                queue.push(Reverse((
                    NotNan::new(seed_cell.cost).unwrap(),
                    seed_cell.index,
                    robot_index,
                )));
            }
        }
    }

    pub fn nearest_non_blocked_ownership(&self, point: Point2<Field>) -> Option<Ownership> {
        let mut distance = vec![f32::INFINITY; self.tiles.len()];
        let mut queue = BinaryHeap::new();

        self.nearest_matching_cell(
            point,
            |ownership| matches!(ownership, Ownership::Robot(_)),
            &mut distance,
            &mut queue,
        )
        .map(|nearest_cell| self.tiles[nearest_cell.index])
    }

    fn nearest_matching_cell(
        &self,
        point: Point2<Field>,
        matches_ownership: impl Fn(Ownership) -> bool,
        distance: &mut [f32],
        queue: &mut BinaryHeap<Reverse<(NotNan<f32>, usize)>>,
    ) -> Option<NearestCell> {
        let start_index = self.point_to_index(point)?;
        if matches_ownership(self.tiles[start_index]) {
            return Some(NearestCell {
                index: start_index,
                cost: 0.0,
            });
        }

        let mut touched = Vec::new();
        queue.clear();

        distance[start_index] = 0.0;
        touched.push(start_index);
        queue.push(Reverse((NotNan::new(0.0).unwrap(), start_index)));

        while let Some(Reverse((current_cost, current_index))) = queue.pop() {
            let current_cost = current_cost.into_inner();
            if current_cost > distance[current_index] {
                continue;
            }
            if matches_ownership(self.tiles[current_index]) {
                for index in touched {
                    distance[index] = f32::INFINITY;
                }
                return Some(NearestCell {
                    index: current_index,
                    cost: current_cost,
                });
            }
            for (neighbor_index, neighbor) in self.neighbor_indices(current_index) {
                let new_cost = current_cost + neighbor.step_cost;
                if new_cost < distance[neighbor_index] {
                    distance[neighbor_index] = new_cost;
                    queue.push(Reverse((NotNan::new(new_cost).unwrap(), neighbor_index)));
                    touched.push(neighbor_index);
                }
            }
        }
        for index in touched {
            distance[index] = f32::INFINITY;
        }
        None
    }

    pub fn target_player_position(
        &self,
        player: PlayerNumber,
        ball_position: Option<Point2<Field>>,
    ) -> Option<Point2<Field>> {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut count = 0;
        let mut candidates = Vec::new();

        for (index, ownership) in self.tiles.iter().copied().enumerate() {
            if ownership != Ownership::Robot(player) {
                continue;
            }

            let point = self.index_to_point(index);
            candidates.push(point);

            if self.cell_overlaps_centroid_bounds(index) {
                sum_x += point.x();
                sum_y += point.y();
                count += 1;
            }
        }

        if count == 0 {
            return None;
        }

        let inv_count = 1.0 / count as f32;
        let centroid: Point2<Field> = point![sum_x * inv_count, sum_y * inv_count];

        let Some(ball_position) = ball_position else {
            return Some(centroid);
        };

        let field_length = self.bounds.grid_max.x() - self.bounds.grid_min.x();
        let half_length = field_length * 0.5;
        let ball_x = ball_position.x();
        let ball_y = ball_position.y();
        let side_factor = (ball_x / half_length).clamp(-1.0, 1.0);

        let support_distance = self
            .parameters
            .ball_support_distance
            .max(self.parameters.grid_resolution);
        let support_sigma = self
            .parameters
            .ball_support_sigma
            .max(self.parameters.grid_resolution);
        let inv_two_support_sigma_sq = 1.0 / (2.0 * support_sigma * support_sigma);

        let centroid_sigma = self
            .parameters
            .centroid_anchor_sigma
            .max(self.parameters.grid_resolution);

        let mut best_target = None;

        for point in candidates {
            let forward_norm = point.x() / half_length;
            let forward_term = self.parameters.forward_weight * side_factor * forward_norm;

            let dx_ball = point.x() - ball_x;
            let dy_ball = point.y() - ball_y;
            let ball_distance = (dx_ball * dx_ball + dy_ball * dy_ball).sqrt();
            let support_distance_error = ball_distance - support_distance;
            let ball_term = self.parameters.ball_weight
                * (-(support_distance_error * support_distance_error) * inv_two_support_sigma_sq)
                    .exp();

            let dx_centroid = point.x() - centroid.x();
            let dy_centroid = point.y() - centroid.y();
            let centroid_penalty = self.parameters.centroid_anchor_weight
                * (dx_centroid * dx_centroid + dy_centroid * dy_centroid).sqrt()
                / centroid_sigma;

            let score = forward_term + ball_term - centroid_penalty;
            if best_target.is_none_or(|(best_score, _)| score > best_score) {
                best_target = Some((score, point));
            }
        }

        best_target.map(|(_, point)| point)
    }

    fn point_to_index(&self, p: Point2<Field>) -> Option<usize> {
        let ix =
            ((p.x() - self.bounds.grid_min.x()) / self.parameters.grid_resolution).floor() as isize;
        let iy =
            ((p.y() - self.bounds.grid_min.y()) / self.parameters.grid_resolution).floor() as isize;

        if (0..self.width_tiles as isize).contains(&ix)
            && (0..self.height_tiles as isize).contains(&iy)
        {
            Some(index_from_xy(self.width_tiles, ix as usize, iy as usize))
        } else {
            None
        }
    }

    pub fn index_to_point(&self, index: usize) -> Point2<Field> {
        let (x, y) = xy_from_index(self.width_tiles, index);
        let resolution = self.parameters.grid_resolution;
        point!(
            self.bounds.grid_min.x() + (x as f32) * resolution + resolution / 2.0,
            self.bounds.grid_min.y() + (y as f32) * resolution + resolution / 2.0
        )
    }

    fn cell_overlaps_centroid_bounds(&self, index: usize) -> bool {
        let (x, y) = xy_from_index(self.width_tiles, index);
        let resolution = self.parameters.grid_resolution;

        let min_x = self.bounds.grid_min.x() + (x as f32) * resolution;
        let max_x = min_x + resolution;
        let min_y = self.bounds.grid_min.y() + (y as f32) * resolution;
        let max_y = min_y + resolution;

        min_x < self.bounds.centroid_max.x()
            && max_x > self.bounds.centroid_min.x()
            && min_y < self.bounds.centroid_max.y()
            && max_y > self.bounds.centroid_min.y()
    }

    fn tile_range_for_bounds(
        &self,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
    ) -> Option<(usize, usize, usize, usize)> {
        if self.width_tiles == 0 || self.height_tiles == 0 {
            return None;
        }

        let resolution = self.parameters.grid_resolution;

        let tile_min_x = self.bounds.grid_min.x();
        let tile_max_x = self.bounds.grid_min.x() + self.width_tiles as f32 * resolution;
        let tile_min_y = self.bounds.grid_min.y();
        let tile_max_y = self.bounds.grid_min.y() + self.height_tiles as f32 * resolution;

        if max_x < tile_min_x || min_x > tile_max_x || max_y < tile_min_y || min_y > tile_max_y {
            return None;
        }

        let mut min_x_index = ((min_x - tile_min_x) / resolution).floor() as isize - 1;
        let mut max_x_index = ((max_x - tile_min_x) / resolution).floor() as isize + 1;
        let mut min_y_index = ((min_y - tile_min_y) / resolution).floor() as isize - 1;
        let mut max_y_index = ((max_y - tile_min_y) / resolution).floor() as isize + 1;

        min_x_index = min_x_index.clamp(0, self.width_tiles as isize - 1);
        max_x_index = max_x_index.clamp(0, self.width_tiles as isize - 1);
        min_y_index = min_y_index.clamp(0, self.height_tiles as isize - 1);
        max_y_index = max_y_index.clamp(0, self.height_tiles as isize - 1);

        Some((
            min_x_index as usize,
            max_x_index as usize,
            min_y_index as usize,
            max_y_index as usize,
        ))
    }

    fn rasterize_bounds(
        &mut self,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
        mut contains: impl FnMut(Point2<Field>) -> bool,
    ) {
        let Some((min_x, max_x, min_y, max_y)) =
            self.tile_range_for_bounds(min_x, max_x, min_y, max_y)
        else {
            return;
        };

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let index = index_from_xy(self.width_tiles, x, y);
                let grid_point = self.index_to_point(index);
                if contains(grid_point) {
                    self.tiles[index] = Ownership::Blocked;
                }
            }
        }
    }

    fn neighbor_indices(&self, index: usize) -> impl Iterator<Item = (usize, Neighbor)> + use<> {
        let (x, y) = xy_from_index(self.width_tiles, index);
        let width_tiles = self.width_tiles;
        let height_tiles = self.height_tiles;

        NEIGHBORS.into_iter().filter_map(move |neighbor| {
            let nx = x as isize + neighbor.dx;
            let ny = y as isize + neighbor.dy;
            if !(0..width_tiles as isize).contains(&nx) || !(0..height_tiles as isize).contains(&ny)
            {
                return None;
            }
            Some((
                index_from_xy(width_tiles, nx as usize, ny as usize),
                neighbor,
            ))
        })
    }

    pub fn ownership_at(&self, point: Point2<Field>) -> Option<Ownership> {
        self.point_to_index(point).map(|index| self.tiles[index])
    }
}

fn rotation_cost(sin_h: f32, cos_h: f32, neighbor: Neighbor, orientation_bias: f32) -> f32 {
    let dot = (cos_h * neighbor.dx as f32 + sin_h * neighbor.dy as f32) * neighbor.inv_norm;
    let turn_factor = (1.0 - dot.clamp(-1.0, 1.0)) * 0.5;
    (turn_factor * orientation_bias).max(0.0)
}

fn index_from_xy(width_tiles: usize, x: usize, y: usize) -> usize {
    y * width_tiles + x
}

fn xy_from_index(width_tiles: usize, index: usize) -> (usize, usize) {
    let x = index % width_tiles;
    let y = index / width_tiles;
    (x, y)
}

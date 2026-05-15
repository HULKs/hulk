use std::{cmp::Reverse, collections::BinaryHeap, f32::consts::SQRT_2};

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Pose2, point};
use ordered_float::NotNan;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{obstacles::Obstacle, rule_obstacles::RuleObstacle};

type QueueItem = Reverse<(NotNan<f32>, usize, usize)>;
type Queue = BinaryHeap<QueueItem>;

const STRAIGHT_COST: f32 = 1.0;
const DIAGONAL_COST: f32 = SQRT_2;
const INV_SQRT_2: f32 = 1.0 / DIAGONAL_COST;

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

impl Ownership {
    fn is_blocked(self) -> bool {
        matches!(self, Self::Blocked)
    }

    fn is_free(self) -> bool {
        matches!(self, Self::Free)
    }
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
    pub width_tiles: usize,
    pub height_tiles: usize,
    pub resolution: f32,
    pub min_bound: Point2<Field>,
}

impl VoronoiGrid {
    pub fn new(width: f32, height: f32, padding: f32, resolution: f32) -> Self {
        let width_tiles = ((width + 2.0 * padding) / resolution).round() as usize;
        let height_tiles = ((height + 2.0 * padding) / resolution).round() as usize;
        let tile_count = (width_tiles) * (height_tiles);

        Self {
            tiles: vec![Ownership::Free; tile_count],
            width_tiles,
            height_tiles,
            resolution,
            min_bound: point!(-width / 2.0 - padding, -height / 2.0 - padding),
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

            let width_tiles = self.width_tiles;
            let height_tiles = self.height_tiles;
            let x = (current_index % width_tiles) as isize;
            let y = (current_index / width_tiles) as isize;

            for neighbor in NEIGHBORS {
                let nx = x + neighbor.dx;
                let ny = y + neighbor.dy;
                if !(0..width_tiles as isize).contains(&nx)
                    || !(0..height_tiles as isize).contains(&ny)
                {
                    continue;
                }

                let neighbor_index = self.index_from_xy(nx as usize, ny as usize);
                if self.tiles[neighbor_index].is_blocked() {
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
        for (robot_index, (robot_pose, player_number)) in robots.iter().enumerate() {
            if let Some(start_index) = self.nearest_non_blocked_cell_index(robot_pose.position())
                && distance[start_index] > 0.0
            {
                distance[start_index] = 0.0;
                self.tiles[start_index] = Ownership::Robot(*player_number);
                queue.push(Reverse((
                    NotNan::new(0.0).unwrap(),
                    start_index,
                    robot_index,
                )));
            }
        }
    }

    pub fn centroid_for_player(&self, player: PlayerNumber) -> Option<Point2<Field>> {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut count = 0;

        for (index, ownership) in self.tiles.iter().copied().enumerate() {
            if ownership == Ownership::Robot(player) {
                let point = self.index_to_point(index);
                sum_x += point.x();
                sum_y += point.y();
                count += 1;
            }
        }

        if count == 0 {
            None
        } else {
            let inv_count = 1.0 / count as f32;
            Some(point![sum_x * inv_count, sum_y * inv_count])
        }
    }

    fn point_to_index(&self, p: Point2<Field>) -> Option<usize> {
        let ix = ((p.x() - self.min_bound.x()) / self.resolution).floor() as isize;
        let iy = ((p.y() - self.min_bound.y()) / self.resolution).floor() as isize;

        if (0..self.width_tiles as isize).contains(&ix)
            && (0..self.height_tiles as isize).contains(&iy)
        {
            Some(self.index_from_xy(ix as usize, iy as usize))
        } else {
            None
        }
    }

    pub fn index_to_point(&self, index: usize) -> Point2<Field> {
        let width_tiles = self.width_tiles;
        let ix = (index % width_tiles) as f32;
        let iy = (index / width_tiles) as f32;

        point!(
            self.min_bound.x() + ix * self.resolution + self.resolution / 2.0,
            self.min_bound.y() + iy * self.resolution + self.resolution / 2.0
        )
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

        let tile_min_x = self.min_bound.x();
        let tile_max_x = self.min_bound.x() + self.width_tiles as f32 * self.resolution;
        let tile_min_y = self.min_bound.y();
        let tile_max_y = self.min_bound.y() + self.height_tiles as f32 * self.resolution;

        if max_x < tile_min_x || min_x > tile_max_x || max_y < tile_min_y || min_y > tile_max_y {
            return None;
        }

        let mut min_x_index = ((min_x - tile_min_x) / self.resolution).floor() as isize - 1;
        let mut max_x_index = ((max_x - tile_min_x) / self.resolution).floor() as isize + 1;
        let mut min_y_index = ((min_y - tile_min_y) / self.resolution).floor() as isize - 1;
        let mut max_y_index = ((max_y - tile_min_y) / self.resolution).floor() as isize + 1;

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

    fn nearest_non_blocked_cell_index(&self, point: Point2<Field>) -> Option<usize> {
        let start_index = self.point_to_index(point)?;
        if self.tiles[start_index].is_free() {
            return Some(start_index);
        }

        let width_tiles = self.width_tiles;
        let height_tiles = self.height_tiles;
        let start_x = start_index % width_tiles;
        let start_y = start_index / width_tiles;
        let max_radius = self.width_tiles.max(self.height_tiles);

        for radius in 1..=max_radius {
            let min_x = start_x.saturating_sub(radius);
            let max_x = (start_x + radius).min(width_tiles - 1);
            let min_y = start_y.saturating_sub(radius);
            let max_y = (start_y + radius).min(height_tiles - 1);

            for x in min_x..=max_x {
                for y in [min_y, max_y] {
                    let index = self.index_from_xy(x, y);
                    if self.tiles[index].is_free() {
                        return Some(index);
                    }
                }
            }

            if max_y > min_y {
                for y in (min_y + 1)..max_y {
                    for x in [min_x, max_x] {
                        let index = self.index_from_xy(x, y);
                        if self.tiles[index].is_free() {
                            return Some(index);
                        }
                    }
                }
            }
        }

        None
    }

    fn index_from_xy(&self, x: usize, y: usize) -> usize {
        y * (self.width_tiles) + x
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
                let index = self.index_from_xy(x, y);
                let grid_point = self.index_to_point(index);
                if contains(grid_point) {
                    self.tiles[index] = Ownership::Blocked;
                }
            }
        }
    }
}

fn rotation_cost(sin_h: f32, cos_h: f32, neighbor: Neighbor, orientation_bias: f32) -> f32 {
    let dot = (cos_h * neighbor.dx as f32 + sin_h * neighbor.dy as f32) * neighbor.inv_norm;
    let turn_factor = (1.0 - dot.clamp(-1.0, 1.0)) * 0.5;
    (turn_factor * orientation_bias).max(0.0)
}

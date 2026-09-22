use std::{cmp::Reverse, collections::BinaryHeap, f32::consts::SQRT_2};

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Pose2, point};
use ordered_float::NotNan;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{obstacles::Obstacle, rule_obstacles::RuleObstacle};

type QueueItem = Reverse<(NotNan<f32>, usize, PlayerNumber)>;
type Queue = BinaryHeap<QueueItem>;

const STRAIGHT_COST: f32 = 1.0;
const DIAGONAL_COST: f32 = SQRT_2;

struct NearestCell {
    pub index: usize,
    pub cost: f32,
}

#[derive(Copy, Clone, Debug)]
struct Neighbor {
    pub dx: isize,
    pub dy: isize,
    pub step_cost: f32,
}

impl Neighbor {
    const fn new(dx: isize, dy: isize, step_cost: f32) -> Self {
        Self { dx, dy, step_cost }
    }
}

const NEIGHBORS: [Neighbor; 8] = [
    Neighbor::new(1, 0, STRAIGHT_COST),
    Neighbor::new(1, 1, DIAGONAL_COST),
    Neighbor::new(0, 1, STRAIGHT_COST),
    Neighbor::new(-1, 1, DIAGONAL_COST),
    Neighbor::new(-1, 0, STRAIGHT_COST),
    Neighbor::new(-1, -1, DIAGONAL_COST),
    Neighbor::new(0, -1, STRAIGHT_COST),
    Neighbor::new(1, -1, DIAGONAL_COST),
];

#[derive(PartialEq, Clone, Copy, Default, Debug, Deserialize, Serialize, Message)]
pub enum Ownership {
    Blocked,
    Robot(PlayerNumber),
    #[default]
    Free,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq, Message)]

pub struct GridGeometry {
    grid_min: Point2<Field>,
    grid_max: Point2<Field>,
    resolution: f32,
    width: usize,
    height: usize,
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize, Default, Message)]
pub struct VoronoiGrid {
    geometry: GridGeometry,
    tiles: Vec<Ownership>,
}

impl VoronoiGrid {
    pub fn new(grid_min: Point2<Field>, grid_max: Point2<Field>, resolution: f32) -> Self {
        let min_cell = grid_min.map(|min| (min / resolution + 0.5).floor());
        let max_cell = grid_max.map(|max| (max / resolution - 0.5).ceil());

        let grid_min = min_cell.map(|min| (min - 0.5) * resolution);
        let grid_max = max_cell.map(|max| (max + 0.5) * resolution);

        let width = (max_cell.x() - min_cell.x() + 1.0) as usize;
        let height = (max_cell.y() - min_cell.y() + 1.0) as usize;

        let tile_count = width * height;

        let geometry = GridGeometry {
            grid_min,
            grid_max,
            resolution,
            width,
            height,
        };

        Self {
            geometry,
            tiles: vec![Ownership::Free; tile_count],
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

    pub fn multi_source_dijkstra(&mut self, robots: &[(Pose2<Field>, PlayerNumber)]) {
        if !self.is_valid_grid() {
            return;
        }
        let (mut distance, mut queue) = self.prepare_dijkstra(robots);

        while let Some(Reverse((current_cost, current_index, player_number))) = queue.pop() {
            let current_cost = current_cost.into_inner();
            if current_cost > distance[current_index]
                || self.tiles[current_index] != Ownership::Robot(player_number)
            {
                continue;
            }

            for (neighbor_index, neighbor) in self.neighbor_indices(current_index) {
                if self.tiles[neighbor_index] == Ownership::Blocked {
                    continue;
                }

                let new_cost = current_cost + neighbor.step_cost;

                self.relax(
                    neighbor_index,
                    new_cost,
                    player_number,
                    &mut distance,
                    &mut queue,
                );
            }
        }
    }

    fn relax(
        &mut self,
        index: usize,
        cost: f32,
        player_number: PlayerNumber,
        distance: &mut [f32],
        queue: &mut Queue,
    ) {
        // Equal-cost claims use the same tie-break on every robot, including at sources.
        let wins_tie = cost == distance[index]
            && matches!(self.tiles[index], Ownership::Robot(owner) if player_number < owner);
        if cost < distance[index] || wins_tie {
            distance[index] = cost;
            self.tiles[index] = Ownership::Robot(player_number);
            queue.push(Reverse((NotNan::new(cost).unwrap(), index, player_number)));
        }
    }

    fn is_valid_grid(&self) -> bool {
        self.geometry.width > 0
            && self.geometry.height > 0
            && self.geometry.width * self.geometry.height == self.tiles.len()
    }

    fn prepare_dijkstra(&mut self, robots: &[(Pose2<Field>, PlayerNumber)]) -> (Vec<f32>, Queue) {
        let mut distance = vec![f32::INFINITY; self.tiles.len()];
        let mut queue = Queue::new();

        self.seed_sources(robots, &mut distance, &mut queue);
        (distance, queue)
    }

    fn seed_sources(
        &mut self,
        robots: &[(Pose2<Field>, PlayerNumber)],
        distance: &mut [f32],
        queue: &mut Queue,
    ) {
        let mut seed_distance = vec![f32::INFINITY; self.tiles.len()];
        let mut seed_queue = BinaryHeap::new();

        for (robot_pose, player_number) in robots {
            if let Some(seed_cell) = self.nearest_matching_cell(
                robot_pose.position(),
                |ownership| ownership != Ownership::Blocked,
                &mut seed_distance,
                &mut seed_queue,
            ) {
                self.relax(
                    seed_cell.index,
                    seed_cell.cost,
                    *player_number,
                    distance,
                    queue,
                );
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

    fn point_to_index(&self, p: Point2<Field>) -> Option<usize> {
        let resolution = self.geometry.resolution;
        let min_cell = self
            .geometry
            .grid_min
            .map(|min| (min / resolution + 0.5).round());
        let ix = ((p.x() / resolution + 0.5).floor() - min_cell.x()) as isize;
        let iy = ((p.y() / resolution + 0.5).floor() - min_cell.y()) as isize;

        if (0..self.geometry.width as isize).contains(&ix)
            && (0..self.geometry.height as isize).contains(&iy)
        {
            Some(index_from_xy(self.geometry.width, ix as usize, iy as usize))
        } else {
            None
        }
    }

    pub fn index_to_point(&self, index: usize) -> Point2<Field> {
        let (x, y) = xy_from_index(self.geometry.width, index);
        let resolution = self.geometry.resolution;
        let min_cell = self
            .geometry
            .grid_min
            .map(|min| (min / resolution + 0.5).round());
        point!(
            (min_cell.x() + x as f32) * resolution,
            (min_cell.y() + y as f32) * resolution
        )
    }

    fn tile_range_for_bounds(
        &self,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
    ) -> Option<(usize, usize, usize, usize)> {
        let geometry = &self.geometry;
        if geometry.width == 0 || geometry.height == 0 {
            return None;
        }

        let resolution = geometry.resolution;

        let tile_min_x = geometry.grid_min.x();
        let tile_max_x = geometry.grid_min.x() + geometry.width as f32 * resolution;
        let tile_min_y = geometry.grid_min.y();
        let tile_max_y = geometry.grid_min.y() + geometry.height as f32 * resolution;

        if max_x < tile_min_x || min_x > tile_max_x || max_y < tile_min_y || min_y > tile_max_y {
            return None;
        }

        let mut min_x_index = ((min_x - tile_min_x) / resolution).floor() as isize - 1;
        let mut max_x_index = ((max_x - tile_min_x) / resolution).floor() as isize + 1;
        let mut min_y_index = ((min_y - tile_min_y) / resolution).floor() as isize - 1;
        let mut max_y_index = ((max_y - tile_min_y) / resolution).floor() as isize + 1;

        min_x_index = min_x_index.clamp(0, geometry.width as isize - 1);
        max_x_index = max_x_index.clamp(0, geometry.width as isize - 1);
        min_y_index = min_y_index.clamp(0, geometry.height as isize - 1);
        max_y_index = max_y_index.clamp(0, geometry.height as isize - 1);

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
                let index = index_from_xy(self.geometry.width, x, y);
                let grid_point = self.index_to_point(index);
                if contains(grid_point) {
                    self.tiles[index] = Ownership::Blocked;
                }
            }
        }
    }

    fn neighbor_indices(&self, index: usize) -> impl Iterator<Item = (usize, Neighbor)> + use<> {
        let (x, y) = xy_from_index(self.geometry.width, index);
        let width_tiles = self.geometry.width;
        let height_tiles = self.geometry.height;

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

    pub fn cells(&self) -> impl Iterator<Item = (Point2<Field>, Ownership)> + '_ {
        self.tiles
            .iter()
            .copied()
            .enumerate()
            .map(|(index, ownership)| (self.index_to_point(index), ownership))
    }

    pub fn resolution(&self) -> f32 {
        self.geometry.resolution
    }
}

fn index_from_xy(width_tiles: usize, x: usize, y: usize) -> usize {
    y * width_tiles + x
}

fn xy_from_index(width_tiles: usize, index: usize) -> (usize, usize) {
    let x = index % width_tiles;
    let y = index / width_tiles;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coincident_sources_do_not_create_artificial_territory() {
        let robots = [
            (Pose2::from(point!(0.1, 0.1)), PlayerNumber::Three),
            (Pose2::from(point!(-0.1, -0.1)), PlayerNumber::Two),
        ];
        let mut grid = VoronoiGrid::new(point!(-3.0, -3.0), point!(3.0, 3.0), 1.0);
        grid.multi_source_dijkstra(&robots);

        let mut reversed = VoronoiGrid::new(point!(-3.0, -3.0), point!(3.0, 3.0), 1.0);
        reversed.multi_source_dijkstra(&[robots[1], robots[0]]);

        assert_eq!(grid, reversed);
        assert!(
            grid.cells()
                .all(|(_, owner)| owner == Ownership::Robot(PlayerNumber::Two))
        );
    }

    #[test]
    fn equal_distance_claims_prefer_player_number_over_grid_index() {
        let robots = [
            (Pose2::from(point!(-1.0, 0.0)), PlayerNumber::Three),
            (Pose2::from(point!(1.0, 0.0)), PlayerNumber::Two),
        ];
        let mut grid = VoronoiGrid::new(point!(-3.0, -3.0), point!(3.0, 3.0), 1.0);
        grid.multi_source_dijkstra(&robots);

        let mut reversed = VoronoiGrid::new(point!(-3.0, -3.0), point!(3.0, 3.0), 1.0);
        reversed.multi_source_dijkstra(&[robots[1], robots[0]]);

        assert_eq!(grid, reversed);
        assert_eq!(
            grid.ownership_at(point!(0.0, 0.0)),
            Some(Ownership::Robot(PlayerNumber::Two))
        );
        assert_eq!(
            grid.ownership_at(point!(-1.0, 0.0)),
            Some(Ownership::Robot(PlayerNumber::Three))
        );
    }
}

use std::collections::VecDeque;

use coordinate_systems::Field;
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Point2, point};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Ownership {
    Blocked,
    Robot(PlayerNumber),
    Free,
}

pub struct Map {
    pub tiles: Vec<Ownership>,
    pub width_tiles: usize,
    pub height_tiles: usize,
    pub resolution: f32,
    pub min_bound: Point2<Field>,
}

impl Map {
    pub fn new(width: f32, height: f32, padding: f32, resolution: f32) -> Self {
        let width_tiles = ((width + 2.0 * padding) / resolution).round() as usize;
        let height_tiles = ((height + 2.0 * padding) / resolution).round() as usize;

        Self {
            tiles: vec![Ownership::Free; width_tiles * height_tiles],
            width_tiles,
            height_tiles,
            resolution,
            min_bound: point!(-width / 2.0 - padding, -height / 2.0 - padding),
        }
    }

    pub fn point_to_index(&self, p: Point2<Field>) -> Option<usize> {
        let ix = ((p.x() - self.min_bound.x()) / self.resolution).floor() as i32;
        let iy = ((p.y() - self.min_bound.y()) / self.resolution).floor() as i32;

        if ix >= 0 && ix < self.width_tiles as i32 && iy >= 0 && iy < self.height_tiles as i32 {
            Some((iy as usize * self.width_tiles) + ix as usize)
        } else {
            None
        }
    }

    pub fn index_to_point(&self, index: usize) -> Point2<Field> {
        let ix = (index % self.width_tiles) as f32;
        let iy = (index / self.width_tiles) as f32;

        point!(
            self.min_bound.x() + ix * self.resolution + self.resolution / 2.0,
            self.min_bound.y() + iy * self.resolution + self.resolution / 2.0
        )
    }

    pub fn nearest_non_blocked_cell_index(&self, point: Point2<Field>) -> Option<usize> {
        if let Some(start_index) = self.point_to_index(point) {
            if self.tiles[start_index] == Ownership::Free {
                return Some(start_index);
            }
            let mut visited = vec![false; self.tiles.len()];
            let mut queue = VecDeque::new();
            queue.push_back(start_index);

            while let Some(current_index) = queue.pop_front() {
                if visited[current_index] {
                    continue;
                }
                visited[current_index] = true;

                if self.tiles[current_index] == Ownership::Free {
                    return Some(current_index);
                }

                let x = current_index % self.width_tiles;
                let y = current_index / self.width_tiles;

                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0
                            && nx < self.width_tiles as isize
                            && ny >= 0
                            && ny < self.height_tiles as isize
                        {
                            let neighbor_index = (ny as usize) * self.width_tiles + (nx as usize);
                            if !visited[neighbor_index] {
                                queue.push_back(neighbor_index);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

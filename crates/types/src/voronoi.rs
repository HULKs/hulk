use coordinate_systems::Field;
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Point2, point};
use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

const STRAIGHT_COST: u32 = 10;
const DIAGONAL_COST: u32 = 14;
const INV_SQRT_2: f32 = 0.70710677;
pub const NEIGHBORS: [(isize, isize, u32, f32); 8] = [
    (1, 0, STRAIGHT_COST, 1.0),
    (1, 1, DIAGONAL_COST, INV_SQRT_2),
    (0, 1, STRAIGHT_COST, 1.0),
    (-1, 1, DIAGONAL_COST, INV_SQRT_2),
    (-1, 0, STRAIGHT_COST, 1.0),
    (-1, -1, DIAGONAL_COST, INV_SQRT_2),
    (0, -1, STRAIGHT_COST, 1.0),
    (1, -1, DIAGONAL_COST, INV_SQRT_2),
];

#[derive(
    PartialEq, Clone, Copy, Default, Debug, Deserialize, Serialize, PathIntrospect, PathSerialize,
)]
pub enum Ownership {
    Blocked,
    Robot(PlayerNumber),
    #[default]
    Free,
}

#[derive(
    PartialEq, Clone, Debug, Deserialize, Serialize, Default, PathIntrospect, PathSerialize,
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

    pub fn centroid_for_player(&self, player: PlayerNumber) -> Option<Point2<Field>> {
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut count = 0u32;

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

    pub fn nearest_non_blocked_cell_index(&self, point: Point2<Field>) -> Option<usize> {
        let start_index = self.point_to_index(point)?;
        if self.tiles[start_index] == Ownership::Free {
            return Some(start_index);
        }

        let start_x = start_index % self.width_tiles;
        let start_y = start_index / self.width_tiles;
        let max_radius = self.width_tiles.max(self.height_tiles);

        for radius in 1..=max_radius {
            let min_x = start_x.saturating_sub(radius);
            let max_x = (start_x + radius).min(self.width_tiles - 1);
            let min_y = start_y.saturating_sub(radius);
            let max_y = (start_y + radius).min(self.height_tiles - 1);

            for x in min_x..=max_x {
                for y in [min_y, max_y] {
                    let index = y * self.width_tiles + x;
                    if self.tiles[index] == Ownership::Free {
                        return Some(index);
                    }
                }
            }

            if max_y > min_y {
                for y in (min_y + 1)..max_y {
                    for x in [min_x, max_x] {
                        let index = y * self.width_tiles + x;
                        if self.tiles[index] == Ownership::Free {
                            return Some(index);
                        }
                    }
                }
            }
        }

        None
    }
}

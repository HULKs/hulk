use coordinate_systems::Field;
use linear_algebra::{Point2, point};
use nalgebra::clamp;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use types::{field_dimensions::FieldDimensions, heatmap::Heatmap as HeatmapMessage};

mod ball_observations;
mod field_of_view;
mod rule_hypotheses;
mod search_selection;
mod search_voronoi;

pub use ball_observations::ball_position_in_field;
pub use field_of_view::SearchOccluder;
pub use search_selection::SearchArea;
pub use search_voronoi::SearchVoronoiSelection;

pub use types::heatmap::grid_dimensions;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Heatmap {
    map: Array2<f32>,
    last_maximum_heatmap_position: Option<(usize, usize)>,
}

impl Heatmap {
    pub fn new(field_dimensions: FieldDimensions, cells_x: usize) -> Self {
        let (length, width) = grid_dimensions(field_dimensions, cells_x);
        Self {
            map: Array2::zeros((length, width)),
            last_maximum_heatmap_position: None,
        }
    }

    /// Regrid by sampling the old map at new cell centres; retain the target's location.
    pub fn resize(&mut self, field: FieldDimensions, cells_x: usize) {
        let shape = grid_dimensions(field, cells_x);
        if self.map.dim() == shape {
            return;
        }
        let target = self
            .last_maximum_heatmap_position
            .map(|index| heatmap_tile_center(field, self.map.dim(), index));
        let mut resized = Self::new(field, cells_x);
        for (index, value) in resized.map.indexed_iter_mut() {
            let point = heatmap_tile_center(field, shape, index);
            *value = self.map[self.field_to_heatmap(field, point)];
        }
        resized.last_maximum_heatmap_position =
            target.map(|point| resized.field_to_heatmap(field, point));
        *self = resized;
    }

    pub fn to_message(&self) -> HeatmapMessage {
        let (length, width) = self.map.dim();
        HeatmapMessage {
            length: length as u32,
            width: width as u32,
            values: self.map.iter().copied().collect(),
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

    /// Scale an overfilled map down uniformly so existing priority ratios remain intact.
    pub fn limit_mean_priority(&mut self, maximum_mean: f32) {
        let mean = self.map.sum() / self.map.len() as f32;
        if mean > maximum_mean {
            self.map *= maximum_mean / mean;
        }
    }

    fn field_to_heatmap(
        &self,
        field_dimensions: FieldDimensions,
        field_point: Point2<Field>,
    ) -> (usize, usize) {
        let heatmap_point = (
            ((field_point.x() + field_dimensions.length / 2.0)
                / (field_dimensions.length / self.map.dim().0 as f32))
                .floor(),
            ((field_point.y() + field_dimensions.width / 2.0)
                / (field_dimensions.width / self.map.dim().1 as f32))
                .floor(),
        );
        (
            clamp(heatmap_point.0, 0.0, (self.map.dim().0 - 1) as f32) as usize,
            clamp(heatmap_point.1, 0.0, (self.map.dim().1 - 1) as f32) as usize,
        )
    }
}

#[cfg(test)]
mod tests;

fn heatmap_tile_center(
    field: FieldDimensions,
    shape: (usize, usize),
    (x, y): (usize, usize),
) -> Point2<Field> {
    point![
        (x as f32 + 0.5) * field.length / shape.0 as f32 - field.length / 2.0,
        (y as f32 + 0.5) * field.width / shape.1 as f32 - field.width / 2.0
    ]
}

fn heatmap_tile_contains(
    field: FieldDimensions,
    shape: (usize, usize),
    (x, y): (usize, usize),
    point: Point2<Field>,
) -> bool {
    let dx = field.length / shape.0 as f32;
    let dy = field.width / shape.1 as f32;
    let min_x = x as f32 * dx - field.length / 2.0;
    let min_y = y as f32 * dy - field.width / 2.0;
    let max_x = if x + 1 == shape.0 {
        field.length / 2.0
    } else {
        min_x + dx
    };
    let max_y = if y + 1 == shape.1 {
        field.width / 2.0
    } else {
        min_y + dy
    };
    (min_x..=max_x).contains(&point.x()) && (min_y..=max_y).contains(&point.y())
}

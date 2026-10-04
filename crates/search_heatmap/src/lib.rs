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

pub use field_of_view::SearchOccluder;
pub use search_selection::SearchVoronoiSelection;

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

    pub fn clamp_values(&mut self) {
        self.map.mapv_inplace(|value| {
            if value.is_nan() {
                0.0
            } else {
                value.clamp(0.0, 1.0)
            }
        });
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
}

fn heatmap_dimensions(field_dimensions: FieldDimensions) -> (usize, usize) {
    (
        field_dimensions.length.ceil().max(1.0) as usize,
        field_dimensions.width.ceil().max(1.0) as usize,
    )
}

fn heatmap_tile_center(field_dimensions: FieldDimensions, (x, y): (usize, usize)) -> Point2<Field> {
    point![
        x as f32 + 0.5 - field_dimensions.length / 2.0,
        y as f32 + 0.5 - field_dimensions.width / 2.0
    ]
}

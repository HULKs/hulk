use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct Heatmap {
    pub length: u32,
    pub width: u32,
    pub values: Vec<f32>,
}

/// Fixed longitudinal resolution with approximately square cells covering the field exactly.
pub fn grid_dimensions(
    field: crate::field_dimensions::FieldDimensions,
    cells_x: usize,
) -> (usize, usize) {
    let cells_x = cells_x.max(1);
    (
        cells_x,
        (cells_x as f32 * field.width / field.length)
            .round()
            .max(1.0) as usize,
    )
}

/// Physical cell dimensions published by the search suggestor (metres in field axes).
pub const SEARCH_CELL_SIZE_TOPIC: &str = "search_cell_size";

pub fn grid_cell_size(
    field: crate::field_dimensions::FieldDimensions,
    cells_x: usize,
) -> linear_algebra::Vector2<coordinate_systems::Field> {
    let (nx, ny) = grid_dimensions(field, cells_x);
    linear_algebra::vector![field.length / nx as f32, field.width / ny as f32]
}

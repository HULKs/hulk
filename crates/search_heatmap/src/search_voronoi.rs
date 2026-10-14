use coordinate_systems::Field;
use hsl_network_messages::PlayerNumber;
use itertools::Itertools;
use linear_algebra::{Pose2, point};
use types::field_dimensions::FieldDimensions;
use voronoi::{Ownership, VoronoiGrid};

use crate::{CELL_SIZE, heatmap_dimensions, heatmap_tile_center};

#[derive(Clone, Debug)]
pub struct SearchVoronoiSelection {
    pub owner: PlayerNumber,
    pub grid: VoronoiGrid,
}

impl SearchVoronoiSelection {
    pub fn new(
        field_dimensions: FieldDimensions,
        owner: PlayerNumber,
        sites: impl IntoIterator<Item = (Pose2<Field>, PlayerNumber)>,
    ) -> Self {
        let mut grid = search_voronoi_grid(field_dimensions);
        grid.multi_source_dijkstra(&sites.into_iter().collect_vec());
        Self { owner, grid }
    }

    pub(crate) fn owns_index(
        &self,
        field_dimensions: FieldDimensions,
        index: (usize, usize),
    ) -> bool {
        self.grid
            .ownership_at(heatmap_tile_center(field_dimensions, index))
            .is_some_and(|ownership| ownership == Ownership::Robot(self.owner))
    }
}

fn search_voronoi_grid(field_dimensions: FieldDimensions) -> VoronoiGrid {
    let (length, width) = heatmap_dimensions(field_dimensions);
    let grid_min = point![
        -field_dimensions.length / 2.0,
        -field_dimensions.width / 2.0
    ];
    let grid_max = point![
        grid_min.x() + length as f32 * CELL_SIZE,
        grid_min.y() + width as f32 * CELL_SIZE
    ];
    VoronoiGrid::new(grid_min, grid_max, CELL_SIZE)
}

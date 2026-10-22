use coordinate_systems::Field;
use hsl_network_messages::PlayerNumber;
use itertools::Itertools;
use linear_algebra::{Point2, Pose2, point};
use types::field_dimensions::FieldDimensions;
use voronoi::{Ownership, VoronoiGrid};

#[derive(Clone, Debug)]
pub struct SearchVoronoiSelection {
    pub owner: PlayerNumber,
    pub grid: VoronoiGrid,
}

impl SearchVoronoiSelection {
    pub fn new(
        field_dimensions: FieldDimensions,
        cells_x: usize,
        owner: PlayerNumber,
        sites: impl IntoIterator<Item = (Pose2<Field>, PlayerNumber)>,
    ) -> Self {
        let mut grid = search_voronoi_grid(field_dimensions, cells_x);
        grid.multi_source_dijkstra(&sites.into_iter().collect_vec());
        Self { owner, grid }
    }

    pub(crate) fn owns_position(&self, position: Point2<Field>) -> bool {
        self.grid
            .ownership_at(position)
            .is_some_and(|ownership| ownership == Ownership::Robot(self.owner))
    }
}

fn search_voronoi_grid(field_dimensions: FieldDimensions, cells_x: usize) -> VoronoiGrid {
    let shape = crate::grid_dimensions(field_dimensions, cells_x);
    // Voronoi uses physical, square steps; sample ownership at the search cell centres.
    let resolution =
        (field_dimensions.length / shape.0 as f32).min(field_dimensions.width / shape.1 as f32);
    VoronoiGrid::new(
        point![
            -field_dimensions.length / 2.0,
            -field_dimensions.width / 2.0
        ],
        point![field_dimensions.length / 2.0, field_dimensions.width / 2.0],
        resolution,
    )
}

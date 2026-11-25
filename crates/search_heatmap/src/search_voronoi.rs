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

/// Retains the last partition while its geometry, owner and source cells are unchanged.
#[derive(Default)]
pub struct SearchVoronoiCache {
    entry: Option<CachedSelection>,
    #[cfg(test)]
    rebuilds: usize,
}

struct CachedSelection {
    geometry: (f32, f32, usize),
    source_cells: Vec<(Option<usize>, PlayerNumber)>,
    selection: SearchVoronoiSelection,
}

impl SearchVoronoiCache {
    pub fn get_or_update(
        &mut self,
        field_dimensions: FieldDimensions,
        cells_x: usize,
        owner: PlayerNumber,
        sites: impl IntoIterator<Item = (Pose2<Field>, PlayerNumber)>,
    ) -> &SearchVoronoiSelection {
        let sites = sites.into_iter().collect_vec();
        let geometry = (field_dimensions.length, field_dimensions.width, cells_x);
        if self.entry.as_ref().is_some_and(|entry| {
            entry.geometry != geometry
                || entry.selection.owner != owner
                || !entry
                    .source_cells
                    .iter()
                    .copied()
                    .eq(sites.iter().map(|(pose, number)| {
                        (
                            entry.selection.grid.point_to_index(pose.position()),
                            *number,
                        )
                    }))
        }) {
            self.entry = None;
        }
        let entry = self.entry.get_or_insert_with(|| {
            #[cfg(test)]
            {
                self.rebuilds += 1;
            }
            let selection = SearchVoronoiSelection::new(
                field_dimensions,
                cells_x,
                owner,
                sites.iter().copied(),
            );
            let source_cells = sites
                .iter()
                .map(|(pose, number)| (selection.grid.point_to_index(pose.position()), *number))
                .collect();
            CachedSelection {
                geometry,
                source_cells,
                selection,
            }
        });
        &entry.selection
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

#[cfg(test)]
mod tests;

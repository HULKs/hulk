use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use itertools::Itertools;
use linear_algebra::{Isometry2, Point2, Pose2, Vector2, point, vector};
use types::{field_dimensions::FieldDimensions, parameters::SearchSuggestorParameters};
use voronoi::{Ownership, VoronoiGrid};

use crate::{Heatmap, heatmap_dimensions, heatmap_tile_center};

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

    fn owns_index(&self, field_dimensions: FieldDimensions, index: (usize, usize)) -> bool {
        self.grid
            .ownership_at(heatmap_tile_center(field_dimensions, index))
            .is_some_and(|ownership| ownership == Ownership::Robot(self.owner))
    }
}

impl Heatmap {
    pub fn selected_position(&self, field_dimensions: FieldDimensions) -> Option<Point2<Field>> {
        self.last_maximum_heatmap_position
            .map(|index| heatmap_tile_center(field_dimensions, index))
    }

    fn get_maximum_position_matching(
        &self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        minimum_validity: f32,
        matches_selection: impl Fn((usize, usize)) -> bool,
    ) -> Option<(usize, usize)> {
        self.map
            .indexed_iter()
            .filter(|(index, value)| **value > minimum_validity && matches_selection(*index))
            .max_by(|(a_index, a), (b_index, b)| {
                selection_score(field_dimensions, parameters, *a_index, **a).total_cmp(
                    &selection_score(field_dimensions, parameters, *b_index, **b),
                )
            })
            .map(|(index, _)| index)
    }

    fn get_maximum_position_with_turn_preference_matching(
        &self,
        minimum_validity: f32,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        robot_position: Vector2<Field>,
        robot_heading: Vector2<Field>,
        matches_selection: impl Fn((usize, usize)) -> bool,
    ) -> Option<(usize, usize)> {
        let maximum_position = self.get_maximum_position_matching(
            field_dimensions,
            parameters,
            minimum_validity,
            &matches_selection,
        )?;
        let maximum_score = selection_score(
            field_dimensions,
            parameters,
            maximum_position,
            self.map[maximum_position],
        );
        self.map
            .indexed_iter()
            .filter(|(index, value)| {
                matches_selection(*index)
                    && **value > minimum_validity
                    && selection_score(field_dimensions, parameters, *index, **value)
                        >= maximum_score - parameters.turn_preference_priority_margin.max(0.0)
            })
            .max_by(|(a_index, a_value), (b_index, b_value)| {
                let a_turn_score =
                    turn_score(*a_index, field_dimensions, robot_position, robot_heading);
                let b_turn_score =
                    turn_score(*b_index, field_dimensions, robot_position, robot_heading);
                a_turn_score.total_cmp(&b_turn_score).then_with(|| {
                    selection_score(field_dimensions, parameters, *a_index, **a_value).total_cmp(
                        &selection_score(field_dimensions, parameters, *b_index, **b_value),
                    )
                })
            })
            .map(|(index, _)| index)
    }

    fn get_suggested_search_index(
        &self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        voronoi_selection: Option<&SearchVoronoiSelection>,
    ) -> Option<(usize, usize)> {
        let matches_selection = |index| {
            voronoi_selection
                .map(|selection| selection.owns_index(field_dimensions, index))
                .unwrap_or(true)
        };
        ground_to_field.map_or_else(
            || {
                self.get_maximum_position_matching(
                    field_dimensions,
                    parameters,
                    parameters.minimum_validity,
                    matches_selection,
                )
            },
            |ground_to_field| {
                let robot_position = ground_to_field.as_pose().position().coords();
                let robot_heading_angle = ground_to_field.orientation().angle();
                let robot_heading = vector![robot_heading_angle.cos(), robot_heading_angle.sin()];
                self.get_maximum_position_with_turn_preference_matching(
                    parameters.minimum_validity,
                    field_dimensions,
                    parameters,
                    robot_position,
                    robot_heading,
                    matches_selection,
                )
            },
        )
    }

    pub fn update_suggested_search_position_with_voronoi(
        &mut self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        voronoi_selection: &SearchVoronoiSelection,
    ) {
        self.update_suggested_search_position_with_optional_voronoi(
            field_dimensions,
            parameters,
            ground_to_field,
            Some(voronoi_selection),
        );
    }

    pub fn clear_suggested_search_position(&mut self) {
        self.last_maximum_heatmap_position = None;
        self.has_decided_for_heatmap_tile = false;
    }

    fn update_suggested_search_position_with_optional_voronoi(
        &mut self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        voronoi_selection: Option<&SearchVoronoiSelection>,
    ) {
        if !self.has_decided_for_heatmap_tile {
            let suggested_search_index = self.get_suggested_search_index(
                field_dimensions,
                parameters,
                ground_to_field,
                voronoi_selection,
            );
            if suggested_search_index.is_some() {
                self.has_decided_for_heatmap_tile = true;
            }
            self.last_maximum_heatmap_position = suggested_search_index;
        } else if let Some(last_maximum_heatmap_index) = self.last_maximum_heatmap_position {
            let current_tile_is_selectable = voronoi_selection
                .map(|selection| selection.owns_index(field_dimensions, last_maximum_heatmap_index))
                .unwrap_or(true);
            let global_max_value = self
                .get_maximum_position_matching(field_dimensions, parameters, 0.0, |index| {
                    voronoi_selection
                        .map(|selection| selection.owns_index(field_dimensions, index))
                        .unwrap_or(true)
                })
                .map_or(0.0, |idx| {
                    selection_score(field_dimensions, parameters, idx, self.map[idx])
                });
            let current_tile_value = selection_score(
                field_dimensions,
                parameters,
                last_maximum_heatmap_index,
                self.map[last_maximum_heatmap_index],
            );

            if !current_tile_is_selectable
                || self.map[last_maximum_heatmap_index] <= parameters.minimum_validity
                || current_tile_value < global_max_value * parameters.tile_switch_hysteresis
            {
                let suggested_search_index = self.get_suggested_search_index(
                    field_dimensions,
                    parameters,
                    ground_to_field,
                    voronoi_selection,
                );
                self.has_decided_for_heatmap_tile = suggested_search_index.is_some();
                self.last_maximum_heatmap_position = suggested_search_index;
            }
        }
    }
}

fn selection_score(
    field_dimensions: FieldDimensions,
    parameters: &SearchSuggestorParameters,
    index: (usize, usize),
    priority: f32,
) -> f32 {
    let center = heatmap_tile_center(field_dimensions, index);
    let distance_to_border = (field_dimensions.length / 2.0 - center.x().abs())
        .min(field_dimensions.width / 2.0 - center.y().abs())
        .max(0.0);
    let view_range = parameters.heatmap_decay_range.end;
    if view_range <= 0.0 {
        return priority;
    }
    let border_proximity = (1.0 - distance_to_border / view_range).clamp(0.0, 1.0);
    priority - parameters.border_view_penalty.max(0.0) * border_proximity
}

fn search_voronoi_grid(field_dimensions: FieldDimensions) -> VoronoiGrid {
    let (length, width) = heatmap_dimensions(field_dimensions);
    let grid_min = point![
        -field_dimensions.length / 2.0,
        -field_dimensions.width / 2.0
    ];
    let grid_max = point![grid_min.x() + length as f32, grid_min.y() + width as f32];
    VoronoiGrid::new(grid_min, grid_max, 1.0)
}

fn turn_score(
    (x, y): (usize, usize),
    field_dimensions: FieldDimensions,
    robot_position: Vector2<Field>,
    robot_heading: Vector2<Field>,
) -> f32 {
    let tile_center = heatmap_tile_center(field_dimensions, (x, y)).coords();
    let robot_to_tile = tile_center - robot_position;
    let robot_to_tile_norm = robot_to_tile.norm();
    let robot_heading_norm = robot_heading.norm();
    if robot_to_tile_norm <= f32::EPSILON || robot_heading_norm <= f32::EPSILON {
        return 1.0;
    }
    robot_heading.dot(&robot_to_tile) / (robot_heading_norm * robot_to_tile_norm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_penalty_prefers_interior_without_hiding_strong_border_evidence() {
        let field = FieldDimensions::SPL_2025;
        let mut heatmap = Heatmap::new(field);
        let border = (8, 5);
        let interior = (4, 2);
        heatmap.map[border] = 0.9;
        heatmap.map[interior] = 0.82;
        let mut parameters = SearchSuggestorParameters {
            minimum_validity: 0.4,
            border_view_penalty: 0.15,
            heatmap_decay_range: 0.0..3.0,
            ..Default::default()
        };

        assert_eq!(
            heatmap.get_suggested_search_index(field, &parameters, None, None),
            Some(interior)
        );

        parameters.border_view_penalty = 0.0;
        assert_eq!(
            heatmap.get_suggested_search_index(field, &parameters, None, None),
            Some(border)
        );

        parameters.border_view_penalty = 0.15;
        heatmap.map[border] = 1.0;
        assert_eq!(
            heatmap.get_suggested_search_index(field, &parameters, None, None),
            Some(border)
        );
    }

    #[test]
    fn border_penalty_changes_choice_with_turn_preference() {
        let field = FieldDimensions::SPL_2025;
        let mut heatmap = Heatmap::new(field);
        let border = (8, 5);
        let interior = (4, 2);
        heatmap.map[border] = 0.95;
        heatmap.map[interior] = 0.82;
        let mut parameters = SearchSuggestorParameters {
            minimum_validity: 0.4,
            border_view_penalty: 0.15,
            turn_preference_priority_margin: 0.1,
            heatmap_decay_range: 0.0..3.0,
            ..Default::default()
        };
        let pose =
            Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], -std::f32::consts::FRAC_PI_2);

        assert_eq!(
            heatmap.get_suggested_search_index(field, &parameters, Some(pose), None),
            Some(interior)
        );

        parameters.border_view_penalty = 0.0;
        assert_eq!(
            heatmap.get_suggested_search_index(field, &parameters, Some(pose), None),
            Some(border)
        );
    }
}

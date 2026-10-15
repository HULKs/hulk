use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2, Vector2, vector};
use types::{field_dimensions::FieldDimensions, parameters::SearchSuggestorParameters};

use crate::{
    Heatmap, heatmap_tile_center, heatmap_tile_contains, search_voronoi::SearchVoronoiSelection,
};

/// Limits search targets in field coordinates, where the own goal has negative x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchArea {
    WholeField,
    OwnHalf,
}

impl SearchArea {
    fn allows_index(self, field_dimensions: FieldDimensions, index: (usize, usize)) -> bool {
        self == Self::WholeField || heatmap_tile_center(field_dimensions, index).x() <= 0.0
    }

    fn allows_position(self, position: Point2<Field>) -> bool {
        self == Self::WholeField || position.x() <= 0.0
    }
}

impl Heatmap {
    fn tile_is_eligible(
        &self,
        field_dimensions: FieldDimensions,
        index: (usize, usize),
        minimum_validity: f32,
        voronoi_selection: &SearchVoronoiSelection,
        search_area: SearchArea,
    ) -> bool {
        self.map[index] > minimum_validity
            && search_area.allows_index(field_dimensions, index)
            && voronoi_selection.owns_index(field_dimensions, index)
    }

    pub fn selected_position(
        &self,
        field_dimensions: FieldDimensions,
        robot_position: Point2<Field>,
        search_area: SearchArea,
    ) -> Option<Point2<Field>> {
        self.last_maximum_heatmap_position
            .filter(|index| search_area.allows_index(field_dimensions, *index))
            .map(|index| {
                if search_area.allows_position(robot_position)
                    && heatmap_tile_contains(field_dimensions, index, robot_position)
                {
                    robot_position
                } else {
                    heatmap_tile_center(field_dimensions, index)
                }
            })
    }

    fn get_maximum_position(
        &self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        minimum_validity: f32,
        voronoi_selection: &SearchVoronoiSelection,
        search_area: SearchArea,
    ) -> Option<(usize, usize)> {
        self.map
            .indexed_iter()
            .filter(|(index, _)| {
                self.tile_is_eligible(
                    field_dimensions,
                    *index,
                    minimum_validity,
                    voronoi_selection,
                    search_area,
                )
            })
            .max_by(|(a_index, a), (b_index, b)| {
                selection_score(field_dimensions, parameters, *a_index, **a).total_cmp(
                    &selection_score(field_dimensions, parameters, *b_index, **b),
                )
            })
            .map(|(index, _)| index)
    }

    fn get_maximum_position_with_turn_preference(
        &self,
        minimum_validity: f32,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        ground_to_field: Isometry2<Ground, Field>,
        voronoi_selection: &SearchVoronoiSelection,
        search_area: SearchArea,
    ) -> Option<(usize, usize)> {
        let robot_position = ground_to_field.as_pose().position().coords();
        let robot_heading_angle = ground_to_field.orientation().angle();
        let robot_heading = vector![robot_heading_angle.cos(), robot_heading_angle.sin()];
        let turn_preference_priority_margin = if minimum_validity < parameters.minimum_validity {
            0.0
        } else {
            parameters.turn_preference_priority_margin
        };
        let maximum_position = self.get_maximum_position(
            field_dimensions,
            parameters,
            minimum_validity,
            voronoi_selection,
            search_area,
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
                self.tile_is_eligible(
                    field_dimensions,
                    *index,
                    minimum_validity,
                    voronoi_selection,
                    search_area,
                ) && selection_score(field_dimensions, parameters, *index, **value)
                    >= maximum_score - turn_preference_priority_margin.max(0.0)
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
        voronoi_selection: &SearchVoronoiSelection,
        minimum_validity: f32,
        search_area: SearchArea,
    ) -> Option<(usize, usize)> {
        ground_to_field.map_or_else(
            || {
                self.get_maximum_position(
                    field_dimensions,
                    parameters,
                    minimum_validity,
                    voronoi_selection,
                    search_area,
                )
            },
            |ground_to_field| {
                self.get_maximum_position_with_turn_preference(
                    minimum_validity,
                    field_dimensions,
                    parameters,
                    ground_to_field,
                    voronoi_selection,
                    search_area,
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
        minimum_validity: f32,
        search_area: SearchArea,
    ) {
        if let Some(last_maximum_heatmap_index) = self.last_maximum_heatmap_position {
            let current_tile_is_selectable = self.tile_is_eligible(
                field_dimensions,
                last_maximum_heatmap_index,
                minimum_validity,
                voronoi_selection,
                search_area,
            );
            let global_max_value = self
                .get_maximum_position(
                    field_dimensions,
                    parameters,
                    minimum_validity,
                    voronoi_selection,
                    search_area,
                )
                .map_or(0.0, |idx| {
                    selection_score(field_dimensions, parameters, idx, self.map[idx])
                });
            let current_tile_value = selection_score(
                field_dimensions,
                parameters,
                last_maximum_heatmap_index,
                self.map[last_maximum_heatmap_index],
            );
            let hysteresis_margin =
                (1.0 - parameters.tile_switch_hysteresis).max(0.0) * global_max_value.abs();

            if !current_tile_is_selectable
                || current_tile_value < global_max_value - hysteresis_margin
            {
                self.last_maximum_heatmap_position = self.get_suggested_search_index(
                    field_dimensions,
                    parameters,
                    ground_to_field,
                    voronoi_selection,
                    minimum_validity,
                    search_area,
                );
            }
        } else {
            self.last_maximum_heatmap_position = self.get_suggested_search_index(
                field_dimensions,
                parameters,
                ground_to_field,
                voronoi_selection,
                minimum_validity,
                search_area,
            );
        }
    }

    pub fn has_regular_search_target(
        &self,
        field_dimensions: FieldDimensions,
        parameters: &SearchSuggestorParameters,
        voronoi_selection: &SearchVoronoiSelection,
        search_area: SearchArea,
    ) -> bool {
        self.map.indexed_iter().any(|(index, _)| {
            self.tile_is_eligible(
                field_dimensions,
                index,
                parameters.minimum_validity,
                voronoi_selection,
                search_area,
            )
        })
    }

    pub fn clear_suggested_search_position(&mut self) {
        self.last_maximum_heatmap_position = None;
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
    let penalty_range = parameters.border_view_penalty_range;
    if penalty_range <= 0.0 {
        return priority;
    }
    let border_proximity = (1.0 - distance_to_border / penalty_range).clamp(0.0, 1.0);
    priority - parameters.border_view_penalty.max(0.0) * border_proximity
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
mod tests;

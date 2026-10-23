use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2};
use ros_z::time::Time;
use search_heatmap::{Heatmap, SearchArea};
use types::{
    field_dimensions::FieldDimensions, parameters::SearchSuggestorParameters, players::Players,
    time_wrapper::TimeWrapper, world_state::PlayerState,
};

use crate::team::{TeammateFreshness, search_voronoi_selection_from_teammates};

pub(crate) struct TargetSelection<'a> {
    pub heatmap: &'a mut Heatmap,
    pub field_dimensions: FieldDimensions,
    pub parameters: &'a SearchSuggestorParameters,
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    pub player_number: Option<PlayerNumber>,
    pub goalkeeper_player_number: Option<PlayerNumber>,
    pub last_teammate_states: &'a Players<Option<TimeWrapper<PlayerState>>>,
    pub teammate_freshness: TeammateFreshness,
    pub last_ball_position: Option<(Point2<Field>, Time)>,
    pub current_ball_position: Option<Point2<Field>>,
    pub is_playing: bool,
    pub no_regular_target_since: &'a mut Option<Time>,
    pub now: Time,
}

pub(crate) fn select_search_position(input: TargetSelection<'_>) -> Option<Point2<Field>> {
    let TargetSelection {
        heatmap,
        field_dimensions,
        parameters,
        ground_to_field,
        player_number,
        goalkeeper_player_number,
        last_teammate_states,
        teammate_freshness,
        last_ball_position,
        current_ball_position,
        is_playing,
        no_regular_target_since,
        now,
    } = input;
    let own_voronoi_site = ground_to_field.zip(player_number);
    let search_area = own_voronoi_site.as_ref().and_then(|(_, player_number)| {
        goalkeeper_player_number.map(|goalkeeper_player_number| {
            search_area_for_player(*player_number, goalkeeper_player_number)
        })
    });
    if last_ball_position.is_none() {
        *no_regular_target_since = None;
        heatmap.clear_suggested_search_position();
    } else if let Some((voronoi_selection, search_area)) = search_voronoi_selection_from_teammates(
        field_dimensions,
        parameters.cells_along_field_length,
        own_voronoi_site,
        goalkeeper_player_number,
        last_teammate_states,
        teammate_freshness,
    )
    .zip(search_area)
    {
        let has_regular_target = heatmap.has_regular_search_target(
            field_dimensions,
            parameters,
            &voronoi_selection,
            search_area,
        );
        let minimum_search_priority = active_minimum_search_priority(
            no_regular_target_since,
            now,
            current_ball_position.is_none() && is_playing,
            has_regular_target,
            parameters,
        );
        heatmap.update_suggested_search_position_with_voronoi(
            field_dimensions,
            parameters,
            ground_to_field,
            &voronoi_selection,
            minimum_search_priority,
            search_area,
        );
    } else {
        *no_regular_target_since = None;
        heatmap.clear_suggested_search_position();
    }

    ground_to_field
        .zip(search_area)
        .and_then(|(ground_to_field, search_area)| {
            heatmap.selected_position(field_dimensions, ground_to_field.translation(), search_area)
        })
}

pub(crate) fn search_area_for_player(
    player_number: PlayerNumber,
    goalkeeper_player_number: PlayerNumber,
) -> SearchArea {
    if player_number == goalkeeper_player_number {
        SearchArea::OwnHalf
    } else {
        SearchArea::WholeField
    }
}

pub(crate) fn ground_to_field_is_finite(ground_to_field: Isometry2<Ground, Field>) -> bool {
    ground_to_field.translation().x().is_finite()
        && ground_to_field.translation().y().is_finite()
        && ground_to_field.orientation().angle().is_finite()
}

pub(crate) fn active_minimum_search_priority(
    no_regular_target_since: &mut Option<Time>,
    now: Time,
    can_use_fallback: bool,
    has_regular_target: bool,
    parameters: &SearchSuggestorParameters,
) -> f32 {
    if !can_use_fallback || has_regular_target {
        *no_regular_target_since = None;
        return parameters.minimum_search_priority;
    }

    let since = no_regular_target_since.get_or_insert(now);
    if now < *since {
        *since = now;
    }
    if now.duration_since(*since) >= parameters.exhausted_search_delay {
        parameters.fallback_minimum_search_priority
    } else {
        parameters.minimum_search_priority
    }
}

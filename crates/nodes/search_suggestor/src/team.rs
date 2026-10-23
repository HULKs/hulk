use std::time::Duration;

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::Isometry2;
use ros_z::time::Time;
use search_heatmap::{Heatmap, SearchVoronoiCache, SearchVoronoiSelection};
use types::{
    field_dimensions::FieldDimensions, parameters::SearchSuggestorParameters, players::Players,
    time_wrapper::TimeWrapper, world_state::PlayerState,
};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub(crate) struct TeammateFreshness {
    pub now: Time,
    pub window: Option<Duration>,
}

impl TeammateFreshness {
    fn contains(self, state_time: Time) -> bool {
        self.now >= state_time
            && self
                .window
                .is_some_and(|window| self.now.duration_since(state_time) <= window)
    }
}

pub(crate) fn search_voronoi_selection_from_teammates<'a>(
    cache: &'a mut SearchVoronoiCache,
    field_dimensions: FieldDimensions,
    cells_x: usize,
    own_voronoi_site: Option<(Isometry2<Ground, Field>, PlayerNumber)>,
    goalkeeper_player_number: Option<PlayerNumber>,
    last_teammate_states: &Players<Option<TimeWrapper<PlayerState>>>,
    teammate_freshness: TeammateFreshness,
) -> Option<&'a SearchVoronoiSelection> {
    let (ground_to_field, own_player_number) = own_voronoi_site?;
    let goalkeeper_player_number = goalkeeper_player_number?;
    let mut sites = vec![(ground_to_field.as_pose(), own_player_number)];

    for (player_number, teammate_state) in last_teammate_states.iter() {
        let Some(teammate_state) = teammate_state else {
            continue;
        };
        // Field players divide the field without the goalkeeper, matching the
        // behavior's calculate_voronoi_grid_without_goalkeeper. The goalkeeper
        // searches its own half independently of the field players' partition.
        if player_number == own_player_number
            || player_number == goalkeeper_player_number
            || own_player_number == goalkeeper_player_number
        {
            continue;
        }
        if teammate_freshness.contains(teammate_state.time) {
            sites.push((teammate_state.inner.pose, player_number));
        }
    }

    Some(cache.get_or_update(field_dimensions, cells_x, own_player_number, sites))
}

pub(crate) fn update_from_player_states(
    heatmap: &mut Heatmap,
    field_dimensions: FieldDimensions,
    player_states: &Players<Option<TimeWrapper<PlayerState>>>,
    last_teammate_states: &mut Players<Option<TimeWrapper<PlayerState>>>,
    parameters: &SearchSuggestorParameters,
    teammate_freshness: TeammateFreshness,
) {
    for (player_number, current) in player_states.iter() {
        let Some(current) = current.as_ref().filter(|state| state.inner.is_finite()) else {
            last_teammate_states[player_number] = None;
            continue;
        };
        let previous = last_teammate_states[player_number].as_ref();
        if previous.is_some_and(|previous| current.time <= previous.time) {
            continue;
        }

        let is_fresh = teammate_freshness.contains(current.time);
        if is_fresh && let Some(ball) = current.inner.ball_position {
            heatmap.update_with_team_ball(field_dimensions, ball.position, parameters);
        }
        if is_fresh
            && let Some(freshness_window) = teammate_freshness.window
            && let Some(previous) = previous
            && let elapsed_since_previous_message = current.time.duration_since(previous.time)
            && elapsed_since_previous_message <= freshness_window
        {
            heatmap.decay_tiles_from_teammate_motion(
                field_dimensions,
                &previous.inner,
                &current.inner,
                elapsed_since_previous_message,
                parameters,
            );
        }
        last_teammate_states[player_number] = Some(current.clone());
    }
}

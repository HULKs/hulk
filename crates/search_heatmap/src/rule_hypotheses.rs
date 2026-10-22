use coordinate_systems::Field;
use hsl_network_messages::{SubState, Team};
use linear_algebra::Point2;
use std::time::Duration;
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
};

use crate::Heatmap;

impl Heatmap {
    pub fn update_with_rule_ball(
        &mut self,
        filtered_game_controller_state: &FilteredGameControllerState,
        field_dimensions: FieldDimensions,
        primary_state: PrimaryState,
        parameters: &SearchSuggestorParameters,
        elapsed: Duration,
    ) {
        if self.regenerate_restart_hypotheses(
            filtered_game_controller_state,
            field_dimensions,
            elapsed.as_secs_f32() / parameters.rule_priority_rise_time.as_secs_f32(),
        ) {
            return;
        }

        for rule_ball_hypothesis in get_rule_hypotheses(
            primary_state,
            filtered_game_controller_state,
            field_dimensions,
        ) {
            let heatmap_point = self.field_to_heatmap(field_dimensions, rule_ball_hypothesis);
            self.map[heatmap_point] +=
                elapsed.as_secs_f32() / parameters.rule_priority_rise_time.as_secs_f32();
        }
    }

    pub fn regenerate_restart_hypotheses(
        &mut self,
        filtered_game_controller_state: &FilteredGameControllerState,
        field_dimensions: FieldDimensions,
        increment: f32,
    ) -> bool {
        let Some(restart_indices) =
            self.restart_hypothesis_indices(filtered_game_controller_state, field_dimensions)
        else {
            return false;
        };
        self.map.indexed_iter_mut().for_each(|(index, value)| {
            if !restart_indices.contains(&index) {
                *value = 0.0;
            }
        });
        for index in restart_indices {
            self.map[index] += increment;
        }
        true
    }

    fn restart_hypothesis_indices(
        &self,
        filtered_game_controller_state: &FilteredGameControllerState,
        field_dimensions: FieldDimensions,
    ) -> Option<Vec<(usize, usize)>> {
        let mut indices: Vec<_> = match filtered_game_controller_state.sub_state {
            Some(SubState::CornerKick) => corner_kick_hypotheses(
                filtered_game_controller_state.kicking_team,
                field_dimensions,
            )
            .into_iter()
            .map(|hypothesis| self.field_to_heatmap(field_dimensions, hypothesis))
            .collect(),
            Some(SubState::GoalKick) => goal_kick_hypotheses(
                filtered_game_controller_state.kicking_team,
                field_dimensions,
            )
            .into_iter()
            .map(|hypothesis| self.field_to_heatmap(field_dimensions, hypothesis))
            .collect(),
            Some(SubState::ThrowIn) => {
                let last_y = self.map.dim().1 - 1;
                (0..self.map.dim().0)
                    .flat_map(|x| [(x, 0), (x, last_y)])
                    .collect()
            }
            _ => return None,
        };

        indices.sort_unstable();
        indices.dedup();
        Some(indices)
    }
}

fn get_rule_hypotheses(
    primary_state: PrimaryState,
    filtered_game_controller_state: &FilteredGameControllerState,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    match (primary_state, filtered_game_controller_state.sub_state) {
        (PrimaryState::Ready, Some(SubState::PenaltyKick)) => {
            let kicking_team_half = kicking_team_half(filtered_game_controller_state.kicking_team)
                .unwrap_or(Half::Own)
                .mirror();
            vec![field_dimensions.penalty_spot(kicking_team_half)]
        }
        (PrimaryState::Ready, None) => vec![field_dimensions.center()],
        (PrimaryState::Playing, Some(SubState::CornerKick)) => corner_kick_hypotheses(
            filtered_game_controller_state.kicking_team,
            field_dimensions,
        ),
        (PrimaryState::Playing, Some(SubState::GoalKick)) => goal_kick_hypotheses(
            filtered_game_controller_state.kicking_team,
            field_dimensions,
        ),
        (_, _) => Vec::new(),
    }
}

fn corner_kick_hypotheses(
    kicking_team: Option<Team>,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    if let Some(kicking_team_half) = kicking_team_half(kicking_team) {
        let kicking_team_half = kicking_team_half.mirror();
        vec![
            field_dimensions.corner(kicking_team_half, Side::Left),
            field_dimensions.corner(kicking_team_half, Side::Right),
        ]
    } else {
        vec![
            field_dimensions.corner(Half::Own, Side::Left),
            field_dimensions.corner(Half::Opponent, Side::Left),
            field_dimensions.corner(Half::Own, Side::Right),
            field_dimensions.corner(Half::Opponent, Side::Right),
        ]
    }
}

fn goal_kick_hypotheses(
    kicking_team: Option<Team>,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    if let Some(kicking_team_half) = kicking_team_half(kicking_team) {
        vec![
            field_dimensions.goal_box_corner(kicking_team_half, Side::Left),
            field_dimensions.goal_box_corner(kicking_team_half, Side::Right),
        ]
    } else {
        vec![
            field_dimensions.goal_box_corner(Half::Own, Side::Left),
            field_dimensions.goal_box_corner(Half::Opponent, Side::Left),
            field_dimensions.goal_box_corner(Half::Own, Side::Right),
            field_dimensions.goal_box_corner(Half::Opponent, Side::Right),
        ]
    }
}

fn kicking_team_half(kicking_team: Option<Team>) -> Option<Half> {
    match kicking_team {
        Some(Team::Opponent) => Some(Half::Opponent),
        Some(Team::Hulks) => Some(Half::Own),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::{
        field_dimensions::GlobalFieldSide, filtered_game_state::FilteredGameState, players::Players,
    };

    #[test]
    fn normal_and_restart_rule_growth_use_elapsed_time() {
        let field = FieldDimensions::SPL_2025;
        let parameters = SearchSuggestorParameters::default();
        for sub_state in [None, Some(SubState::CornerKick)] {
            let state = FilteredGameControllerState {
                game_state: FilteredGameState::Ready,
                opponent_game_state: FilteredGameState::Ready,
                half: hsl_network_messages::Half::First,
                remaining_time_in_half: Duration::ZERO,
                game_phase: hsl_network_messages::GamePhase::Normal,
                kicking_team: Some(Team::Hulks),
                penalties: Players::new(None),
                remaining_number_of_messages: 100,
                sub_state,
                global_field_side: GlobalFieldSide::Home,
                new_own_penalties_last_cycle: Default::default(),
                new_opponent_penalties_last_cycle: Default::default(),
            };
            for steps in [1, 50] {
                let mut map = Heatmap::new(field, 28);
                for _ in 0..steps {
                    map.update_with_rule_ball(
                        &state,
                        field,
                        PrimaryState::Ready,
                        &parameters,
                        Duration::from_secs_f32(1.0 / steps as f32),
                    );
                }
                let maximum = map.map.iter().copied().fold(0.0, f32::max);
                assert!((maximum - 1.0 / 16.0).abs() < 1e-6);
            }
        }
    }
}

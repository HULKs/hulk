use coordinate_systems::Field;
use hsl_network_messages::{SubState, Team};
use linear_algebra::Point2;
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
    ) {
        if self.regenerate_restart_hypotheses(
            filtered_game_controller_state,
            field_dimensions,
            parameters.rule_ball_weight_increment,
        ) {
            return;
        }

        for rule_ball_hypothesis in get_rule_hypotheses(
            primary_state,
            filtered_game_controller_state,
            field_dimensions,
        ) {
            let heatmap_point = self.field_to_heatmap(field_dimensions, rule_ball_hypothesis);
            self.map[heatmap_point] += parameters.rule_ball_weight_increment;
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

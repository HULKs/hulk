use std::cmp::Ordering;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use types::game_controller_state::GameControllerState;
#[derive(Deserialize, Serialize)]
pub struct ActivePlayerFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,

    jersey_number: Parameter<usize, "jersey_number">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_in_position_index: MainOutput<usize>,
    pub replacement_keeper_priority: MainOutput<Option<usize>>,
    pub striker_priority: MainOutput<Option<usize>>,
}

impl ActivePlayerFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let penalties = context
            .game_controller_state
            .map(|game_controller_state| &game_controller_state.penalties);

        let mut walk_in_position_index = if let Some(list) = penalties {
            list.inner()
                .keys()
                .sorted()
                .position(|&key| key == *context.jersey_number)
        } else {
            None
        }
        .unwrap_or(7);

        let goal_keeper_jersey_number =
            context.game_controller_state.map(|game_controller_state| {
                game_controller_state.hulks_team.goal_keeper_jersey_number
            });
        if let Some(goal_keeper_number) = goal_keeper_jersey_number {
            match goal_keeper_number.cmp(context.jersey_number) {
                Ordering::Equal => {
                    walk_in_position_index = 0;
                }
                Ordering::Greater => {
                    walk_in_position_index += 1;
                }
                _ => {}
            }
        }

        let available_field_players =
            if let Some(game_controller_state) = context.game_controller_state {
                game_controller_state
                    .penalties
                    .inner()
                    .iter()
                    .filter(|(&jersey_number, penalty)| {
                        penalty.is_none()
                            && jersey_number
                                != game_controller_state.hulks_team.goal_keeper_jersey_number
                    })
                    .map(|(&jersey_number, _)| jersey_number)
                    .sorted()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
        let mut replacement_keeper_priority = available_field_players
            .iter()
            .position(|&jersey_number| jersey_number == *context.jersey_number);
        if let (Some(game_controller_state), Some(keeper_priority)) =
            (context.game_controller_state, replacement_keeper_priority)
        {
            match game_controller_state
                .penalties
                .inner()
                .get(&game_controller_state.hulks_team.goal_keeper_jersey_number)
            {
                Some(_penalty) => {}
                None => replacement_keeper_priority = Some(keeper_priority + 1),
            }
        }
        let striker_priority = available_field_players
            .iter()
            .rev()
            .position(|&jersey_number| jersey_number == *context.jersey_number);

        Ok(MainOutputs {
            walk_in_position_index: walk_in_position_index.into(),
            replacement_keeper_priority: replacement_keeper_priority.into(),
            striker_priority: striker_priority.into(),
        })
    }
}
// fn filter_penalties(
//     penalties: &HashMap<usize, Option<Penalty>>,
// ) -> HashMap<usize, Option<Penalty>> {
//     penalties
//         .iter()
//         .filter_map(|(index, penalty_option)| match penalty_option {
//             Some(Penalty::Substitute { .. }) => None,
//             _ => Some((*index, *penalty_option)),
//         })
//         .collect()
// }

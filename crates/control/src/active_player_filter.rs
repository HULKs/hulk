use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use types::filtered_game_controller_state::FilteredGameControllerState;

#[derive(Deserialize, Serialize)]
pub struct ActivePlayerFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,

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
            .filtered_game_controller_state
            .map(|game_controller_state| &game_controller_state.penalties);
        let mut walk_in_position_index = if let Some(list) = penalties {
            list.keys()
                .sorted()
                .position(|&key| key == *context.jersey_number)
        } else {
            None
        }
        .unwrap_or(7);

        let goalkeeper_jersey_number = context
            .filtered_game_controller_state
            .map(|game_controller_state| game_controller_state.goal_keeper_number);
        if let Some(goalkeeper_number) = goalkeeper_jersey_number {
            if goalkeeper_number == *context.jersey_number {
                walk_in_position_index = 0;
            } else if goalkeeper_number > *context.jersey_number {
                walk_in_position_index += 1;
            }
        }

        let available_field_players = if let Some(game_controller_state) =
            context.filtered_game_controller_state
        {
            game_controller_state
                .penalties
                .iter()
                .filter(|(&jersey_number, penalty)| {
                    penalty.is_none() && jersey_number != game_controller_state.goal_keeper_number
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
        if let (Some(game_controller_state), Some(keeper_priority)) = (
            context.filtered_game_controller_state,
            replacement_keeper_priority,
        ) {
            match game_controller_state
                .penalties
                .get(&game_controller_state.goal_keeper_number)
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

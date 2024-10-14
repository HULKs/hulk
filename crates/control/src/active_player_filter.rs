use std::cmp::Ordering;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use spl_network_messages::Penalty;
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
        let mut walk_in_position_index = 0;
        let mut replacement_keeper_priority = Some(0);
        let mut striker_priority = Some(0);
        if let Some(game_controller_state) = context.game_controller_state {
            let penalties = &game_controller_state.penalties;
            walk_in_position_index = penalties
                .inner()
                .iter()
                .filter(|&(_, penalty)| !matches!(penalty, Some(Penalty::Substitute { .. })))
                .map(|(&key, _)| key)
                .sorted()
                .position(|key| key == *context.jersey_number)
                .unwrap_or(7);

            match game_controller_state
                .hulks_team
                .goal_keeper_jersey_number
                .cmp(context.jersey_number)
            {
                Ordering::Equal => {
                    walk_in_position_index = 0;
                }
                Ordering::Greater => {
                    walk_in_position_index += 1;
                }
                _ => {}
            }
            let available_field_players = penalties
                .inner()
                .iter()
                .filter(|(&jersey_number, penalty)| {
                    penalty.is_none()
                        && jersey_number
                            != game_controller_state.hulks_team.goal_keeper_jersey_number
                })
                .map(|(&jersey_number, _)| jersey_number)
                .sorted()
                .collect::<Vec<_>>();
            replacement_keeper_priority = available_field_players
                .iter()
                .position(|&jersey_number| jersey_number == *context.jersey_number);
            if let (Some(game_controller_state), Some(keeper_priority)) =
                (context.game_controller_state, replacement_keeper_priority)
            {
                match game_controller_state
                    .penalties
                    .inner()
                    .get(&game_controller_state.hulks_team.goal_keeper_jersey_number)
                    .expect("ffailed to find goal keeper penalty")
                {
                    Some(_penalty) => {}
                    None => replacement_keeper_priority = Some(keeper_priority + 1),
                }
            }
            striker_priority = available_field_players
                .iter()
                .rev()
                .position(|&jersey_number| jersey_number == *context.jersey_number);
        }

        Ok(MainOutputs {
            walk_in_position_index: walk_in_position_index.into(),
            replacement_keeper_priority: replacement_keeper_priority.into(),
            striker_priority: striker_priority.into(),
        })
    }
}

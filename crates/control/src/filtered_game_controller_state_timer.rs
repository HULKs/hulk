use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime, filtered_game_controller_state::FilteredGameControllerState,
    last_filtered_game_controller_state_change::LastFilteredGameControllerStateChanges,
};

#[derive(Deserialize, Serialize)]
pub struct FilteredGameControllerStateTimer {
    last_filtered_game_controller_state: FilteredGameControllerState,
    filtered_game_controller_state_changes: LastFilteredGameControllerStateChanges,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_game_controller_state:
        RequiredInput<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub last_filtered_game_controller_state_changes:
        MainOutput<Option<LastFilteredGameControllerStateChanges>>,
}

impl FilteredGameControllerStateTimer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_filtered_game_controller_state: FilteredGameControllerState::default(),
            filtered_game_controller_state_changes: LastFilteredGameControllerStateChanges::default(
            ),
        })
    }
    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        fn update_state_change<T: PartialEq>(
            current_state: T,
            last_state: T,
            change_time: &mut SystemTime,
            cycle_start_time: SystemTime,
        ) {
            if current_state != last_state {
                *change_time = cycle_start_time;
            }
        }

        update_state_change(
            context.filtered_game_controller_state.game_state,
            self.last_filtered_game_controller_state.game_state,
            &mut self.filtered_game_controller_state_changes.game_state,
            cycle_start_time,
        );
        update_state_change(
            context.filtered_game_controller_state.opponent_game_state,
            self.last_filtered_game_controller_state.opponent_game_state,
            &mut self
                .filtered_game_controller_state_changes
                .opponent_game_state,
            cycle_start_time,
        );
        update_state_change(
            context.filtered_game_controller_state.game_phase,
            self.last_filtered_game_controller_state.game_phase,
            &mut self.filtered_game_controller_state_changes.game_phase,
            cycle_start_time,
        );
        update_state_change(
            context.filtered_game_controller_state.kicking_team,
            self.last_filtered_game_controller_state.kicking_team,
            &mut self.filtered_game_controller_state_changes.kicking_team,
            cycle_start_time,
        );

        if context.filtered_game_controller_state.penalties
            != self.last_filtered_game_controller_state.penalties
        {
            for (jersey_number, penalty) in context.filtered_game_controller_state.penalties.inner()
            {
                match (
                    penalty,
                    self.filtered_game_controller_state_changes
                        .penalties
                        .contains_key(jersey_number),
                ) {
                    (None, false) => {
                        self.filtered_game_controller_state_changes
                            .penalties
                            .insert(*jersey_number, None);
                    }
                    (Some(_), _) => {
                        self.filtered_game_controller_state_changes
                            .penalties
                            .insert(*jersey_number, Some(cycle_start_time));
                    }
                    _ => (),
                }
            }
            let players_to_remove: Vec<usize> = self
                .filtered_game_controller_state_changes
                .penalties
                .keys()
                .filter(|&&player_number| {
                    !&context
                        .filtered_game_controller_state
                        .penalties
                        .inner()
                        .contains_key(&player_number)
                })
                .cloned()
                .collect();

            for player_number in players_to_remove {
                self.last_filtered_game_controller_state
                    .penalties
                    .remove(&player_number);
            }
        }

        if context.filtered_game_controller_state.sub_state
            != self.last_filtered_game_controller_state.sub_state
        {
            self.filtered_game_controller_state_changes.sub_state = Some(cycle_start_time);
        }

        self.last_filtered_game_controller_state = context.filtered_game_controller_state.clone();

        Ok(MainOutputs {
            last_filtered_game_controller_state_changes: Some(
                self.filtered_game_controller_state_changes.clone(),
            )
            .into(),
        })
    }
}

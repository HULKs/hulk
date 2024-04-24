use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use spl_network_messages::Penalty;
use types::{
    cycle_time::CycleTime, filtered_game_controller_state::FilteredGameControllerState,
    last_filtered_game_controller_state_change::LastFilteredGameControllerStateChanges,
    players::Players,
};

#[derive(PartialEq)]
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
        RequiredInput<Option<FilteredGameControllerState>, "filtered_game_controller_state">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub last_filterd_game_controller_sate_changes:
        MainOutput<Option<LastFilteredGameControllerStateChanges>>,
}

impl FilteredGameControllerStateTimer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_filtered_game_controller_state: FilteredGameControllerState::default(),
            filtered_game_controller_state_changes: LastFilteredGameControllerStateChanges {
                game_state: CycleTime::default(),
                opponent_game_state: CycleTime::default(),
                game_phase: CycleTime::default(),
                kicking_team: CycleTime::default(),
                penalties: Players::default(),
                sub_state: None,
            },
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        if context.filtered_game_controller_state.game_state
            != self.last_filtered_game_controller_state.game_state
        {
            self.filtered_game_controller_state_changes
                .game_state
                .start_time = cycle_start_time;
            self.last_filtered_game_controller_state.game_state =
                context.filtered_game_controller_state.game_state;
        }

        if context.filtered_game_controller_state.opponent_game_state
            != self.last_filtered_game_controller_state.opponent_game_state
        {
            self.filtered_game_controller_state_changes
                .opponent_game_state
                .start_time = cycle_start_time;
            self.last_filtered_game_controller_state.opponent_game_state =
                context.filtered_game_controller_state.opponent_game_state;
        }

        if context.filtered_game_controller_state.game_phase
            != self.last_filtered_game_controller_state.game_phase
        {
            self.filtered_game_controller_state_changes
                .game_phase
                .start_time = cycle_start_time;
            self.last_filtered_game_controller_state.game_phase =
                context.filtered_game_controller_state.game_phase;
        }

        if context.filtered_game_controller_state.kicking_team
            != self.last_filtered_game_controller_state.kicking_team
        {
            self.filtered_game_controller_state_changes
                .kicking_team
                .start_time = cycle_start_time;
            self.last_filtered_game_controller_state.kicking_team =
                context.filtered_game_controller_state.kicking_team;
        }

        if context.filtered_game_controller_state.penalties
            != self.last_filtered_game_controller_state.penalties
        {
            fn update_player_penalty(
                last_penalty: Option<Penalty>,
                new_penalty: Option<Penalty>,
            ) -> Option<CycleTime> {
                if last_penalty != new_penalty {
                    Some(CycleTime::default())
                } else {
                    None
                }
            }

            self.filtered_game_controller_state_changes.penalties = Players {
                one: update_player_penalty(
                    self.last_filtered_game_controller_state.penalties.one,
                    context.filtered_game_controller_state.penalties.one,
                ),
                two: update_player_penalty(
                    self.last_filtered_game_controller_state.penalties.two,
                    context.filtered_game_controller_state.penalties.two,
                ),
                three: update_player_penalty(
                    self.last_filtered_game_controller_state.penalties.three,
                    context.filtered_game_controller_state.penalties.three,
                ),
                four: update_player_penalty(
                    self.last_filtered_game_controller_state.penalties.four,
                    context.filtered_game_controller_state.penalties.four,
                ),
                five: update_player_penalty(
                    self.last_filtered_game_controller_state.penalties.five,
                    context.filtered_game_controller_state.penalties.five,
                ),
                six: update_player_penalty(
                    self.last_filtered_game_controller_state.penalties.six,
                    context.filtered_game_controller_state.penalties.six,
                ),
                seven: update_player_penalty(
                    self.last_filtered_game_controller_state.penalties.seven,
                    context.filtered_game_controller_state.penalties.seven,
                ),
            };
        }

        if context.filtered_game_controller_state.sub_state
            != self.last_filtered_game_controller_state.sub_state
        {
            self.filtered_game_controller_state_changes.sub_state = Some(CycleTime::default());
        }

        Ok(MainOutputs {
            last_filterd_game_controller_sate_changes: Some(
                self.filtered_game_controller_state_changes,
            )
            .into(),
        })
    }
}

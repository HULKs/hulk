use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    cycle_time::CycleTime, filtered_game_controller_state::FilteredGameControllerState,
    players::Players,
};

#[derive(PartialEq)]
pub struct FilterdGameControllerStateTimer {
    last_filtered_game_controller_state: FilteredGameControllerState,
    filtered_game_controller_state_changes: LastFilterdGameControllerStateChanges,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    filterd_game_controller_state:
        RequiredInput<Option<FilteredGameControllerState>, "filterd_game_controller_state">,
}

#[derive(Clone, Copy, PartialEq)]
pub struct LastFilterdGameControllerStateChanges {
    pub game_state: SystemTime,
    pub opponent_game_state: SystemTime,
    pub game_phase: SystemTime,
    pub kicking_team: SystemTime,
    pub penalties: Players<Option<SystemTime>>,
    pub sub_state: Option<SystemTime>,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub last_filterd_game_controller_sate_changes:
        MainOutput<Option<LastFilterdGameControllerStateChanges>>,
}

impl FilterdGameControllerStateTimer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_filtered_game_controller_state: FilteredGameControllerState::default(),
            filtered_game_controller_state_changes: LastFilterdGameControllerStateChanges {
                game_state: SystemTime::now(),
                opponent_game_state: SystemTime::now(),
                game_phase: SystemTime::now(),
                kicking_team: SystemTime::now(),
                penalties: Players {
                    one: Some(SystemTime::now()),
                    two: Some(SystemTime::now()),
                    three: Some(SystemTime::now()),
                    four: Some(SystemTime::now()),
                    five: Some(SystemTime::now()),
                    six: Some(SystemTime::now()),
                    seven: Some(SystemTime::now()),
                },
                sub_state: None,
            },
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        if context.filterd_game_controller_state.game_state
            != self.last_filtered_game_controller_state.game_state
        {
            self.filtered_game_controller_state_changes.game_state = SystemTime::now();
        }

        if context.filterd_game_controller_state.opponent_game_state
            != self.last_filtered_game_controller_state.opponent_game_state
        {
            self.filtered_game_controller_state_changes
                .opponent_game_state = SystemTime::now();
        }

        if context.filterd_game_controller_state.game_phase
            != self.last_filtered_game_controller_state.game_phase
        {
            self.filtered_game_controller_state_changes.game_phase = SystemTime::now();
        }

        if context.filterd_game_controller_state.kicking_team
            != self.last_filtered_game_controller_state.kicking_team
        {
            self.filtered_game_controller_state_changes.kicking_team = SystemTime::now();
        }

        if context.filterd_game_controller_state.penalties
            != self.last_filtered_game_controller_state.penalties
        {
            self.filtered_game_controller_state_changes.penalties = Players {
                one: Some(SystemTime::now()),
                two: Some(SystemTime::now()),
                three: Some(SystemTime::now()),
                four: Some(SystemTime::now()),
                five: Some(SystemTime::now()),
                six: Some(SystemTime::now()),
                seven: Some(SystemTime::now()),
            };
        }

        if context.filterd_game_controller_state.sub_state
            != self.last_filtered_game_controller_state.sub_state
        {
            self.filtered_game_controller_state_changes.sub_state = Some(SystemTime::now());
        }

        Ok(MainOutputs {
            last_filterd_game_controller_sate_changes: Some(
                self.filtered_game_controller_state_changes,
            )
            .into(),
        })
    }
}

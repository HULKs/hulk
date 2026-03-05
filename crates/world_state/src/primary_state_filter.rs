use std::collections::HashSet;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{RecordingInterface, SpeakerInterface};
use hsl_network_messages::PlayerNumber;
use serde::{Deserialize, Serialize};
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState, primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct PrimaryStateFilter {
    last_primary_state: PrimaryState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    should_exit_safe_mode: Input<bool, "should_exit_safe_mode">,

    injected_primary_state: Parameter<Option<PrimaryState>, "injected_primary_state?">,
    player_number: Parameter<PlayerNumber, "player_number">,
    recorded_primary_states: Parameter<HashSet<PrimaryState>, "recorded_primary_states">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub primary_state: MainOutput<PrimaryState>,
}

impl PrimaryStateFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_primary_state: PrimaryState::Safe,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl RecordingInterface + SpeakerInterface>,
    ) -> Result<MainOutputs> {
        if let Some(injected_primary_state) = context.injected_primary_state {
            self.last_primary_state = *injected_primary_state;
            return Ok(MainOutputs {
                primary_state: (*injected_primary_state).into(),
            });
        }

        let (is_penalized, game_state) = match context.filtered_game_controller_state {
            Some(game_controller_state) => (
                game_controller_state.penalties[*context.player_number].is_some(),
                game_controller_state.game_state,
            ),
            None => (false, FilteredGameState::default()),
        };

        let next_primary_state = match (
            self.last_primary_state,
            context.should_exit_safe_mode,
            is_penalized,
            game_state,
        ) {
            (PrimaryState::Safe, true, _, _) => PrimaryState::Initial,
            (PrimaryState::Initial, _, false, FilteredGameState::Ready) => PrimaryState::Ready,
            (PrimaryState::Ready, _, false, FilteredGameState::Set) => PrimaryState::Set,
            (PrimaryState::Set, _, false, FilteredGameState::Playing { .. }) => {
                PrimaryState::Playing
            }
            (state, _, _, FilteredGameState::Finished) if !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Finished
            }
            (PrimaryState::Playing, _, false, FilteredGameState::Ready) => PrimaryState::Ready,
            (state, _, _, FilteredGameState::Stop) if !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Stop
            }
            (PrimaryState::Stop, _, is_penalized, game_state) => {
                self.game_state_to_primary_state(game_state, is_penalized)
            }
            (state, _, true, _) if !matches!(state, PrimaryState::Safe) => PrimaryState::Penalized,
            (PrimaryState::Penalized, _, false, game_state) => {
                self.game_state_to_primary_state(game_state, false)
            }
            _ => self.last_primary_state,
        };

        context.hardware_interface.set_whether_to_record(
            context
                .recorded_primary_states
                .contains(&next_primary_state),
        );

        self.last_primary_state = next_primary_state;

        Ok(MainOutputs {
            primary_state: next_primary_state.into(),
        })
    }

    fn game_state_to_primary_state(
        &self,
        game_state: FilteredGameState,
        is_penalized: bool,
    ) -> PrimaryState {
        if is_penalized {
            if game_state == FilteredGameState::Finished {
                return PrimaryState::Finished;
            }
            PrimaryState::Penalized
        } else {
            match game_state {
                FilteredGameState::Initial => PrimaryState::Initial,
                FilteredGameState::Ready => PrimaryState::Ready,
                FilteredGameState::Set => PrimaryState::Set,
                FilteredGameState::Playing { .. } => PrimaryState::Playing,
                FilteredGameState::Finished => PrimaryState::Finished,
                FilteredGameState::Stop => PrimaryState::Stop,
            }
        }
    }
}

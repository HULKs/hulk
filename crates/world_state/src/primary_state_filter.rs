use std::collections::HashSet;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{RecordingInterface, SimulatorInterface, SpeakerInterface};
use hsl_network_messages::PlayerNumber;
use serde::{Deserialize, Serialize};
use types::{
    buttons::{ButtonPressType, Buttons},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct PrimaryStateFilter {
    last_primary_state: PrimaryState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    buttons: Input<Buttons<Option<ButtonPressType>>, "buttons">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    is_safe_pose: Input<bool, "is_safe_pose">,

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
        context: CycleContext<impl RecordingInterface + SimulatorInterface + SpeakerInterface>,
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

        let next_primary_state = match (self.last_primary_state, context.buttons, game_state) {
            (
                _,
                Buttons {
                    f1: Some(ButtonPressType::Short),
                    stand: Some(ButtonPressType::Short),
                    ..
                },
                _,
            ) => PrimaryState::Safe,
            (
                PrimaryState::Safe,
                Buttons {
                    stand: Some(ButtonPressType::Long),
                    ..
                },
                _,
            ) if *context.is_safe_pose => PrimaryState::Initial,
            (
                PrimaryState::Initial,
                Buttons {
                    stand: Some(ButtonPressType::Long),
                    ..
                },
                _,
            ) if *context.is_safe_pose => PrimaryState::Playing,
            (PrimaryState::Safe, _, _) => {
                if context.hardware_interface.is_simulation()? {
                    PrimaryState::Initial
                } else {
                    PrimaryState::Safe
                }
            }
            (PrimaryState::Initial, _, FilteredGameState::Ready) if !is_penalized => {
                PrimaryState::Ready
            }
            (PrimaryState::Ready, _, FilteredGameState::Set) if !is_penalized => PrimaryState::Set,
            (PrimaryState::Set, _, FilteredGameState::Playing { .. }) if !is_penalized => {
                PrimaryState::Playing
            }
            (PrimaryState::Playing, _, FilteredGameState::Ready) if !is_penalized => {
                PrimaryState::Ready
            }
            (state, _, FilteredGameState::Finished) if !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Finished
            }
            (state, _, FilteredGameState::Stop) if !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Stop
            }
            (state, _, _) if is_penalized && !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Penalized
            }
            (PrimaryState::Stop, _, game_state) => {
                self.game_state_to_primary_state(game_state, is_penalized)
            }
            (PrimaryState::Penalized, _, game_state) if !is_penalized => {
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

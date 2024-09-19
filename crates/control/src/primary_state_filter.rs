use std::collections::HashSet;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{RecordingInterface, SpeakerInterface};
use serde::{Deserialize, Serialize};
use types::{
    audio::{Sound, SpeakerRequest},
    buttons::Buttons,
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
    buttons: Input<Buttons, "buttons">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,

    jersey_number: Parameter<usize, "jersey_number">,
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
            last_primary_state: PrimaryState::Unstiff,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl RecordingInterface + SpeakerInterface>,
    ) -> Result<MainOutputs> {
        let is_penalized = match context.filtered_game_controller_state {
            Some(game_controller_state) => {
                game_controller_state.penalties[context.jersey_number].is_some()
            }
            None => false,
        };

        self.last_primary_state = match (
            self.last_primary_state,
            context.buttons.head_buttons_touched,
            context.buttons.is_chest_button_pressed_once,
            context.buttons.calibration_buttons_touched,
            context.filtered_game_controller_state,
            context.buttons.animation_buttons_touched,
        ) {
            // Unstiff transitions (entering and exiting)
            (last_primary_state, true, _, _, _, _) => {
                if last_primary_state != PrimaryState::Unstiff {
                    context
                        .hardware_interface
                        .write_to_speakers(SpeakerRequest::PlaySound { sound: Sound::Sigh });
                }
                PrimaryState::Unstiff
            }

            (PrimaryState::Calibration, ..) => PrimaryState::Calibration,

            (PrimaryState::Initial, _, _, true, _, _) => PrimaryState::Calibration,

            // GameController transitions (entering listening mode and staying within)
            (PrimaryState::Unstiff, _, true, _, Some(filtered_game_controller_state), _)
            | (PrimaryState::Finished, _, true, _, Some(filtered_game_controller_state), _) => {
                Self::game_state_to_primary_state(
                    filtered_game_controller_state.game_state,
                    is_penalized,
                )
            }
            (_, _, _, _, Some(filtered_game_controller_state), _)
                if {
                    let finished_to_initial = self.last_primary_state == PrimaryState::Finished
                        && filtered_game_controller_state.game_state == FilteredGameState::Initial;

                    self.last_primary_state != PrimaryState::Unstiff || finished_to_initial
                } =>
            {
                Self::game_state_to_primary_state(
                    filtered_game_controller_state.game_state,
                    is_penalized,
                )
            }

            // non-GameController transitions
            (PrimaryState::Unstiff, _, true, _, None, _) => PrimaryState::Initial,
            (
                PrimaryState::Unstiff | PrimaryState::Animation { stiff: true },
                _,
                false,
                _,
                None,
                true,
            ) => PrimaryState::Animation { stiff: false },
            (PrimaryState::Animation { .. }, _, true, _, None, false) => {
                PrimaryState::Animation { stiff: true }
            }
            (PrimaryState::Finished, _, true, _, None, _) => PrimaryState::Initial,
            (PrimaryState::Initial, _, true, _, None, _) => PrimaryState::Penalized,
            (PrimaryState::Penalized, _, true, _, None, _) => PrimaryState::Playing,
            (PrimaryState::Playing, _, true, _, None, _) => PrimaryState::Penalized,

            (_, _, _, _, _, _) => self.last_primary_state,
        };

        context.hardware_interface.set_whether_to_record(
            context
                .recorded_primary_states
                .contains(&self.last_primary_state),
        );

        Ok(MainOutputs {
            primary_state: self.last_primary_state.into(),
        })
    }

    fn game_state_to_primary_state(
        game_state: FilteredGameState,
        is_penalized: bool,
    ) -> PrimaryState {
        if is_penalized {
            return PrimaryState::Penalized;
        }
        match game_state {
            FilteredGameState::Ready { .. } => PrimaryState::Ready,
            FilteredGameState::Initial => PrimaryState::Initial,
            FilteredGameState::Set => PrimaryState::Set,
            FilteredGameState::Playing { .. } => PrimaryState::Playing,
            FilteredGameState::Finished => PrimaryState::Finished,
            FilteredGameState::Standby => PrimaryState::Standby,
        }
    }
}

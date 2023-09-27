use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::SpeakerInterface;
use spl_network_messages::PlayerNumber;
use types::{
    audio::{Sound, SpeakerRequest},
    buttons::Buttons,
    filtered_game_state::FilteredGameState,
    game_controller_state::GameControllerState,
    primary_state::PrimaryState,
};

pub struct PrimaryStateFilter {
    last_primary_state: PrimaryState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    buttons: Input<Buttons, "buttons">,
    filtered_game_state: Input<Option<FilteredGameState>, "filtered_game_state?">,
    game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,

    player_number: Parameter<PlayerNumber, "player_number">,

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

    pub fn cycle(&mut self, context: CycleContext<impl SpeakerInterface>) -> Result<MainOutputs> {
        let is_penalized = match context.game_controller_state {
            Some(game_controller_state) => {
                game_controller_state.penalties[*context.player_number].is_some()
            }
            None => false,
        };

        self.last_primary_state = match (
            self.last_primary_state,
            context.buttons.head_buttons_touched,
            context.buttons.is_chest_button_pressed,
            context.buttons.calibration_buttons_touched,
            context.filtered_game_state,
        ) {
            // Unstiff transitions (entering and exiting)
            (last_primary_state, true, _, _, _) => {
                if last_primary_state != PrimaryState::Unstiff {
                    context
                        .hardware_interface
                        .write_to_speakers(SpeakerRequest::PlaySound { sound: Sound::Sigh });
                }
                PrimaryState::Unstiff
            }

            (PrimaryState::Initial, _, _, true, _) => PrimaryState::Calibration,

            // GameController transitions (entering listening mode and staying within)
            (PrimaryState::Unstiff, _, true, _, Some(game_state))
            | (PrimaryState::Finished, _, true, _, Some(game_state)) => {
                Self::game_state_to_primary_state(*game_state, is_penalized)
            }
            (_, _, _, _, Some(game_state))
                if self.last_primary_state != PrimaryState::Unstiff
                    && self.last_primary_state != PrimaryState::Finished =>
            {
                Self::game_state_to_primary_state(*game_state, is_penalized)
            }

            // non-GameController transitions
            (PrimaryState::Unstiff, _, true, _, None) => PrimaryState::Initial,
            (PrimaryState::Finished, _, true, _, None) => PrimaryState::Initial,
            (PrimaryState::Initial, _, true, _, None) => PrimaryState::Penalized,
            (PrimaryState::Penalized, _, true, _, None) => PrimaryState::Playing,
            (PrimaryState::Playing, _, true, _, None) => PrimaryState::Penalized,

            (_, _, _, _, _) => self.last_primary_state,
        };

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
        }
    }
}

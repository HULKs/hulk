use module_derive::{module, require_some};
use spl_network::PlayerNumber;
use types::{Buttons, FilteredGameState, GameControllerState, PrimaryState};

pub struct PrimaryStateFilter {
    last_primary_state: PrimaryState,
}

#[module(control)]
#[input(path = buttons, data_type = Buttons)]
#[parameter(path = player_number, data_type = PlayerNumber)]
#[input(path = game_controller_state, data_type = GameControllerState)]
#[input(path = filtered_game_state, data_type = FilteredGameState)]
#[input(path = has_ground_contact, data_type = bool)]
#[main_output(data_type = PrimaryState)]
impl PrimaryStateFilter {}

impl PrimaryStateFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_primary_state: PrimaryState::Unstiff,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let has_ground_contact = require_some!(context.has_ground_contact);
        let buttons = require_some!(context.buttons);
        let is_penalized = match context.game_controller_state {
            Some(game_controller_state) => {
                game_controller_state.penalties[*context.player_number].is_some()
            }
            None => false,
        };

        self.last_primary_state = match (
            self.last_primary_state,
            buttons.head_buttons_touched,
            buttons.is_chest_button_pressed,
            buttons.calibration_buttons_touched,
            context.filtered_game_state,
            has_ground_contact,
        ) {
            // Unstiff transitions (entering and exiting)
            (_, true, _, _, _, true) => PrimaryState::Finished,
            (_, true, _, _, _, false) => PrimaryState::Unstiff,

            (PrimaryState::Initial, _, _, true, _, _) => PrimaryState::Calibration,

            // GameController transitions (entering listening mode and staying within)
            (PrimaryState::Unstiff, _, true, _, Some(game_state), _)
            | (PrimaryState::Finished, _, true, _, Some(game_state), _) => {
                Self::game_state_to_primary_state(*game_state, is_penalized)
            }
            (_, _, _, _, Some(game_state), _)
                if self.last_primary_state != PrimaryState::Unstiff
                    && self.last_primary_state != PrimaryState::Finished =>
            {
                Self::game_state_to_primary_state(*game_state, is_penalized)
            }

            // non-GameController transitions
            (PrimaryState::Unstiff, _, true, _, None, _) => PrimaryState::Initial,
            (PrimaryState::Finished, _, true, _, None, _) => PrimaryState::Initial,
            (PrimaryState::Initial, _, true, _, None, _) => PrimaryState::Penalized,
            (PrimaryState::Penalized, _, true, _, None, _) => PrimaryState::Playing,
            (PrimaryState::Playing, _, true, _, None, _) => PrimaryState::Penalized,

            (_, _, _, _, _, _) => self.last_primary_state,
        };

        Ok(MainOutputs {
            primary_state: Some(self.last_primary_state),
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

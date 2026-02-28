use std::collections::HashSet;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{RecordingInterface, SpeakerInterface};
use hsl_network_messages::PlayerNumber;
use serde::{Deserialize, Serialize};
use types::{
    filtered_game_controller_state::FilteredGameControllerState, primary_state::PrimaryState,
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
        let _is_penalized = match context.filtered_game_controller_state {
            Some(game_controller_state) => {
                game_controller_state.penalties[*context.player_number].is_some()
            }
            None => false,
        };

        // TODO mode switching state machine
        let next_primary_state = PrimaryState::Safe;

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
}

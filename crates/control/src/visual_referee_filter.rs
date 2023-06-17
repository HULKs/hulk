use std::time::SystemTime;

use context_attribute::context;
use types::{CycleTime, FilteredWhistle, PrimaryState};

pub struct VisualRefereeFilter {
    last_whistle_caused_state_transition_time: SystemTime,
}

#[context]
pub struct CreationContext {
    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct CycleContext {
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub filtered_whistle: Input<Option<FilteredWhistle>, "filtered_whistle">,
    pub hardware: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl VisualRefereeFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok((Self {
            last_whistle_caused_state_transition_time: None,
        }))
    }

    pub fn cycle(&mut self, context: CycleContext) {
        let send_game_controller_vr_return_message: bool = false;


        if send_game_controller_vr_return_message {
            
        }
    }
}

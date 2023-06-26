use num_traits::cast::FromPrimitive;
use rand::prelude::*;
use std::time::Duration;

use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use spl_network_messages::VisualRefereeDecision;
use spl_network_messages::{PlayerNumber, VisualRefereeMessage};
use types::{
    hardware::Interface, messages::OutgoingMessage, CycleTime, FilteredWhistle, PrimaryState,
};

pub struct VisualRefereeFilter {
    last_primary_state: PrimaryState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,
    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub hardware: HardwareInterface,
}


#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl VisualRefereeFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_primary_state: PrimaryState::Unstiff,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let send_game_controller_visual_referee_return_message = matches!(
            (self.last_primary_state, *context.primary_state),
            (PrimaryState::Set, PrimaryState::Playing)
                | (
                    PrimaryState::Playing,
                    PrimaryState::Finished | PrimaryState::Ready
                )
        );
        self.last_primary_state = *context.primary_state;

        // Initially a random visual referee decision
        let mut rng = thread_rng();
        let gesture = VisualRefereeDecision::from_u32(rng.gen_range(1..=13)).unwrap();

        if send_game_controller_visual_referee_return_message {
            let mut duration_since_last_whistle = context
                .filtered_whistle
                .last_detection
                .map(|last_detection| {
                    context
                        .cycle_time
                        .start_time
                        .duration_since(last_detection)
                        .unwrap()
                })
                .unwrap_or(Duration::from_secs(15));
            if duration_since_last_whistle.as_secs_f32() < 1.0 {
                duration_since_last_whistle = Duration::from_secs(5)
            }
            let message = OutgoingMessage::VisualReferee(VisualRefereeMessage {
                player_number: *context.player_number,
                gesture,
                whistle_age: duration_since_last_whistle,
            });
            context
                .hardware
                .write_to_network(message)
                .wrap_err("failed to write VisualRefereeMessage to hardware")?;
        }
        Ok(MainOutputs::default())
    }
}

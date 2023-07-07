use num_traits::cast::FromPrimitive;
use rand::prelude::*;
use std::time::{Duration, SystemTime};

use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use hardware::NetworkInterface;
use spl_network_messages::{PlayerNumber, VisualRefereeMessage};
use spl_network_messages::{SubState, VisualRefereeDecision};
use types::{
    messages::OutgoingMessage, CycleTime, FilteredWhistle, GameControllerState, PrimaryState,
};

pub struct VisualRefereeFilter {
    last_primary_state: PrimaryState,
    time_of_last_visual_referee_related_state_change: Option<SystemTime>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,
    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub hardware: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    // VRC Output an Role Assignment
}

impl VisualRefereeFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_primary_state: PrimaryState::Unstiff,
            time_of_last_visual_referee_related_state_change: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl NetworkInterface>) -> Result<MainOutputs> {
        match (self.last_primary_state, *context.primary_state) {
            (PrimaryState::Set, PrimaryState::Playing)
            | (PrimaryState::Playing, PrimaryState::Finished | PrimaryState::Ready)
                if !matches!(
                    context.game_controller_state.sub_state,
                    Some(SubState::PenaltyKick)
                ) =>
            {
                self.time_of_last_visual_referee_related_state_change =
                    Some(context.cycle_time.start_time);
            }
            _ => {}
        }
        self.last_primary_state = *context.primary_state;

        if self
            .time_of_last_visual_referee_related_state_change
            .map_or(false, |time_of_last_state_change| {
                context
                    .cycle_time
                    .start_time
                    .duration_since(time_of_last_state_change)
                    .unwrap()
                    .as_secs_f32()
                    > 8.0
            })
        {
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
                duration_since_last_whistle = Duration::from_secs(8)
            }

            // Initially a random visual referee decision
            let mut rng = thread_rng();
            let gesture = VisualRefereeDecision::from_u32(rng.gen_range(1..=13)).unwrap();

            let message = OutgoingMessage::VisualReferee(VisualRefereeMessage {
                player_number: *context.player_number,
                gesture,
                whistle_age: duration_since_last_whistle,
            });
            context
                .hardware
                .write_to_network(message)
                .wrap_err("failed to write VisualRefereeMessage to hardware")?;

            self.time_of_last_visual_referee_related_state_change = None;
        }
        Ok(MainOutputs::default())
    }
}

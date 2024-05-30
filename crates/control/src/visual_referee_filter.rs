use num_traits::cast::FromPrimitive;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::{
    net::SocketAddr,
    time::{Duration, SystemTime},
};

use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use hardware::NetworkInterface;
use serde::{Deserialize, Serialize};
use spl_network_messages::{PlayerNumber, SubState, VisualRefereeDecision, VisualRefereeMessage};
use types::{
    cycle_time::CycleTime, filtered_whistle::FilteredWhistle,
    game_controller_state::GameControllerState, messages::OutgoingMessage,
    primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct VisualRefereeFilter {
    last_primary_state: PrimaryState,
    time_of_last_visual_referee_related_state_change: Option<SystemTime>,
    random_state: ChaChaRng,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    game_controller_address: Input<Option<SocketAddr>, "game_controller_address?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,
    player_number: Parameter<PlayerNumber, "player_number">,

    hardware: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl VisualRefereeFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_primary_state: PrimaryState::Unstiff,
            time_of_last_visual_referee_related_state_change: None,
            random_state: ChaChaRng::from_entropy(),
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
            .is_some_and(|time| {
                context
                    .cycle_time
                    .start_time
                    .duration_since(time)
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
            let gesture =
                VisualRefereeDecision::from_u32(self.random_state.gen_range(1..=13)).unwrap();

            if let Some(address) = context.game_controller_address {
                let message = OutgoingMessage::VisualReferee(
                    *address,
                    VisualRefereeMessage {
                        player_number: *context.player_number,
                        gesture,
                        whistle_age: duration_since_last_whistle,
                    },
                );
                context
                    .hardware
                    .write_to_network(message)
                    .wrap_err("failed to write VisualRefereeMessage to hardware")?;

                self.time_of_last_visual_referee_related_state_change = None;
            }
        }
        Ok(MainOutputs::default())
    }
}

use std::time::{Duration, SystemTime};

use color_eyre::Result;
use framework::{MainOutput, PerceptionInput};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use spl_network_messages::{HulkMessage, StrikerMessage};
use types::{cycle_time::CycleTime, messages::IncomingMessage};

#[derive(Deserialize, Serialize)]
pub struct ReceivedTimeToReachKickPosition {
    received_team_player_at_kick_position: Option<SystemTime>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    messages: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub team_time_to_reach_kick_position: MainOutput<Option<Duration>>,
}

impl ReceivedTimeToReachKickPosition {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            received_team_player_at_kick_position: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        self.received_team_player_at_kick_position = context
            .messages
            .persistent
            .into_iter()
            .flat_map(|(time, messages)| {
                messages
                    .into_iter()
                    .filter_map(move |message| match message {
                        Some(IncomingMessage::Spl(HulkMessage::Striker(StrikerMessage {
                            time_to_reach_kick_position: Some(duration),
                            ..
                        }))) => Some(time + *duration),
                        _ => None,
                    })
            })
            .min();

        let now = context.cycle_time.start_time;
        let team_to_reach_kick_position = self
            .received_team_player_at_kick_position
            .map(|time| time.duration_since(now).unwrap_or_default());

        Ok(MainOutputs {
            team_time_to_reach_kick_position: team_to_reach_kick_position.into(),
        })
    }
}

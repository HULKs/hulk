use std::{boxed::Box, future::Future, pin::Pin};
use std::{collections::VecDeque, sync::Arc, time::SystemTime};

use color_eyre::Result;
use hsl_network_messages::{
    GamePhase, GameState, HulkMessage, PlayerNumber, SubState, WhistleMessage,
};
use serde::{Deserialize, Serialize};

use ros_z::{prelude::*, qos::QosDurability};
use types::{
    filtered_whistle::FilteredWhistle,
    game_controller_state::GameControllerState,
    messages::{IncomingMessage, OutgoingMessage},
    time_wrapper::TimeWrapper,
    whistle::Whistle,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub broadcast: bool,
    pub remaining_amount_of_messages_to_stop_sending: u16,
    pub buffer_length: usize,
    pub minimum_detections: usize,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("whistle_filter").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("whistle_filter")?;
    let detected_whistle_sub = node
        .subscriber::<Whistle>("detected_whistle")
        .build()
        .await?;
    let network_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")
        .build()
        .await?;
    let player_number_cache = node
        .subscriber::<PlayerNumber>("player_number")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let game_controller_state_cache = node
        .subscriber::<Option<GameControllerState>>("game_controller_state")
        .cache(1)
        .build()
        .await?;
    let outgoing_message_pub = node
        .publisher::<OutgoingMessage>("outputs/message")
        .build()
        .await?;
    let filtered_whistle_pub = node
        .publisher::<FilteredWhistle>("filtered_whistle")
        .build()
        .await?;

    let mut filter = WhistleFilter::default();

    loop {
        let filtered_whistle = tokio::select! {
            detected_whistle = detected_whistle_sub.recv() => {
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let (filtered_whistle, should_broadcast) = filter.update_audio(
                    detected_whistle?, parameters, node.clock().now().to_wallclock(),
                );
                if should_broadcast && let Some(player_number) = player_number_cache.get_latest() {
                    let state = game_controller_state_cache.get_latest();
                    let allowed_to_broadcast = state.as_deref().and_then(Option::as_ref).is_none_or(|state| {
                        !state.stopped
                            && matches!(state.game_state, GameState::Set | GameState::Playing)
                            && state.penalties[*player_number].is_none()
                            && !matches!(state.game_phase, GamePhase::PenaltyShootout { .. })
                            && state.sub_state != Some(SubState::PenaltyKick)
                            && state.hulks_team.remaining_amount_of_messages
                                >= parameters.remaining_amount_of_messages_to_stop_sending
                    });
                    if allowed_to_broadcast {
                        outgoing_message_pub.publish(&OutgoingMessage::Hsl(HulkMessage::Whistle(
                            WhistleMessage { player_number: *player_number },
                        ))).await?;
                    }
                }
                filtered_whistle
            }
            message = network_message_sub.recv() => {
                if !matches!(message?.inner, IncomingMessage::Hsl(HulkMessage::Whistle(_))) {
                    continue;
                }
                filter.receive_whistle(node.clock().now().to_wallclock())
            }
        };
        filtered_whistle_pub.publish(&filtered_whistle).await?;
    }
}

#[derive(Default)]
struct WhistleFilter {
    detection_buffer: VecDeque<bool>,
    was_detected: bool,
    last_detection: Option<SystemTime>,
}

impl WhistleFilter {
    fn update_audio(
        &mut self,
        whistle: Whistle,
        parameters: &Parameters,
        now: SystemTime,
    ) -> (FilteredWhistle, bool) {
        for is_detected in whistle.is_detected {
            self.detection_buffer.push_front(is_detected);
        }
        self.detection_buffer.truncate(parameters.buffer_length);
        let number_of_detections = self
            .detection_buffer
            .iter()
            .filter(|&&is_detected| is_detected)
            .count();
        let is_detected = number_of_detections >= parameters.minimum_detections;
        let started = is_detected && !self.was_detected;
        if started {
            self.last_detection = Some(now);
        }
        self.was_detected = is_detected;

        (
            FilteredWhistle {
                is_detected,
                last_detection: self.last_detection,
            },
            started && parameters.broadcast,
        )
    }

    fn receive_whistle(&mut self, now: SystemTime) -> FilteredWhistle {
        self.last_detection = Some(now);
        FilteredWhistle {
            is_detected: true,
            last_detection: self.last_detection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARAMETERS: Parameters = Parameters {
        broadcast: true,
        buffer_length: 2,
        minimum_detections: 2,
        remaining_amount_of_messages_to_stop_sending: 20,
    };

    fn audio(detected: bool) -> Whistle {
        Whistle {
            is_detected: vec![detected; 2],
        }
    }

    #[test]
    fn broadcasts_once_per_local_whistle_when_enabled() {
        let mut filter = WhistleFilter::default();
        let now = SystemTime::UNIX_EPOCH;
        assert!(filter.update_audio(audio(true), &PARAMETERS, now).1);
        assert!(!filter.update_audio(audio(true), &PARAMETERS, now).1);
        filter.update_audio(audio(false), &PARAMETERS, now);
        let disabled = Parameters {
            broadcast: false,
            ..PARAMETERS
        };
        let (whistle, broadcast) = filter.update_audio(audio(true), &disabled, now);
        assert!(whistle.is_detected);
        assert!(!broadcast);
    }

    #[test]
    fn receives_without_audio_and_does_not_rebroadcast() {
        let mut filter = WhistleFilter::default();
        let now = SystemTime::UNIX_EPOCH;
        let whistle = filter.receive_whistle(now);
        assert!(whistle.is_detected);
        assert_eq!(whistle.last_detection, Some(now));
        let (after, broadcast) = filter.update_audio(audio(false), &PARAMETERS, now);
        assert!(!after.is_detected);
        assert_eq!(after.last_detection, Some(now));
        assert!(!broadcast);
        assert!(filter.update_audio(audio(true), &PARAMETERS, now).1);
    }
}

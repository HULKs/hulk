use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, SystemTime},
};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use hardware::SpeakerInterface;
use serde::{Deserialize, Serialize};
use spl_network_messages::GameControllerStateMessage;
use types::{
    audio::{Sound, SpeakerRequest},
    cycle_time::CycleTime,
    game_controller_state::GameControllerState,
    messages::IncomingMessage,
};

#[derive(Deserialize, Serialize)]
pub struct GameControllerFilter {
    game_controller_state: Option<GameControllerState>,
    last_game_state_change: Option<SystemTime>,

    last_contact: HashMap<SocketAddr, SystemTime>,
    last_collision_warning: Option<SystemTime>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
    cycle_time: Input<CycleTime, "cycle_time">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,

    time_since_last_message_to_consider_ip_active:
        Parameter<Duration, "game_controller_filter.time_since_last_message_to_consider_ip_active">,
    collision_alert_cooldown:
        Parameter<Duration, "game_controller_filter.collision_alert_cooldown">,

    last_contact:
        AdditionalOutput<HashMap<SocketAddr, SystemTime>, "game_controller_address_contacts_times">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub game_controller_state: MainOutput<Option<GameControllerState>>,
    pub game_controller_address: MainOutput<Option<SocketAddr>>,
}

impl GameControllerFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            game_controller_state: None,
            last_game_state_change: None,
            last_contact: HashMap::new(),
            last_collision_warning: None,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl SpeakerInterface>,
    ) -> Result<MainOutputs> {
        for (time, source_address, message) in context
            .network_message
            .persistent
            .iter()
            .flat_map(|(time, messages)| messages.iter().flatten().map(|message| (*time, *message)))
            .filter_map(|(time, message)| match message {
                IncomingMessage::GameController(source_address, message) => {
                    Some((time, source_address, message))
                }
                _ => None,
            })
        {
            self.update_game_controller_state(&context, message);
            self.alert_if_multiple_game_controllers(&context, *source_address, time);
        }

        context
            .last_contact
            .fill_if_subscribed(|| self.last_contact.clone());

        let last_address = self
            .last_contact
            .iter()
            .max_by_key(|(_address, time)| *time)
            .map(|(address, _time)| *address);

        Ok(MainOutputs {
            game_controller_state: self.game_controller_state.clone().into(),
            game_controller_address: last_address.into(),
        })
    }

    fn update_game_controller_state<T>(
        &mut self,
        context: &CycleContext<T>,
        message: &GameControllerStateMessage,
    ) {
        let game_state_changed = match &self.game_controller_state {
            Some(game_controller_state) => game_controller_state.game_state != message.game_state,
            None => true,
        };
        if game_state_changed {
            self.last_game_state_change = Some(context.cycle_time.start_time);
        }
        self.game_controller_state = Some(GameControllerState {
            game_state: message.game_state,
            game_phase: message.game_phase,
            kicking_team: message.kicking_team,
            last_game_state_change: self.last_game_state_change.unwrap(),
            penalties: message.hulks_team.clone().into(),
            opponent_penalties: message.opponent_team.clone().into(),
            remaining_amount_of_messages: message.hulks_team.remaining_amount_of_messages,
            sub_state: message.sub_state,
            hulks_team_is_home_after_coin_toss: message.hulks_team_is_home_after_coin_toss,
            hulks_team: message.hulks_team.clone(),
            opponent_team: message.opponent_team.clone(),
        });
    }

    fn alert_if_multiple_game_controllers(
        &mut self,
        context: &CycleContext<impl SpeakerInterface>,
        source_address: SocketAddr,
        time: SystemTime,
    ) {
        self.last_contact.insert(source_address, time);

        let recent_contacts = self.last_contact.iter().filter(|(_address, last_contact)| {
            time.duration_since(**last_contact)
                .expect("time ran backwards")
                < *context.time_since_last_message_to_consider_ip_active
        });
        let collision_detected = recent_contacts.count() > 1;

        let alert_is_on_cooldown =
            self.last_collision_warning
                .is_some_and(|last_collision_warning| {
                    time.duration_since(last_collision_warning)
                        .expect("time ran backwards")
                        < *context.collision_alert_cooldown
                });

        if collision_detected && !alert_is_on_cooldown {
            context
                .hardware_interface
                .write_to_speakers(SpeakerRequest::PlaySound {
                    sound: Sound::GameControllerCollision,
                });
            self.last_collision_warning = Some(time);
        }
    }
}

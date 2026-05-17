use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::Result;
use ros_z_streams::CreateFutureMapBuilder;
use serde::{Deserialize, Serialize};

use hsl_network_messages::GameControllerStateMessage;
use ros_z::{prelude::*, time::Clock};
use types::{
    field_dimensions::GlobalFieldSide, game_controller_state::GameControllerState,
    messages::IncomingMessage,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub time_since_last_message_to_consider_ip_active: Duration,
    pub collision_alert_cooldown: Duration,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("game_controller_filter").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("game_controller_filter")?;
    let mut network_message_sub = node
        .create_future_map_builder()
        .create_future_subscriber::<IncomingMessage>("filtered_message", Duration::from_millis(1))
        .await?
        .build();
    let last_contact_pub = node
        .publisher::<HashMap<SocketAddr, SystemTime>>("game_controller_address_contacts_times")?
        .build()
        .await?;
    let game_controller_state_pub = node
        .publisher::<Option<GameControllerState>>("game_controller_state")?
        .build()
        .await?;
    let game_controller_address_pub = node
        .publisher::<Option<SocketAddr>>("game_controller_address")?
        .build()
        .await?;

    let mut game_controller_filter = GameControllerFilter::default();

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let network_message = network_message_sub.recv().await?;

        for (_, source_address, message) in
            network_message
                .persistent
                .iter()
                .filter_map(|(time, message)| match message {
                    (Some(IncomingMessage::GameController(source_address, message)),) => {
                        Some((time, source_address, message))
                    }
                    _ => None,
                })
        {
            game_controller_filter.update_game_controller_state(ctx.clock(), message);
            game_controller_filter.alert_if_multiple_game_controllers(
                ctx.clock(),
                parameters,
                *source_address,
            );

            last_contact_pub
                .publish(&game_controller_filter.last_contact)
                .await?;

            let last_address = game_controller_filter
                .last_contact
                .iter()
                .max_by_key(|(_address, time)| *time)
                .map(|(address, _time)| *address);

            game_controller_state_pub
                .publish(&game_controller_filter.game_controller_state)
                .await?;
            game_controller_address_pub.publish(&last_address).await?;
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct GameControllerFilter {
    game_controller_state: Option<GameControllerState>,
    last_game_state_change: Option<SystemTime>,

    last_contact: HashMap<SocketAddr, SystemTime>,
    last_collision_warning: Option<SystemTime>,
}

impl GameControllerFilter {
    fn update_game_controller_state(
        &mut self,
        clock: &Clock,
        message: &GameControllerStateMessage,
    ) {
        let game_state_changed = match &self.game_controller_state {
            Some(game_controller_state) => game_controller_state.game_state != message.game_state,
            None => true,
        };
        if game_state_changed {
            self.last_game_state_change = Some(clock.now().to_wallclock());
        }
        self.game_controller_state = Some(GameControllerState {
            game_state: message.game_state,
            stopped: message.stopped,
            game_phase: message.game_phase,
            remaining_time_in_half: message.remaining_time_in_half,
            kicking_team: message.kicking_team,
            last_game_state_change: self.last_game_state_change.unwrap(),
            penalties: message.hulks_team.clone().into(),
            opponent_penalties: message.opponent_team.clone().into(),
            sub_state: message.sub_state,
            global_field_side: if message.hulks_team_is_home_after_coin_toss {
                GlobalFieldSide::Home
            } else {
                GlobalFieldSide::Away
            },
            hulks_team: message.hulks_team.clone(),
            opponent_team: message.opponent_team.clone(),
        });
    }

    fn alert_if_multiple_game_controllers(
        &mut self,
        clock: &Clock,
        parameters: &Parameters,
        source_address: SocketAddr,
    ) {
        self.last_contact
            .insert(source_address, clock.now().to_wallclock());

        let recent_contacts = self.last_contact.iter().filter(|(_address, last_contact)| {
            clock.now().duration_since((**last_contact).into())
                < parameters.time_since_last_message_to_consider_ip_active
        });
        let collision_detected = recent_contacts.count() > 1;

        let alert_is_on_cooldown =
            self.last_collision_warning
                .is_some_and(|last_collision_warning| {
                    clock.now().duration_since(last_collision_warning.into())
                        < parameters.collision_alert_cooldown
                });

        if collision_detected && !alert_is_on_cooldown {
            // TODO: We currently do not have audio output implemented
            //     hardware_interface
            //     .write_to_speakers(SpeakerRequest::PlaySound {
            //         sound: Sound::GameControllerCollision,
            //     });
            self.last_collision_warning = Some(clock.now().to_wallclock());
        }
    }
}

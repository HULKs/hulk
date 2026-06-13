use std::{boxed::Box, future::Future, pin::Pin};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use hsl_network_messages::GameControllerStateMessage;
use ros_z::{prelude::*, time::Time};
use types::{
    field_dimensions::GlobalFieldSide, game_controller_state::GameControllerState,
    messages::IncomingMessage, time_wrapper::TimeWrapper,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub time_since_last_message_to_consider_ip_active: Duration,
    pub collision_alert_cooldown: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("game_controller_filter").build().await?;

    let parameters = node
        .bind_parameter_as::<Parameters>("game_controller_filter")
        .await?;
    let network_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")?
        .build()
        .await?;
    let last_contact_pub = node
        .publisher::<HashMap<SocketAddr, Time>>("game_controller_address_contacts_times")?
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

        let TimeWrapper {
            time,
            inner: IncomingMessage::GameController(source_address, message),
        } = network_message
        else {
            continue;
        };

        game_controller_filter.update_game_controller_state(&time, &message);
        game_controller_filter.alert_if_multiple_game_controllers(
            &time,
            parameters,
            source_address,
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

#[derive(Default, Deserialize, Serialize)]
pub struct GameControllerFilter {
    game_controller_state: Option<GameControllerState>,
    last_game_state_change: Option<Time>,
    last_contact: HashMap<SocketAddr, Time>,
    last_collision_warning: Option<Time>,
}

impl GameControllerFilter {
    fn update_game_controller_state(&mut self, time: &Time, message: &GameControllerStateMessage) {
        let game_state_changed = match &self.game_controller_state {
            Some(game_controller_state) => game_controller_state.game_state != message.game_state,
            None => true,
        };
        if game_state_changed {
            self.last_game_state_change = Some(*time);
        }
        self.game_controller_state = Some(GameControllerState {
            game_state: message.game_state,
            stopped: message.stopped,
            game_phase: message.game_phase,
            remaining_time_in_half: message.remaining_time_in_half,
            kicking_team: message.kicking_team,
            last_game_state_change: self.last_game_state_change.unwrap().to_wallclock(),
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
        time: &Time,
        parameters: &Parameters,
        source_address: SocketAddr,
    ) {
        self.last_contact.insert(source_address, *time);

        let recent_contacts = self.last_contact.iter().filter(|(_address, last_contact)| {
            time.duration_since(**last_contact)
                < parameters.time_since_last_message_to_consider_ip_active
        });
        let collision_detected = recent_contacts.count() > 1;

        let alert_is_on_cooldown =
            self.last_collision_warning
                .is_some_and(|last_collision_warning| {
                    time.duration_since(last_collision_warning)
                        < parameters.collision_alert_cooldown
                });

        if collision_detected && !alert_is_on_cooldown {
            // TODO: We currently do not have audio output implemented
            //     hardware_interface
            //     .write_to_speakers(SpeakerRequest::PlaySound {
            //         sound: Sound::GameControllerCollision,
            //     });
            self.last_collision_warning = Some(*time);
        }
    }
}

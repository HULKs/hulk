use std::{boxed::Box, future::Future, pin::Pin};
use std::{collections::HashSet, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use hsl_network_messages::PlayerNumber;
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    buttons::{ButtonPressType, Buttons},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    primary_state::PrimaryState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub injected_primary_state: Option<PrimaryState>,
    pub recorded_primary_states: HashSet<PrimaryState>,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("primary_state_filter").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("primary_state_filter")?;
    let player_number_cache = node
        .create_cache::<PlayerNumber>("player_number", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;
    let buttons_sub = node
        .subscriber::<Buttons<Option<ButtonPressType>>>("buttons")?
        .build()
        .await?;
    let is_safe_pose_cache = node
        .create_cache::<bool>("is_safe_pose", 1)?
        .build()
        .await?;

    let primary_state_pub = node
        .publisher::<PrimaryState>("primary_state")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let mut primary_state_filter = PrimaryStateFilter::default();
    primary_state_pub
        .publish(&primary_state_filter.primary_state)
        .await?;

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        tokio::select! {
            received_filtered_game_controller_state = filtered_game_controller_state_sub.recv() => {
                let Some(player_number) = player_number_cache.get_latest() else {continue};

                let filtered_game_controller_state = received_filtered_game_controller_state?;

                primary_state_filter.update_with_filtered_game_contoller_state(
                    &filtered_game_controller_state,
                    *player_number,
                );
            }
            received_buttons = buttons_sub.recv() => {
                let Some(is_safe_pose) = is_safe_pose_cache.get_latest() else {continue};

                let buttons = received_buttons?;

                primary_state_filter.update_with_buttons(&buttons, *is_safe_pose);

            }
        }

        if let Some(injected_primary_state) = parameters.injected_primary_state {
            primary_state_filter.update_with_injected_primary_state(injected_primary_state);
        }

        primary_state_pub
            .publish(&primary_state_filter.primary_state)
            .await?;
    }
}

#[derive(Default)]
struct PrimaryStateFilter {
    pub primary_state: PrimaryState,
}

impl PrimaryStateFilter {
    fn update_with_filtered_game_contoller_state(
        &mut self,
        filtered_game_controller_state: &FilteredGameControllerState,
        player_number: PlayerNumber,
    ) {
        let is_penalized = filtered_game_controller_state.penalties[player_number].is_some();
        let filtered_game_state = filtered_game_controller_state.game_state;

        self.primary_state = match (self.primary_state, filtered_game_state) {
            (PrimaryState::Safe, _) => PrimaryState::Safe,
            (PrimaryState::Initial, FilteredGameState::Ready) if !is_penalized => {
                PrimaryState::Ready
            }
            (PrimaryState::Ready, FilteredGameState::Set) if !is_penalized => PrimaryState::Set,
            (PrimaryState::Set, FilteredGameState::Playing { .. }) if !is_penalized => {
                PrimaryState::Playing
            }
            (PrimaryState::Playing, FilteredGameState::Ready) if !is_penalized => {
                PrimaryState::Ready
            }
            (state, FilteredGameState::Finished) if !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Finished
            }
            (state, FilteredGameState::Stop) if !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Stop
            }
            (state, _) if is_penalized && !matches!(state, PrimaryState::Safe) => {
                PrimaryState::Penalized
            }
            (PrimaryState::Stop, game_state) => {
                game_state_to_primary_state(game_state, is_penalized)
            }
            (PrimaryState::Penalized, game_state) if !is_penalized => {
                game_state_to_primary_state(game_state, false)
            }
            _ => self.primary_state,
        }
    }

    fn update_with_buttons(
        &mut self,
        buttons: &Buttons<Option<ButtonPressType>>,
        is_safe_pose: bool,
    ) {
        self.primary_state = match (self.primary_state, buttons) {
            (
                _,
                Buttons {
                    f1: Some(ButtonPressType::Short),
                    ..
                }
                | Buttons {
                    stand: Some(ButtonPressType::Short),
                    ..
                },
            ) => PrimaryState::Safe,
            (
                PrimaryState::Safe,
                Buttons {
                    stand: Some(ButtonPressType::Long),
                    ..
                },
            ) if is_safe_pose => PrimaryState::Initial,
            (
                PrimaryState::Safe,
                Buttons {
                    f1: Some(ButtonPressType::Long),
                    ..
                },
            ) => PrimaryState::Custom,
            (
                PrimaryState::Initial,
                Buttons {
                    stand: Some(ButtonPressType::Long),
                    ..
                },
            ) if is_safe_pose => PrimaryState::Playing,
            _ => self.primary_state,
        }
    }

    fn update_with_injected_primary_state(&mut self, injected_primary_state: PrimaryState) {
        self.primary_state = injected_primary_state
    }
}

fn game_state_to_primary_state(game_state: FilteredGameState, is_penalized: bool) -> PrimaryState {
    if is_penalized {
        if game_state == FilteredGameState::Finished {
            return PrimaryState::Finished;
        }
        PrimaryState::Penalized
    } else {
        match game_state {
            FilteredGameState::Initial => PrimaryState::Initial,
            FilteredGameState::Ready => PrimaryState::Ready,
            FilteredGameState::Set => PrimaryState::Set,
            FilteredGameState::Playing { .. } => PrimaryState::Playing,
            FilteredGameState::Finished => PrimaryState::Finished,
            FilteredGameState::Stop => PrimaryState::Stop,
        }
    }
}

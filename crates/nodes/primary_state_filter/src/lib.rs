use std::{boxed::Box, future::Future, pin::Pin, time::Duration};
use std::{collections::HashSet, sync::Arc};

use booster_sdk::client::BoosterClient;
use color_eyre::{Result, eyre::WrapErr};
use serde::{Deserialize, Serialize};

use hsl_network_messages::PlayerNumber;
use ros_z::{prelude::*, qos::QosDurability};
use tracing::{error, info, warn};
use types::{
    buttons::{ButtonPressType, Buttons},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    primary_state::PrimaryState,
};

const BOOSTER_MODE_RETRY_INTERVAL: Duration = Duration::from_millis(100);

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
        .subscriber::<PlayerNumber>("player_number")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await?;
    let buttons_sub = node
        .subscriber::<Buttons<Option<ButtonPressType>>>("buttons")
        .build()
        .await?;
    let is_safe_pose_cache = node
        .subscriber::<bool>("is_safe_pose")
        .cache(1)
        .build()
        .await?;

    let primary_state_pub = node
        .publisher::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let booster_client = BoosterClient::new()
        .wrap_err("failed to create BoosterClient for primary state initialization")?;
    let initial_primary_state = initial_primary_state_from_booster_mode(&booster_client).await;
    let mut primary_state_filter = PrimaryStateFilter::new(initial_primary_state);
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

                let previous_primary_state = primary_state_filter.primary_state;
                primary_state_filter.update_with_filtered_game_contoller_state(
                    &filtered_game_controller_state,
                    *player_number,
                );
                if primary_state_filter.primary_state != previous_primary_state {
                    info!(
                        target: "primary_state_filter::input",
                        previous_primary_state = ?previous_primary_state,
                        primary_state = ?primary_state_filter.primary_state,
                        game_state = ?filtered_game_controller_state.game_state,
                        player_number = ?*player_number,
                        penalty = ?filtered_game_controller_state.penalties[*player_number],
                        "primary state changed from game controller"
                    );
                }
            }
            received_buttons = buttons_sub.recv() => {
                let Some(is_safe_pose) = is_safe_pose_cache.get_latest() else {continue};

                let buttons = received_buttons?;

                let previous_primary_state = primary_state_filter.primary_state;
                let has_button_event = buttons.f1.is_some() || buttons.stand.is_some() || buttons.walking.is_some();
                if has_button_event {
                    info!(
                        target: "primary_state_filter::input",
                        ?buttons,
                        is_safe_pose = *is_safe_pose,
                        previous_primary_state = ?previous_primary_state,
                        "received button input"
                    );
                }
                primary_state_filter.update_with_buttons(&buttons, *is_safe_pose);
                if primary_state_filter.primary_state != previous_primary_state {
                    info!(
                        target: "primary_state_filter::input",
                        ?buttons,
                        is_safe_pose = *is_safe_pose,
                        previous_primary_state = ?previous_primary_state,
                        primary_state = ?primary_state_filter.primary_state,
                        "primary state changed from buttons"
                    );
                }
            }
        }

        if let Some(injected_primary_state) = parameters.injected_primary_state {
            let previous_primary_state = primary_state_filter.primary_state;
            primary_state_filter.update_with_injected_primary_state(injected_primary_state);
            if primary_state_filter.primary_state != previous_primary_state {
                info!(
                    target: "primary_state_filter::input",
                    previous_primary_state = ?previous_primary_state,
                    primary_state = ?primary_state_filter.primary_state,
                    "primary state changed from injected parameter"
                );
            }
        }

        primary_state_pub
            .publish(&primary_state_filter.primary_state)
            .await?;
    }
}

struct PrimaryStateFilter {
    pub primary_state: PrimaryState,
}

impl PrimaryStateFilter {
    fn new(primary_state: PrimaryState) -> Self {
        Self { primary_state }
    }

    fn update_with_filtered_game_contoller_state(
        &mut self,
        filtered_game_controller_state: &FilteredGameControllerState,
        player_number: PlayerNumber,
    ) {
        let is_penalized = filtered_game_controller_state.penalties[player_number].is_some();
        let filtered_game_state = filtered_game_controller_state.game_state;

        self.primary_state = match (self.primary_state, filtered_game_state) {
            (PrimaryState::Damping, _) => PrimaryState::Damping,
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
            (state, FilteredGameState::Finished) if !matches!(state, PrimaryState::Damping) => {
                PrimaryState::Finished
            }
            (state, FilteredGameState::Stop) if !matches!(state, PrimaryState::Damping) => {
                PrimaryState::Stop
            }
            (state, _) if is_penalized && !matches!(state, PrimaryState::Damping) => {
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
                },
            ) => PrimaryState::Damping,
            (
                _,
                Buttons {
                    stand: Some(ButtonPressType::Short),
                    ..
                },
            ) => PrimaryState::Prepare,
            (
                PrimaryState::Prepare,
                Buttons {
                    stand: Some(ButtonPressType::Long),
                    ..
                },
            ) if is_safe_pose => PrimaryState::Initial,
            (
                PrimaryState::Initial,
                Buttons {
                    walking: Some(ButtonPressType::Long),
                    ..
                },
            ) => PrimaryState::Playing,
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

fn initial_primary_state_from_robot_mode(
    robot_mode: booster_sdk::types::RobotMode,
) -> PrimaryState {
    match robot_mode {
        booster_sdk::types::RobotMode::Damping => PrimaryState::Damping,
        _ => PrimaryState::Prepare,
    }
}

async fn initial_primary_state_from_booster_mode(client: &BoosterClient) -> PrimaryState {
    loop {
        match client.get_mode().await {
            Ok(response) => {
                let Some(robot_mode) = response.mode_enum() else {
                    warn!(
                        target: "primary_state_filter::startup",
                        mode = response.mode,
                        "booster returned unrecognized robot mode"
                    );
                    tokio::time::sleep(BOOSTER_MODE_RETRY_INTERVAL).await;
                    continue;
                };

                let primary_state = initial_primary_state_from_robot_mode(robot_mode);
                info!(
                    target: "primary_state_filter::startup",
                    ?robot_mode,
                    ?primary_state,
                    "initialized primary state from booster mode"
                );
                return primary_state;
            }
            Err(error) => {
                error!(
                    target: "primary_state_filter::startup",
                    %error,
                    "failed to query booster mode"
                );
                tokio::time::sleep(BOOSTER_MODE_RETRY_INTERVAL).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damping_robot_mode_initializes_damping_primary_state() {
        assert_eq!(
            initial_primary_state_from_robot_mode(booster_sdk::types::RobotMode::Damping),
            PrimaryState::Damping
        );
    }

    #[test]
    fn non_damping_robot_modes_initialize_prepare_primary_state() {
        for robot_mode in [
            booster_sdk::types::RobotMode::Unknown,
            booster_sdk::types::RobotMode::Prepare,
            booster_sdk::types::RobotMode::Walking,
            booster_sdk::types::RobotMode::Custom,
            booster_sdk::types::RobotMode::Soccer,
        ] {
            assert_eq!(
                initial_primary_state_from_robot_mode(robot_mode),
                PrimaryState::Prepare,
                "{robot_mode:?} should initialize Prepare"
            );
        }
    }

    #[test]
    fn update_with_buttons_enters_playing_from_initial_with_safe_long_stand_press() {
        let mut primary_state_filter = PrimaryStateFilter::new(PrimaryState::Initial);
        let buttons = Buttons {
            f1: None,
            stand: None,
            walking: Some(ButtonPressType::Long),
        };

        primary_state_filter.update_with_buttons(&buttons, true);

        assert_eq!(primary_state_filter.primary_state, PrimaryState::Playing);
    }
}

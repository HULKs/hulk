use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk_interface::{GetRobotMode, GetRobotModeRequest, HighLevelCommand, RobotMode};
use ros_z::prelude::*;
use types::{
    buttons::{ButtonPressType, Buttons},
    primary_state::PrimaryState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct Parameters {
    pub wait_before_prepare: Duration,
    pub remote_stop_toggle: bool,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("robot_mode_handler").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("robot_mode_handler")?;
    let primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")?
        .build()
        .await?;
    let buttons_sub = node
        .subscriber::<Buttons<Option<ButtonPressType>>>("buttons")?
        .build()
        .await?;
    let high_level_command_pub = node
        .publisher::<HighLevelCommand>("commands/high_level_command")?
        .build()
        .await?;
    let get_robot_mode_client = node
        .create_service_client::<GetRobotMode>("services/get_robot_mode")?
        .build()
        .await?;

    let mut local_stop_toggle = false;
    let mut last_primary_state = PrimaryState::default();

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        tokio::select! {
            button_press = buttons_sub.recv() => {
                let buttons = button_press?;

                let is_local_stop_toggle_short_press =
                matches!(buttons.f1, Some(ButtonPressType::Short))
                    || matches!(buttons.stand, Some(ButtonPressType::Short));

                let should_enter_damping_mode = local_stop_toggle != parameters.remote_stop_toggle;

                if should_enter_damping_mode && is_local_stop_toggle_short_press {
                    local_stop_toggle = !local_stop_toggle;
                }

                if should_enter_damping_mode {
                    change_mode(&high_level_command_pub, RobotMode::Damping).await;
                }
            }
            primary_state = primary_state_sub.recv() => {
                let primary_state = primary_state?;

                let robot_mode = get_robot_mode_client
                    .call_async(&GetRobotModeRequest {})
                    .await?
                    .robot_mode;

                if primary_state != last_primary_state {
                    last_primary_state = primary_state;
                } else {
                    continue;
                }

                match (primary_state, robot_mode) {
                    (PrimaryState::Safe | PrimaryState::Initial, RobotMode::Walking) => {
                        change_mode(&high_level_command_pub, RobotMode::Prepare).await
                    }
                    (
                        PrimaryState::Ready
                        | PrimaryState::Playing
                        | PrimaryState::Set
                        | PrimaryState::Stop
                        | PrimaryState::Finished
                        | PrimaryState::Penalized,
                        RobotMode::Prepare
                    ) => change_mode(&high_level_command_pub, RobotMode::Walking).await,
                    (_, _) => (),
                };
            }
        }
    }
}

async fn change_mode(
    publischer: &Publisher<HighLevelCommand, SerdeCdrCodec<HighLevelCommand>>,
    robot_mode_to_set: RobotMode,
) {
    let _ = publischer
        .publish(&HighLevelCommand::ChangeMode {
            mode: robot_mode_to_set,
        })
        .await;
}

use std::sync::Arc;

use booster_sdk::client::light_control::SetLedLightColorParameter;
use booster_sdk_interface::LedCommand;
use color_eyre::Result;

use ros_z::{IntoEyreResultExt, prelude::*};
use types::primary_state::PrimaryState;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("led_handler").build().await.into_eyre()?;
    let primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let led_command_pub = node
        .publisher::<LedCommand>("commands/led_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let mut last_primary_state = None;

    loop {
        let primary_state = primary_state_sub.recv().await.into_eyre()?;

        if last_primary_state == Some(primary_state) {
            continue;
        }

        let light_control_parameter = match primary_state {
            PrimaryState::Safe => SetLedLightColorParameter::BLUE,
            PrimaryState::Stop => SetLedLightColorParameter::LIGHT_BLUE,
            PrimaryState::Ready => SetLedLightColorParameter::LIGHT_GREEN,
            PrimaryState::Initial => SetLedLightColorParameter::YELLOW,
            PrimaryState::Set => SetLedLightColorParameter::ORANGE,
            PrimaryState::Playing => SetLedLightColorParameter::GREEN,
            PrimaryState::Penalized => SetLedLightColorParameter::LIGHT_RED,
            PrimaryState::Finished => SetLedLightColorParameter::PURPLE,
        };

        let led_command = LedCommand::SetParam {
            r: light_control_parameter.r,
            g: light_control_parameter.g,
            b: light_control_parameter.b,
        };
        last_primary_state = Some(primary_state);

        led_command_pub.publish(&led_command).await.into_eyre()?;
    }
}

pub trait DefaultLEDColors {
    const BLUE: Self;
    const LIGHT_BLUE: Self;
    const RED: Self;
    const LIGHT_RED: Self;
    const GREEN: Self;
    const LIGHT_GREEN: Self;
    const ORANGE: Self;
    const YELLOW: Self;
    const PURPLE: Self;
}

impl DefaultLEDColors for SetLedLightColorParameter {
    const BLUE: Self = SetLedLightColorParameter { r: 0, g: 0, b: 255 };
    const RED: Self = SetLedLightColorParameter { r: 255, g: 0, b: 0 };
    const GREEN: Self = SetLedLightColorParameter { r: 0, g: 255, b: 0 };
    const LIGHT_BLUE: Self = SetLedLightColorParameter {
        r: 128,
        g: 128,
        b: 255,
    };
    const LIGHT_RED: Self = SetLedLightColorParameter {
        r: 255,
        g: 128,
        b: 128,
    };
    const LIGHT_GREEN: Self = SetLedLightColorParameter {
        r: 128,
        g: 255,
        b: 128,
    };
    const ORANGE: Self = SetLedLightColorParameter {
        r: 255,
        g: 128,
        b: 0,
    };
    const YELLOW: Self = SetLedLightColorParameter {
        r: 255,
        g: 255,
        b: 0,
    };
    const PURPLE: Self = SetLedLightColorParameter {
        r: 128,
        g: 0,
        b: 255,
    };
}

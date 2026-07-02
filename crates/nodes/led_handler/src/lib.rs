use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use booster::LedColor;
use booster_sdk_interface::LedCommand;
use color_eyre::Result;

use ros_z::{prelude::*, qos::QosDurability};
use types::primary_state::PrimaryState;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("led_handler").build().await?;
    let primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let led_command_pub = node
        .publisher::<LedCommand>("commands/led_command")
        .build()
        .await?;

    let mut last_primary_state = None;

    loop {
        let primary_state = primary_state_sub.recv().await?;

        if last_primary_state == Some(primary_state) {
            continue;
        }

        let light_control_parameter = match primary_state {
            PrimaryState::Damping => LedColor::BLUE,
            PrimaryState::Prepare => LedColor::YELLOW,
            PrimaryState::Stop => LedColor::BLACK,
            PrimaryState::Ready => LedColor::WHITE,
            PrimaryState::Initial => LedColor::MAGENTA,
            PrimaryState::Set => LedColor::ORANGE,
            PrimaryState::Playing => LedColor::GREEN,
            PrimaryState::Penalized => LedColor::RED,
            PrimaryState::Finished => LedColor::PURPLE,
        };

        let led_command = LedCommand::SetParam {
            r: light_control_parameter.r,
            g: light_control_parameter.g,
            b: light_control_parameter.b,
        };
        last_primary_state = Some(primary_state);

        led_command_pub.publish(&led_command).await?;
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
    const MAGENTA: Self;
    const BLACK: Self;
    const WHITE: Self;
}

impl DefaultLEDColors for LedColor {
    const BLUE: Self = LedColor { r: 0, g: 0, b: 255 };
    const RED: Self = LedColor { r: 255, g: 0, b: 0 };
    const GREEN: Self = LedColor {
        r: 0,
        g: 255,
        b: 50,
    };
    const LIGHT_BLUE: Self = LedColor {
        r: 128,
        g: 128,
        b: 255,
    };
    const LIGHT_RED: Self = LedColor {
        r: 255,
        g: 128,
        b: 128,
    };
    const LIGHT_GREEN: Self = LedColor {
        r: 128,
        g: 255,
        b: 128,
    };
    const ORANGE: Self = LedColor {
        r: 255,
        g: 94,
        b: 0,
    };
    const YELLOW: Self = LedColor {
        r: 255,
        g: 255,
        b: 0,
    };
    const PURPLE: Self = LedColor {
        r: 128,
        g: 50,
        b: 161,
    };
    const MAGENTA: Self = LedColor {
        r: 255,
        g: 0,
        b: 116,
    };
    const BLACK: Self = LedColor { r: 0, g: 0, b: 0 };
    const WHITE: Self = LedColor {
        r: 255,
        g: 255,
        b: 255,
    };
}

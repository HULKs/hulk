use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use gilrs::{Gilrs, ev::AxisOrBtn};
use ros_z::prelude::*;
use tokio::time::{MissedTickBehavior, interval};
use tracing::warn;
use types::controller_input::{ControllerAxis, ControllerButton, ControllerInput};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("controller_handler").build().await?;

    let controller_input_pub = node
        .publisher::<ControllerInput>("inputs/controller_input")
        .build()
        .await?;

    let mut gilrs = match Gilrs::new() {
        Ok(gilrs) => gilrs,
        Err(error) => {
            warn!(%error, "failed to initialize controller handler");
            return Ok(());
        }
    };
    let mut ticker = interval(Duration::from_millis(20));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        let controller_input = read_controller_input(&mut gilrs);
        controller_input_pub.publish(&controller_input).await?;
    }
}

fn read_controller_input(gilrs: &mut Gilrs) -> ControllerInput {
    while gilrs.next_event().is_some() {}

    let Some((_, gamepad)) = gilrs.gamepads().next() else {
        return ControllerInput::default();
    };

    let mut input = ControllerInput {
        connected: true,
        device_name: gamepad.name().to_owned(),
        axes: Vec::new(),
        buttons: Vec::new(),
    };

    for (code, data) in gamepad.state().axes() {
        if let Some(AxisOrBtn::Axis(axis)) = gamepad.axis_or_btn_name(code) {
            input.axes.push(ControllerAxis {
                name: format!("{axis:?}"),
                value: data.value(),
            });
        }
    }

    for (code, data) in gamepad.state().buttons() {
        if let Some(AxisOrBtn::Btn(button)) = gamepad.axis_or_btn_name(code) {
            input.buttons.push(ControllerButton {
                name: format!("{button:?}"),
                pressed: data.is_pressed(),
                value: data.value(),
            });
        }
    }

    input
}

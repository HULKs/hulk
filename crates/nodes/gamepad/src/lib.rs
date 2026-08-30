use std::{boxed::Box, future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use ros_z::prelude::*;

mod evdev_input;
pub mod state;

use evdev_input::GamepadReader;
use state::GamepadState;
use types::gamepad::GamepadManualMotion;

pub use state::Parameters;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("gamepad").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("gamepad")?;
    let manual_motion_pub = node
        .publisher::<GamepadManualMotion>("gamepad/manual_motion")
        .build()
        .await?;

    let initial_parameters_snapshot = parameters.snapshot();
    let mut state = GamepadState::new(initial_parameters_snapshot.typed());
    let mut reader = None;
    let mut next_reconnect = node.clock().now();
    let mut last_tick = node.clock().now();
    let mut timer = node.create_timer(Duration::from_millis(20));

    loop {
        timer.tick().await;

        let now = node.clock().now();
        let cycle_duration = now.duration_since(last_tick);
        last_tick = now;
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        if reader.is_none() && now >= next_reconnect {
            match GamepadReader::open(&parameters.device_path) {
                Ok(opened_reader) => reader = Some(opened_reader),
                Err(_) => next_reconnect = now + parameters.reconnect_interval,
            }
        }

        if let Some(open_reader) = &mut reader
            && let Err(error) = open_reader.drain_events(&mut state, parameters)
        {
            tracing::info!(
                target: "gamepad::evdev",
                error = %error,
                reconnect_interval = ?parameters.reconnect_interval,
                "lost gamepad device while reading events"
            );
            reader = None;
            next_reconnect = now + parameters.reconnect_interval;
        }

        let input_is_unavailable = reader.is_none();
        let manual_motion = state.manual_motion(parameters, cycle_duration, input_is_unavailable);
        manual_motion_pub.publish(&manual_motion).await?;
    }
}

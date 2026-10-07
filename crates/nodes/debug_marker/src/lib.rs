use std::{pin::Pin, sync::Arc};

use color_eyre::Result;
use ros_z::context::Context;
use types::controller_input::{Button, ControllerInput};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("debug_marker").build().await?;

    let controller_input_sub = node
        .subscriber::<ControllerInput>("inputs/controller_input")
        .build()
        .await?;

    let mut marker_counter = 0;
    let mut previous_east_pressed = false;

    loop {
        let controller_input = controller_input_sub.recv().await?;

        let east_pressed = controller_input.is_pressed(Button::East);
        if east_pressed != previous_east_pressed && east_pressed {
            log::warn!("===== Marker {} =====", marker_counter);
            marker_counter += 1;
        }
        previous_east_pressed = east_pressed;
    }
}

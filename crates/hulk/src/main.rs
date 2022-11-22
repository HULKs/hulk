use std::sync::Arc;

use anyhow::Context;
use cyclers::run;
use structs::Configuration;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "nao")]
mod nao;
#[cfg(feature = "nao")]
use nao::Interface;

#[cfg(feature = "webots")]
mod webots;
#[cfg(feature = "webots")]
use crate::webots::Interface;

fn main() -> anyhow::Result<()> {
    let keep_running = CancellationToken::new();
    {
        let keep_running = keep_running.clone();
        ctrlc::set_handler(move || {
            keep_running.cancel();
        })?;
    }
    let hardware_interface = Arc::new(
        Interface::new(keep_running.clone()).context("Failed to create hardware interface")?,
    );
    let initial_configuration = Configuration::default();
    run(hardware_interface, initial_configuration, keep_running)
}

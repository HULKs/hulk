use std::sync::Arc;

use cyclers::run;
use hardware::HardwareInterface;
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
    let hardware_interface = Arc::new(Interface::default());
    hardware_interface.print_number(42);
    let initial_configuration = Configuration::default();
    run(hardware_interface, initial_configuration, keep_running)
}

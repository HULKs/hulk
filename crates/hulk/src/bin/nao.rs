use std::{fs::File, sync::Arc};

use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use cyclers::run;
use hulk::{nao, setup_logger};
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;
use types::hardware::Interface;

fn main() -> Result<()> {
    setup_logger(true)?;
    install()?;
    let keep_running = CancellationToken::new();
    ctrlc::set_handler({
        let keep_running = keep_running.clone();
        move || {
            keep_running.cancel();
        }
    })?;
    let file = File::open("etc/configuration/hardware.json")
        .wrap_err("failed to open hardware parameters")?;
    let hardware_parameters = from_reader(file).wrap_err("failed to parse hardware parameters")?;
    let hardware_interface = nao::Interface::new(keep_running.clone(), hardware_parameters)
        .wrap_err("failed to create hardware interface")?;
    let ids = hardware_interface.get_ids();
    run(
        Arc::new(hardware_interface),
        Some("[::]:1337"),
        "etc/configuration",
        ids.body_id,
        ids.head_id,
        keep_running,
    )
}

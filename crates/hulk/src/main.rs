use std::{fmt::Debug, fs::File, path::Path, sync::Arc};

use color_eyre::{eyre::WrapErr, Result};
use cyclers::run;
use parameters::Parameters;
use serde_json::from_reader;
use structs::Configuration;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "nao")]
mod nao;
mod parameters;
#[cfg(feature = "webots")]
mod webots;

fn main() -> Result<()> {
    let keep_running = CancellationToken::new();
    {
        let keep_running = keep_running.clone();
        ctrlc::set_handler(move || {
            keep_running.cancel();
        })?;
    }
    let hardware_parameters = parse_hardware_parameters("etc/configuration")
        .wrap_err("failed to parse hardware parameters")?;
    let hardware_interface = new_hardware_interface(keep_running.clone(), hardware_parameters)?;
    let initial_configuration = Configuration::default();
    run(hardware_interface, initial_configuration, keep_running)
}

fn parse_hardware_parameters(path: impl AsRef<Path> + Debug) -> Result<Parameters> {
    let file = File::open(&path).wrap_err_with(|| format!("failed to open {path:?}"))?;
    from_reader(file).wrap_err_with(|| format!("failed to parse {path:?}"))
}

#[cfg(feature = "nao")]
fn new_hardware_interface(
    _keep_running: CancellationToken,
    parameters: Parameters,
) -> Result<Arc<nao::Interface>> {
    Ok(Arc::new(
        nao::Interface::new(parameters.nao).wrap_err("failed to create hardware interface")?,
    ))
}

#[cfg(feature = "webots")]
fn new_hardware_interface(
    keep_running: CancellationToken,
    _parameters: Parameters,
) -> Result<Arc<webots::Interface>> {
    Ok(Arc::new(webots::Interface::new(keep_running)))
}

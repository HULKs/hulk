use std::{env::args, fmt::Debug, fs::File, io::stdout, path::Path, sync::Arc};

use color_eyre::{eyre::WrapErr, install, Result};
use ctrlc::set_handler;
use cyclers::run;
use parameters::Parameters;
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;
use types::hardware::Interface;

#[cfg(feature = "nao")]
mod nao;
mod network;
mod parameters;
#[cfg(feature = "webots")]
mod webots;

fn setup_logger(is_verbose: bool) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}  {:<18}  {:>5}  {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(if is_verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(stdout())
        .apply()?;
    Ok(())
}

fn main() -> Result<()> {
    setup_logger(true)?;
    install()?;
    let hardware_parameters_path = args()
        .skip(1)
        .next()
        .unwrap_or("etc/parameters/hardware.json".to_string());
    let keep_running = CancellationToken::new();
    {
        let keep_running = keep_running.clone();
        set_handler(move || {
            keep_running.cancel();
        })?;
    }
    let hardware_parameters = cancel_on_error(
        &keep_running,
        parse_hardware_parameters(hardware_parameters_path),
    )
    .wrap_err("failed to parse hardware parameters")?;
    let hardware_interface = cancel_on_error(
        &keep_running,
        new_hardware_interface(keep_running.clone(), hardware_parameters),
    )
    .wrap_err("failed to create hardware interface")?;
    let ids = hardware_interface.get_ids();
    let paths = hardware_interface.get_paths();
    run(
        hardware_interface,
        Some("[::]:1337"),
        paths.parameters,
        ids.body_id,
        ids.head_id,
        keep_running,
    )
}

fn cancel_on_error<T, E>(keep_running: &CancellationToken, result: Result<T, E>) -> Result<T, E> {
    if result.is_err() {
        keep_running.cancel();
    }
    result
}

fn parse_hardware_parameters(path: impl AsRef<Path> + Debug) -> Result<Parameters> {
    let file = File::open(&path).wrap_err_with(|| format!("failed to open {path:?}"))?;
    from_reader(file).wrap_err_with(|| format!("failed to parse {path:?}"))
}

#[cfg(feature = "nao")]
fn new_hardware_interface(
    keep_running: CancellationToken,
    parameters: Parameters,
) -> Result<Arc<nao::Interface>> {
    Ok(Arc::new(nao::Interface::new(keep_running, parameters.nao)?))
}

#[cfg(all(feature = "webots", not(feature = "nao")))]
fn new_hardware_interface(
    keep_running: CancellationToken,
    parameters: Parameters,
) -> Result<Arc<webots::Interface>> {
    Ok(Arc::new(webots::Interface::new(
        keep_running,
        parameters.webots,
    )?))
}

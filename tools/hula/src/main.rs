use std::{
    io::stdout,
    sync::{Arc, Mutex},
};

use clap::Parser;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use log::debug;
use systemd::daemon::{notify, STATE_READY};

use termination::TerminationRequest;

use crate::proxy::Proxy;

mod aliveness;
mod beacon;
mod lola;
mod proxy;
mod service_manager;
mod systemd1;
mod termination;

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

#[derive(Parser, Debug)]
#[clap(
    name = "hula",
    about = "Forwards messages between LoLA and other applications, exports metrics over DBus"
)]
struct Arguments {
    /// Log with Debug log level
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let matches = Arguments::parse();
    setup_logger(matches.verbose).wrap_err("failed to setup logger")?;

    let termination_request = TerminationRequest::default();
    {
        let termination_request = termination_request.clone();
        ctrlc::set_handler(move || {
            termination_request.terminate();
        })
        .wrap_err("failed to set signal handler")?;
    }

    let robot_configuration = Arc::new(Mutex::default());
    let battery = Arc::new(Mutex::default());

    let proxy = Proxy::start(
        termination_request.clone(),
        robot_configuration.clone(),
        battery.clone(),
    )
    .wrap_err("failed to start proxy")?;

    let aliveness =
        match aliveness::start(termination_request.clone(), robot_configuration, battery) {
            Ok(service) => service,
            Err(error) => {
                termination_request.terminate();
                proxy.join().unwrap();
                bail!("failed to start aliveness: {error}");
            }
        };

    notify(false, [(STATE_READY, "1")].iter())
        .wrap_err("failed to contact SystemD for ready notification")?;

    debug!("Waiting for termination request...");
    termination_request.wait();
    debug!("Got termination request, initiating shutdown...");

    match (proxy.join(), aliveness.join().unwrap()) {
        (Ok(_), Ok(_)) => Ok(()),
        (Ok(_), Err(error)) | (Err(error), Ok(_)) => Err(error),
        (Err(proxy_error), Err(aliveness_error)) => {
            Err(eyre!("PANIC {proxy_error}, {aliveness_error}"))
        }
    }
}

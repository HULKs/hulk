use std::{
    io::stdout,
    path::Path,
    sync::{Arc, Mutex},
};

use clap::{Arg, ArgAction, Command};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use log::{debug, error};
use systemd::daemon::{notify, STATE_READY};

use termination::TerminationRequest;

use crate::{aliveness::Aliveness, proxy::Proxy};

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

fn main() -> Result<()> {
    let app = Command::new("proxy")
        .about("Forwards messages between LoLA and other applications, exports metrics over DBus")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Log with DEBUG log level")
                .action(ArgAction::SetTrue),
        );

    let matches = app.get_matches();

    setup_logger(matches.get_flag("verbose")).wrap_err("failed to setup logger")?;

    let termination_request = TerminationRequest::default();
    let termination_request_for_handler = termination_request.clone();
    ctrlc::set_handler(move || {
        debug!("Got signal, will terminate...");
        termination_request_for_handler.terminate();
    })
    .wrap_err("failed to set signal handler")?;

    let robot_configuration = Arc::new(Mutex::default());
    let battery = Arc::new(Mutex::default());

    let proxy = Proxy::start(
        termination_request.clone(),
        robot_configuration.clone(),
        battery.clone(),
    )
    .wrap_err("failed to start proxy")?;
    let disable_aliveness = Path::new("/home/nao/disable_aliveness").exists();
    let aliveness = match disable_aliveness {
        true => None,
        false => Some(
            match Aliveness::start(termination_request.clone(), robot_configuration, battery) {
                Ok(aliveness) => aliveness,
                Err(error) => {
                    termination_request.terminate();
                    if let Err(error) = proxy.join() {
                        error!("Failed to join proxy: {:?}", error);
                    }
                    bail!("failed to start aliveness: {:?}", error);
                }
            },
        ),
    };

    notify(false, [(STATE_READY, "1")].iter())
        .wrap_err("failed to contact SystemD for ready notification")?;

    debug!("Waiting for termination request...");
    termination_request.wait();
    debug!("Got termination request, initiating shutdown...");

    let proxy_stop_result = proxy.join();
    let aliveness_stop_result = aliveness.map(|aliveness| aliveness.join());

    proxy_stop_result?;
    if let Some(aliveness_stop_result) = aliveness_stop_result {
        aliveness_stop_result?;
    }

    Ok(())
}

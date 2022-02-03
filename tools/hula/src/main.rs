use std::io::stdout;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use log::{debug, error};
use termination::TerminationRequest;

use crate::aliveness::Aliveness;
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

fn main() -> Result<()> {
    let app = App::new("proxy")
        .about("Forwards messages between LoLA and other applications, exports metrics over DBus")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Log with DEBUG log level"),
        );

    let matches = app.get_matches();

    setup_logger(matches.is_present("verbose")).context("Failed to setup logger")?;

    let termination_request = TerminationRequest::default();
    let termination_request_for_handler = termination_request.clone();
    ctrlc::set_handler(move || {
        debug!("Got signal, will terminate...");
        termination_request_for_handler.terminate();
    })
    .context("Failed to set signal handler")?;

    let robot_configuration = Arc::new(Mutex::default());
    let battery = Arc::new(Mutex::default());

    let proxy = Proxy::start(
        termination_request.clone(),
        robot_configuration.clone(),
        battery.clone(),
    )
    .context("Failed to start proxy")?;
    let aliveness =
        match Aliveness::start(termination_request.clone(), robot_configuration, battery) {
            Ok(aliveness) => aliveness,
            Err(error) => {
                termination_request.terminate();
                if let Err(error) = proxy.join() {
                    error!("Failed to join proxy: {:?}", error);
                }
                bail!("Failed to start aliveness: {:?}", error);
            }
        };

    debug!("Waiting for termination request...");
    termination_request.wait();
    debug!("Got termination request, initiating shutdown...");

    let proxy_stop_result = proxy.join();
    let dbus_stop_result = aliveness.join();

    proxy_stop_result?;
    dbus_stop_result?;

    Ok(())
}

use std::sync::{Arc, Mutex};

use clap::Parser;
use color_eyre::eyre::{Result, WrapErr};
use hula_types::{Battery, RobotConfiguration};
use log::{debug, LevelFilter};
use systemd::daemon::{notify, STATE_READY};

use crate::{dbus::serve_dbus, proxy::Proxy};

mod dbus;
mod idle;
mod proxy;

#[derive(Parser, Debug)]
#[clap(
    name = "hula",
    about = "Forwards messages between LoLA and other applications"
)]
struct Arguments {
    /// Log with Debug log level
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Default)]
pub struct SharedState {
    pub battery: Option<Battery>,
    pub configuration: Option<RobotConfiguration>,
}

fn main() -> Result<()> {
    let matches = Arguments::parse();
    env_logger::builder()
        .filter(
            None,
            if matches.verbose {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            },
        )
        .init();

    let shared_state = Arc::new(Mutex::new(SharedState::default()));
    let _handle = serve_dbus(shared_state.clone());

    let proxy = Proxy::initialize(shared_state).wrap_err("failed to initialize proxy")?;
    notify(false, [(STATE_READY, "1")].iter())
        .wrap_err("failed to contact SystemD for ready notification")?;
    debug!("Initialized Proxy. HuLA ready");
    proxy.run()
}

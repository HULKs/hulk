use std::io::stdout;

use clap::Parser;
use color_eyre::eyre::{Result, WrapErr};
use log::debug;
use systemd::daemon::{notify, STATE_READY};

use crate::proxy::Proxy;

mod control_frame;
mod idle;
mod listener;
mod lola;
mod proxy;
mod robot_state;

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
    about = "Forwards messages between LoLA and other applications"
)]
struct Arguments {
    /// Log with Debug log level
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let matches = Arguments::parse();
    setup_logger(matches.verbose).wrap_err("failed to setup logger")?;

    let proxy = Proxy::initialize().wrap_err("failed to initialize proxy")?;
    notify(false, [(STATE_READY, "1")].iter())
        .wrap_err("failed to contact SystemD for ready notification")?;
    debug!("Initialized Proxy. HuLA ready");
    proxy.run()
}

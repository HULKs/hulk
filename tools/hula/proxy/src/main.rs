use clap::Parser;
use color_eyre::eyre::{Result, WrapErr};
use log::{debug, LevelFilter};
use systemd::daemon::{notify, STATE_READY};

use crate::proxy::Proxy;

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

    let proxy = Proxy::initialize().wrap_err("failed to initialize proxy")?;
    notify(false, [(STATE_READY, "1")].iter())
        .wrap_err("failed to contact SystemD for ready notification")?;
    debug!("Initialized Proxy. HuLA ready");
    proxy.run()
}

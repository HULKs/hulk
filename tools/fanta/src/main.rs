use std::str::FromStr;

use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;

use crate::{
    communication::{Connection, CyclerOutput},
    logging::setup_logger,
};

mod communication;
mod logging;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CommandlineArguments {
    #[clap(short, long, default_value = "localhost")]
    address: String,
    path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger()?;

    let arguments = CommandlineArguments::parse();
    let output_to_subscribe = CyclerOutput::from_str(&arguments.path)?;
    let connection = Connection::connect(&format!("ws://{}:1337", arguments.address)).await?;
    let (output_sender, mut output_receiver) = mpsc::channel(1);
    connection
        .subscribe(output_to_subscribe, output_sender)
        .await?;
    while let Some(output) = output_receiver.recv().await {
        println!("{output:#}")
    }
    Ok(())
}

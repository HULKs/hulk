use std::str::FromStr;

use clap::Parser;
use color_eyre::{eyre::bail, Result};
use communication::{Communication, CyclerOutput, SubscriberMessage};
use log::{error, info};

use crate::logging::setup_logger;

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
    let communication = Communication::new(Some(format!("ws://{}:1337", arguments.address)), true);
    let (_uuid, mut receiver) = communication.subscribe_output(output_to_subscribe).await;
    while let Some(message) = receiver.recv().await {
        match message {
            SubscriberMessage::Update { value } => println!("{value:#}"),
            SubscriberMessage::SubscriptionSuccess => info!("Successfully subscribed"),
            SubscriberMessage::SubscriptionFailure { info } => {
                error!("Failed to subscribe: {info:?}");
                break;
            }
            SubscriberMessage::UpdateImage { .. } => bail!("Cannot print Image data"),
        }
    }
    Ok(())
}

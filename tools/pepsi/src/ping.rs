use std::time::Duration;

use clap::Args;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use nao::Nao;

use crate::{
    parsers::NaoAddress,
    progress_indicator::{ProgressIndicator, TaskMessage},
};

#[derive(Args)]
pub struct Arguments {
    /// Number of ping retries
    #[arg(long, default_value = "1")]
    pub retries: usize,
    /// Time after which ping is aborted
    #[arg(long, default_value = "2.0")]
    pub timeout: f32,
    /// The NAOs to ping to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn ping(arguments: Arguments) {
    let multi_progress = ProgressIndicator::new();

    arguments
        .naos
        .iter()
        .map(|nao_address| (nao_address, multi_progress.task(nao_address.to_string())))
        .map(|(nao_address, progress)| async move {
            progress.set_message("Pinging NAO...");

            let ping_state = Nao::try_new_with_ping_and_arguments(
                nao_address.ip,
                arguments.retries,
                Duration::from_secs_f32(arguments.timeout),
            )
            .await
            .map(|_| TaskMessage::EmptyMessage);
            progress.finish_with(ping_state);
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;
}

use std::time::Duration;

use clap::Args;
use color_eyre::{Report, owo_colors::OwoColorize};
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use nao::Nao;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// Number of ping retries
    #[arg(long, default_value = "1")]
    pub retries: usize,
    /// Time after which ping is aborted
    #[arg(long, default_value = "2.0")]
    pub timeout: f32,
    /// The NAOs to upload to e.g. 20w or 10.1.24.22
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
            let nao = Nao::new(nao_address.ip);
            progress.set_message(format!("Pinging {}", nao_address.short().bold()));
            match nao
                .is_reachable(
                    arguments.retries,
                    Duration::from_secs_f32(arguments.timeout),
                )
                .await
            {
                true => progress.finish_with_success(format!("{} reachable", nao_address.short())),
                false => {
                    progress.finish_with_error(Report::msg(format!("{} unreachable", nao_address.short())))
                }
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;
}

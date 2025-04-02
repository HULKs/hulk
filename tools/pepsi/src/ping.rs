use std::time::Duration;

use clap::Args;

use argument_parsers::NaoAddress;
use nao::Nao;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Timeout in seconds after which ping is aborted
    #[arg(long, default_value = "2")]
    pub timeout: u64,
    /// The NAOs to ping to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn ping(arguments: Arguments) {
    ProgressIndicator::map_tasks(
        arguments.naos,
        "Pinging NAO...",
        |nao_address, _progress_bar| async move {
            Nao::try_new_with_ping_and_arguments(
                nao_address.ip,
                Duration::from_secs(arguments.timeout),
            )
            .await
            .map(|_| ())
        },
    )
    .await;
}

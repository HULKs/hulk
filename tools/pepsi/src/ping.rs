use clap::Args;
use nao::Nao;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// Time in seconds after which ping is aborted
    #[arg(long, default_value = "2")]
    pub timeout_seconds: u32,
    /// The NAOs to ping to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn ping(arguments: Arguments) {
    ProgressIndicator::map_tasks(
        arguments.naos,
        "Pinging NAO...",
        |nao_address, _progress_bar| async move {
            Nao::try_new_with_ping_and_arguments(nao_address.ip, arguments.timeout_seconds)
                .await
                .map(|_| ())
        },
    )
    .await;
}

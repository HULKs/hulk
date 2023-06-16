use std::time::Duration;

use clap::Args;
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
    /// The NAOs to ping to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn ping(arguments: Arguments) {
    ProgressIndicator::map_tasks(arguments.naos, "Pinging NAO...", |nao_address| async move {
        Nao::try_new_with_ping_and_arguments(
            nao_address.ip,
            arguments.retries,
            Duration::from_secs_f32(arguments.timeout),
        )
        .await
        .map(|_| ())
    })
    .await;
}

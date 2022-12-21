use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Args,
};
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::{Nao, SystemctlAction};

use crate::{
    parsers::{parse_systemctl_action, NaoAddress, SYSTEMCTL_ACTION_POSSIBLE_VALUES},
    progress_indicator::ProgressIndicator,
};

#[derive(Args)]
pub struct Arguments {
    /// The systemctl action to execute for the HULK service
    #[arg(
        value_parser = PossibleValuesParser::new(SYSTEMCTL_ACTION_POSSIBLE_VALUES)
            .map(|s| parse_systemctl_action(&s).unwrap()))
    ]
    pub action: SystemctlAction,
    /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn hulk(arguments: Arguments) -> Result<()> {
    let multi_progress = ProgressIndicator::new();

    let tasks = arguments.naos.into_iter().map(|nao_address| {
        let multi_progress = multi_progress.clone();
        async move {
            let progress = multi_progress.task(nao_address.to_string());
            let nao = Nao::new(nao_address.ip);

            progress.set_message("Executing systemctl hulk...");

            progress.finish_with(
                nao.execute_systemctl(arguments.action, "hulk")
                    .await
                    .wrap_err_with(|| format!("failed to execute systemctl hulk on {nao_address}")),
            )
        }
    });

    join_all(tasks).await;

    Ok(())
}

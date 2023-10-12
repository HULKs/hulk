use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Args,
};
use color_eyre::{eyre::WrapErr, Result};

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
    ProgressIndicator::map_tasks(
        arguments.naos,
        "Executing systemctl hulk...",
        |nao_address, _progress_bar| async move {
            let nao = Nao::try_new_with_ping(nao_address.ip).await?;
            nao.execute_systemctl(arguments.action, "hulk")
                .await
                .wrap_err_with(|| format!("failed to execute systemctl hulk on {nao_address}"))
        },
    )
    .await;

    Ok(())
}

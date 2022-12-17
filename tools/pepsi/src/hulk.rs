use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Args,
};
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::{Nao, SystemctlAction};

use crate::{
    parsers::{parse_systemctl_action, NaoAddress, SYSTEMCTL_ACTION_POSSIBLE_VALUES},
    results::gather_results,
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
    let tasks = arguments.naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.ip);
        nao.execute_systemctl(arguments.action, "hulk")
            .await
            .wrap_err_with(|| format!("failed to execute systemctl hulk on {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "failed to execute some systemctl hulk tasks")?;

    Ok(())
}

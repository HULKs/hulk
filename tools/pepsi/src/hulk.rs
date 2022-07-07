use anyhow::Context;
use futures::future::join_all;
use nao::{Nao, SystemctlAction};
use repository::Repository;
use structopt::StructOpt;

use crate::{
    parsers::{parse_systemctl_action, NaoAddress, SYSTEMCTL_ACTION_POSSIBLE_VALUES},
    results::gather_results,
};

#[derive(StructOpt)]
pub struct Arguments {
    /// The systemctl action to execute for the HULK service
    #[structopt(possible_values = SYSTEMCTL_ACTION_POSSIBLE_VALUES, parse(try_from_str = parse_systemctl_action))]
    pub action: SystemctlAction,
    /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
    #[structopt(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn hulk(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let tasks = arguments.naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

        nao.execute_systemctl(arguments.action, "hulk")
            .await
            .with_context(|| format!("Failed to execute systemctl hulk on {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some systemctl hulk tasks")?;

    Ok(())
}

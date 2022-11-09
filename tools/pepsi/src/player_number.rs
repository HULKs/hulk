use anyhow::Context;
use clap::Args;
use futures::future::join_all;

use repository::Repository;

use crate::{parsers::NaoNumberPlayerAssignment, results::gather_results};

#[derive(Args)]
pub struct Arguments {
    /// The assignments to change e.g. 20:2 or 32:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<NaoNumberPlayerAssignment>,
}

pub async fn player_number(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let hardware_ids = repository
        .get_hardware_ids()
        .await
        .context("Failed to get hardware IDs")?;

    let tasks = arguments.assignments.into_iter().map(|assignment| {
        let head_id = &hardware_ids[&assignment.nao_number.number].head_id;
        async move {
            repository
                .set_player_number(head_id, assignment.player_number)
                .await
                .with_context(|| format!("Failed to set player number for {assignment:?}"))
        }
    });

    let results = join_all(tasks).await;
    gather_results(
        results,
        "Failed to execute some player number setting tasks",
    )?;

    Ok(())
}

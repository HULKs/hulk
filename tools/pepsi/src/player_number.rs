use std::collections::HashSet;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use repository::Repository;

use crate::{parsers::NaoNumberPlayerAssignment, results::gather_results};

#[derive(Args)]
pub struct Arguments {
    /// The assignments to change e.g. 20:2 or 32:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<NaoNumberPlayerAssignment>,
}

pub async fn player_number(arguments: Arguments, repository: &Repository) -> Result<()> {
    let hardware_ids = repository
        .get_hardware_ids()
        .await
        .wrap_err("failed to get hardware IDs")?;

    // Check if two NaoNumbers are assigned to the same PlayerNumber
    // or if a NaoNumber is assigned to multiple PlayerNumbers
    let mut existing_player_numbers = HashSet::new();
    let mut existing_nao_numbers = HashSet::new();

    if arguments.assignments.iter().any(
        |NaoNumberPlayerAssignment {
             nao_number,
             player_number,
         }| {
            !existing_nao_numbers.insert(nao_number)
                || !existing_player_numbers.insert(player_number)
        },
    ) {
        bail!("Duplication in NAO to player number assignments")
    }

    let tasks = arguments.assignments.into_iter().map(|assignment| {
        let head_id = &hardware_ids[&assignment.nao_number.number].head_id;
        async move {
            repository
                .set_player_number(head_id, assignment.player_number)
                .await
                .wrap_err_with(|| format!("failed to set player number for {assignment:?}"))
        }
    });

    let results = join_all(tasks).await;
    gather_results(
        results,
        "failed to execute some player number setting tasks",
    )?;

    Ok(())
}

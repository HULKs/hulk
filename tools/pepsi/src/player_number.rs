use std::collections::HashSet;

use clap::Args;
use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};

use argument_parsers::NaoNumberPlayerAssignment;
use repository::Repository;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// The assignments to change e.g. 20:2 or 32:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<NaoNumberPlayerAssignment>,
}

pub async fn player_number(arguments: Arguments, repository: &Repository) -> Result<()> {
    let team = repository
        .read_team_configuration()
        .await
        .wrap_err("failed to get team configuration")?;

    check_for_duplication(&arguments.assignments)?;

    // reborrows the team to avoid moving it into the closure
    let naos = &team.naos;

    ProgressIndicator::map_tasks(
        arguments.assignments,
        "Setting player number...",
        |assignment, _progress_bar| async move {
            let number = assignment.nao_number.number;
            let nao = naos
                .iter()
                .find(|nao| nao.number == number)
                .ok_or_else(|| eyre!("NAO with Hardware ID {number} does not exist"))?;
            repository
                .configure_player_number(&nao.head_id, assignment.player_number)
                .await
                .wrap_err_with(|| format!("failed to set player number for {assignment}"))
        },
    )
    .await;

    Ok(())
}

fn check_for_duplication(assignments: &[NaoNumberPlayerAssignment]) -> Result<()> {
    // Check if two NaoNumbers are assigned to the same PlayerNumber
    // or if a NaoNumber is assigned to multiple PlayerNumbers
    let mut existing_player_numbers = HashSet::new();
    let mut existing_nao_numbers = HashSet::new();

    if assignments.iter().any(
        |NaoNumberPlayerAssignment {
             nao_number,
             player_number,
         }| {
            !existing_nao_numbers.insert(nao_number)
                || !existing_player_numbers.insert(player_number)
        },
    ) {
        bail!("duplication in NAO to player number assignments")
    }
    Ok(())
}

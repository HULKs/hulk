use std::collections::HashSet;

use clap::Args;
use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};

use argument_parsers::NaoNumberJerseyAssignment;
use repository::Repository;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// The assignments to change e.g. 20:2 or 32:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<NaoNumberJerseyAssignment>,
}

pub async fn jersey_number(arguments: Arguments, repository: &Repository) -> Result<()> {
    let hardware_ids = repository
        .get_hardware_ids()
        .await
        .wrap_err("failed to get hardware IDs")?;

    // Check if two NaoNumbers are assigned to the same jersey number
    // or if a NaoNumber is assigned to multiple jersey numberss
    let mut existing_jersey_numbers = HashSet::new();
    let mut existing_nao_numbers = HashSet::new();

    if arguments.assignments.iter().any(
        |NaoNumberJerseyAssignment {
             nao_number,
             jersey_number,
         }| {
            !existing_nao_numbers.insert(nao_number)
                || !existing_jersey_numbers.insert(jersey_number)
        },
    ) {
        bail!("Duplication in NAO to player number assignments")
    }
    let hardware_ids = &hardware_ids;
    ProgressIndicator::map_tasks(
        arguments.assignments,
        "Setting player number...",
        |assignment, _progress_bar| async move {
            let number = assignment.nao_number.number;
            let hardware_id = hardware_ids
                .get(&number)
                .ok_or_else(|| eyre!("NAO with Hardware ID {number} does not exist"))?;
            repository
                .set_jersey_number(&hardware_id.head_id, assignment.jersey_number)
                .await
                .wrap_err_with(|| format!("failed to set player number for {assignment}"))
        },
    )
    .await;

    Ok(())
}

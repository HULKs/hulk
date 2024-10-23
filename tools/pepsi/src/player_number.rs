use std::{collections::HashSet, path::Path};

use clap::Args;
use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};

use argument_parsers::NaoNumberPlayerAssignment;
use repository::{hardware_ids::get_hardware_ids, player_number::set_player_number};

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// The assignments to change e.g. 20:2 or 32:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<NaoNumberPlayerAssignment>,
}

pub async fn player_number(arguments: Arguments, repository_root: impl AsRef<Path>) -> Result<()> {
    let repository_root = repository_root.as_ref();
    let hardware_ids = get_hardware_ids(repository_root)
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
    let hardware_ids = &hardware_ids;
    ProgressIndicator::map_tasks(
        arguments.assignments,
        "Setting player number...",
        |assignment, _progress_bar| async move {
            let number = assignment.nao_number.number;
            let hardware_id = hardware_ids
                .get(&number)
                .ok_or_else(|| eyre!("NAO with Hardware ID {number} does not exist"))?;
            set_player_number(
                &hardware_id.head_id,
                assignment.player_number,
                repository_root,
            )
            .await
            .wrap_err_with(|| format!("failed to set player number for {assignment}"))
        },
    )
    .await;

    Ok(())
}

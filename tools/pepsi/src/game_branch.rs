use clap::Args;
use color_eyre::{Result, eyre::WrapErr};
use parameters::directory::LocationTarget;
use repository::Repository;
use tokio::io::{AsyncBufReadExt, BufReader, stdin};

use crate::{
    deploy_config::DeployConfig,
    git::{create_and_switch_to_branch, create_commit, merge_squash, reset_to_head},
    player_number::{Arguments as PlayerNumberArguments, player_number},
};

#[derive(Args)]
pub struct Arguments {
    /// Create the game branch even if it already exists
    #[arg(short, long)]
    pub force: bool,
}

pub async fn game_branch(arguments: Arguments, repository: &Repository) -> Result<()> {
    let config = DeployConfig::read_from_file(repository)
        .await
        .wrap_err("failed to read deploy config from file")?;

    let branch_name = config.branch_name();
    create_and_switch_to_branch(&branch_name, &config.base, arguments.force)
        .await
        .wrap_err("failed to create and switch to branch")?;

    'branches: for branch in &config.branches {
        let status = merge_squash(&branch.to_string()).await;

        if status.is_err() {
            eprintln!("Automatic merge failed.");
            let skip_prompt = format!("Do you want to skip deploying '{branch}'?");

            loop {
                let skip = confirmation_prompt(&skip_prompt)
                    .await
                    .wrap_err("failed to create confirmation prompt")?;

                if skip {
                    reset_to_head()
                        .await
                        .wrap_err("failed to reset repository to HEAD")?;

                    continue 'branches;
                } else {
                    eprintln!("Please resolve all conflicts now.");

                    let conflicts_resolved =
                        confirmation_prompt("Were you able to resolve the conflicts?")
                            .await
                            .wrap_err("failed to create confirmation prompt")?;

                    if conflicts_resolved {
                        break;
                    }
                }
            }
        }

        create_commit(&format!("merge-squash {branch}"))
            .await
            .wrap_err("failed to create commit")?;
    }

    configure_repository(repository, config).await?;
    create_commit("Add player number assigments and framework config")
        .await
        .wrap_err("failed to create commit")?;

    Ok(())
}

async fn configure_repository(repository: &Repository, config: DeployConfig) -> Result<()> {
    repository
        .set_location(LocationTarget::Default, &config.location)
        .await
        .wrap_err_with(|| format!("failed to set location to {}", config.location))?;

    player_number(
        PlayerNumberArguments {
            assignments: config
                .assignments
                .iter()
                .copied()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        },
        repository,
    )
    .await
    .wrap_err("failed to set player numbers")?;

    Ok(())
}

async fn confirmation_prompt(message: &str) -> Result<bool> {
    let reader = BufReader::new(stdin());

    let mut lines = reader.lines();

    loop {
        eprint!("{message} [y/n] ");

        if let Some(line) = lines
            .next_line()
            .await
            .wrap_err("failed to get next line")?
        {
            match line {
                line if line == "y" => return Ok(true),
                line if line == "n" => return Ok(false),
                _ => {}
            }
        }
    }
}

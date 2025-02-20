use std::collections::HashMap;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use repository::Repository;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    process::Command,
};

use crate::{
    deploy_config::{Branch, DeployConfig},
    git::{create_and_switch_to_branch, create_commit, reset_to_head},
    player_number::{player_number, Arguments as PlayerNumberArguments},
    recording::parse_key_value,
};

#[derive(Args)]
pub struct Arguments {
    /// Enable communication, communication is disabled by default
    #[arg(long)]
    pub with_communication: bool,
    /// Intervals between cycle recordings, e.g. Control=1,VisionTop=30 to record every cycle in Control
    /// and one out of every 30 in VisionTop. Set to 0 or don't specify to disable recording for a cycler.
    #[arg(
        long,
        value_delimiter=',',
        value_parser = parse_key_value::<String, usize>,
        default_value = "Control=1,VisionTop=30,VisionBottom=30,SplNetwork=1",
    )]
    pub recording_intervals: Vec<(String, usize)>,
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

    configure_repository(repository, arguments, &config).await?;
    create_commit("Add player number assigments and framework config")
        .await
        .wrap_err("failed to create commit")?;

    'branches: for Branch { remote, branch } in &config.branches {
        let status = Command::new(repository.root.join("scripts/deploy"))
            .arg(remote)
            .arg(branch)
            .status()
            .await
            .wrap_err("failed to execute deploy script")?;

        if !status.success() {
            eprintln!("Automatic merge failed.");
            let skip_prompt = format!("Do you want to skip deploying '{remote}/{branch}'?");

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

        create_commit(&format!("{remote}/{branch}"))
            .await
            .wrap_err("failed to create commit")?;
    }

    Ok(())
}

async fn configure_repository(
    repository: &Repository,
    arguments: Arguments,
    config: &DeployConfig,
) -> Result<()> {
    repository
        .configure_recording_intervals(HashMap::from_iter(arguments.recording_intervals))
        .await
        .wrap_err("failed to apply recording settings")?;

    repository
        .set_location("nao", &config.location)
        .await
        .wrap_err_with(|| format!("failed to set location for nao to {}", config.location))?;

    repository
        .configure_communication(arguments.with_communication)
        .await
        .wrap_err("failed to set communication")?;

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

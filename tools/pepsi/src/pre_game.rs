use std::{collections::HashMap, path::Path, str::FromStr};

use clap::Args;
use color_eyre::{
    eyre::{bail, ContextCompat, WrapErr},
    Result,
};

use argument_parsers::{parse_network, NaoAddress, NaoAddressPlayerAssignment};
use indicatif::ProgressBar;
use nao::{Nao, Network, SystemctlAction};
use repository::{upload::get_hulk_binary, Repository};
use serde::{de::Error as DeserializeError, Deserialize, Deserializer};
use tempfile::tempdir;
use tokio::{
    fs::read_to_string,
    io::{stdin, AsyncBufReadExt, BufReader},
    process::Command,
};
use toml::{from_str, value::Datetime};
use tracing::warn;

use crate::{
    cargo::{self, build, cargo, environment::EnvironmentArguments, CargoCommand},
    git::{create_and_switch_to_branch, create_commit, reset_to_head, switch_to_branch},
    player_number::{player_number, Arguments as PlayerNumberArguments},
    progress_indicator::ProgressIndicator,
    recording::parse_key_value,
};

#[derive(Args)]
#[group(skip)]
pub struct Arguments {
    #[command(flatten)]
    pub pre_game: PreGameArguments,
    #[command(flatten)]
    pub environment: EnvironmentArguments,
    #[command(flatten, next_help_heading = "Cargo Options")]
    pub build: build::Arguments,
}

#[derive(Args)]
pub struct PreGameArguments {
    /// Do not build before uploading
    #[arg(long)]
    pub no_build: bool,
    /// Do not restart HULK nor HULA service after uploading
    #[arg(long)]
    pub no_restart: bool,
    /// Do not remove existing remote files during uploading
    #[arg(long)]
    pub no_clean: bool,
    /// Skip the OS version check
    #[arg(long)]
    pub skip_os_check: bool,
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
    /// Prepare everything for the upload without performing the actual one
    #[arg(long)]
    pub prepare: bool,
}

#[derive(Deserialize)]
pub struct Config {
    date: Datetime,
    opponent: String,
    location: String,
    #[serde(deserialize_with = "deserialize_network")]
    wifi: Network,
    base: String,
    branches: Vec<Branch>,
    #[serde(deserialize_with = "deserialize_assignments")]
    assignments: Vec<NaoAddressPlayerAssignment>,
}

impl Config {
    async fn deploy(&self, repository: &Repository) -> Result<()> {
        let branch_name = self
            .branch_name()
            .await
            .wrap_err("failed to get branch name")?;

        if create_and_switch_to_branch(&branch_name, &self.base)
            .await
            .is_ok()
        {
            'branches: for Branch { remote, branch } in &self.branches {
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
        } else {
            warn!("Branch already exists, switching to it instead");

            switch_to_branch(&branch_name)
                .await
                .wrap_err("failed to switch to branch")?;
        }

        Ok(())
    }

    async fn branch_name(&self) -> Result<String> {
        let date = self.date.date.wrap_err("missing date")?;

        Ok(format!(
            "{date}-HULKs-vs-{opponent}",
            opponent = self.opponent
        ))
    }
}

fn deserialize_network<'de, D, E>(deserializer: D) -> Result<Network, E>
where
    D: Deserializer<'de>,
    E: DeserializeError + From<D::Error>,
{
    let network = String::deserialize(deserializer)?;

    parse_network(&network).map_err(|error| E::custom(format!("{error:?}")))
}

fn deserialize_assignments<'de, D, E>(deserializer: D) -> Result<Vec<NaoAddressPlayerAssignment>, E>
where
    D: Deserializer<'de>,
    E: DeserializeError + From<D::Error>,
{
    let assignments: Vec<String> = Vec::deserialize(deserializer)?;

    assignments
        .into_iter()
        .map(|assignment| {
            NaoAddressPlayerAssignment::from_str(&assignment)
                .map_err(|error| E::custom(format!("{error:?}")))
        })
        .collect()
}

pub struct Branch {
    remote: String,
    branch: String,
}

impl<'de> Deserialize<'de> for Branch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let branch = String::deserialize(deserializer)?;

        let (remote, branch) = branch.split_once("/").ok_or_else(|| {
            D::Error::custom("deploy target has to follow the format 'remote/branch'")
        })?;

        Ok(Self {
            remote: remote.to_owned(),
            branch: branch.to_owned(),
        })
    }
}

pub async fn pre_game(arguments: Arguments, repository: &Repository) -> Result<()> {
    let deploy_config = read_to_string(repository.root.join("deploy.toml"))
        .await
        .wrap_err("failed to read deploy.toml")?;
    let config: Config =
        from_str(&deploy_config).wrap_err("could not deserialize config from deploy.toml")?;

    config
        .deploy(repository)
        .await
        .wrap_err("failed to deploy config")?;

    let naos: Vec<_> = config
        .assignments
        .iter()
        .map(|assignment| assignment.nao_address)
        .collect();

    repository
        .configure_recording_intervals(HashMap::from_iter(
            arguments.pre_game.recording_intervals.clone(),
        ))
        .await
        .wrap_err("failed to apply recording settings")?;

    repository
        .set_location("nao", &config.location)
        .await
        .wrap_err_with(|| format!("failed setting location for nao to {}", config.location))?;

    repository
        .configure_communication(arguments.pre_game.with_communication)
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

    let upload_directory = tempdir().wrap_err("failed to get temporary directory")?;
    let hulk_binary = get_hulk_binary(arguments.build.profile());

    let cargo_arguments = cargo::Arguments {
        manifest: Some(
            repository
                .root
                .join("crates/hulk_nao/Cargo.toml")
                .into_os_string(),
        ),
        environment: arguments.environment,
        cargo: arguments.build,
    };

    if !arguments.pre_game.no_build {
        cargo(cargo_arguments, repository, &[&hulk_binary])
            .await
            .wrap_err("failed to build")?;
    }
    if arguments.pre_game.prepare {
        eprintln!("Preparation complete, skipping the rest");
        return Ok(());
    }

    repository
        .populate_upload_directory(&upload_directory, hulk_binary)
        .await
        .wrap_err("failed to populate upload directory")?;

    let arguments = &arguments.pre_game;
    let config = &config;
    let upload_directory = &upload_directory;

    ProgressIndicator::map_tasks(
        &naos,
        "Executing pregame tasks",
        |nao_address, progress_bar| async move {
            setup_nao(
                nao_address,
                upload_directory,
                arguments,
                config,
                progress_bar,
                repository,
            )
            .await
        },
    )
    .await;

    Ok(())
}

async fn setup_nao(
    nao_address: &NaoAddress,
    upload_directory: impl AsRef<Path>,
    arguments: &PreGameArguments,
    config: &Config,
    progress: ProgressBar,
    repository: &Repository,
) -> Result<()> {
    progress.set_message("Pinging NAO...");
    let nao = Nao::try_new_with_ping(nao_address.ip).await?;

    if !arguments.skip_os_check {
        progress.set_message("Checking OS version...");
        let nao_os_version = nao
            .get_os_version()
            .await
            .wrap_err_with(|| format!("failed to get OS version of {nao_address}"))?;
        let expected_os_version = repository
            .read_os_version()
            .await
            .wrap_err("failed to get configured OS version")?;
        if nao_os_version != expected_os_version {
            bail!("mismatched OS versions: Expected {expected_os_version}, found {nao_os_version}");
        }
    }

    progress.set_message("Stopping HULK...");
    nao.execute_systemctl(SystemctlAction::Stop, "hulk")
        .await
        .wrap_err_with(|| format!("failed to stop HULK service on {nao_address}"))?;

    progress.set_message("Uploading: ...");
    nao.upload(upload_directory, "hulk", !arguments.no_clean, |status| {
        progress.set_message(format!("Uploading: {}", status))
    })
    .await
    .wrap_err_with(|| format!("failed to upload binary to {nao_address}"))?;

    if config.wifi != Network::None {
        progress.set_message("Scanning for WiFi...");
        nao.scan_networks()
            .await
            .wrap_err_with(|| format!("failed to scan for networks on {nao_address}"))?;
    }

    progress.set_message("Setting WiFi...");
    nao.set_wifi(config.wifi)
        .await
        .wrap_err_with(|| format!("failed to set network on {nao_address}"))?;

    if !arguments.no_restart {
        progress.set_message("Restarting HULK...");
        if let Err(error) = nao.execute_systemctl(SystemctlAction::Start, "hulk").await {
            let logs = nao
                .retrieve_logs()
                .await
                .wrap_err("failed to retrieve logs")?;
            bail!("failed to restart hulk: {error:#?}\nLogs:\n{logs}")
        };
    }

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

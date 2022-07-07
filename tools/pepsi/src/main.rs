mod aliveness;
mod cargo;
mod communication;
mod hulk;
mod logs;
mod parsers;
mod player_number;
mod post_game;
mod power_off;
mod pre_game;
mod reboot;
mod results;
mod sdk;
mod shell;
mod upload;
mod wireless;

use std::{env::current_dir, path::PathBuf};

use aliveness::{aliveness, Arguments as AlivenessArguments};
use anyhow::{anyhow, bail, Context};
use cargo::{cargo, Arguments as CargoArguments, Command as CargoCommand};
use communication::{communication, Arguments as CommunicationArguments};
use hulk::{hulk, Arguments as HulkArguments};
use logs::{logs, Arguments as LogsArguments};
use player_number::{player_number, Arguments as PlayerNumberArguments};
use post_game::{post_game, Arguments as PostGameArguments};
use power_off::{power_off, Arguments as PoweroffArguments};
use pre_game::{pre_game, Arguments as PreGameArguments};
use reboot::{reboot, Arguments as RebootArguments};
use repository::Repository;
use sdk::{sdk, Arguments as SdkArguments};
use shell::{shell, Arguments as ShellArguments};
use structopt::StructOpt;
use tokio::fs::read_dir;
use upload::{upload, Arguments as UploadArguments};
use wireless::{wireless, Arguments as WirelessArguments};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let arguments = Arguments::from_args();
    let repository_root = match arguments.repository_root {
        Some(repository_root) => repository_root,
        None => get_repository_root()
            .await
            .context("Failed to get repository root")?,
    };
    let repository = Repository::new(repository_root);

    repository
        .fix_private_key_permissions()
        .await
        .context("Failed to fix private key permissions")?;

    match arguments.command {
        Command::Aliveness(arguments) => aliveness(arguments, &repository)
            .await
            .context("Failed to execute aliveness command")?,
        Command::Build(arguments) => cargo(arguments, &repository, CargoCommand::Build)
            .await
            .context("Failed to execute build command")?,
        Command::Check(arguments) => cargo(arguments, &repository, CargoCommand::Check)
            .await
            .context("Failed to execute check command")?,
        Command::Communication(arguments) => communication(arguments, &repository)
            .await
            .context("Failed to execute communication command")?,
        Command::Hulk(arguments) => hulk(arguments, &repository)
            .await
            .context("Failed to execute hulk command")?,
        Command::Logs(arguments) => logs(arguments, &repository)
            .await
            .context("Failed to execute logs command")?,
        Command::PlayerNumber(arguments) => player_number(arguments, &repository)
            .await
            .context("Failed to execute player_number command")?,
        Command::PostGame(arguments) => post_game(arguments, &repository)
            .await
            .context("Failed to execute post_game command")?,
        Command::PowerOff(arguments) => power_off(arguments, &repository)
            .await
            .context("Failed to execute power_off command")?,
        Command::PreGame(arguments) => pre_game(arguments, &repository)
            .await
            .context("Failed to execute pre_game command")?,
        Command::Reboot(arguments) => reboot(arguments, &repository)
            .await
            .context("Failed to execute reboot command")?,
        Command::Run(arguments) => cargo(arguments, &repository, CargoCommand::Run)
            .await
            .context("Failed to execute run command")?,
        Command::Sdk(arguments) => sdk(arguments, &repository)
            .await
            .context("Failed to execute sdk command")?,
        Command::Shell(arguments) => shell(arguments, &repository)
            .await
            .context("Failed to execute shell command")?,
        Command::Upload(arguments) => upload(arguments, &repository)
            .await
            .context("Failed to execute upload command")?,
        Command::Wireless(arguments) => wireless(arguments, &repository)
            .await
            .context("Failed to execute wireless command")?,
    }

    Ok(())
}

#[derive(StructOpt)]
#[structopt(name = "pepsi")]
struct Arguments {
    /// Alternative repository root (if not given the parent of .git is used)
    #[structopt(long)]
    repository_root: Option<PathBuf>,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    /// Enable/disable aliveness on NAOs
    Aliveness(AlivenessArguments),
    /// Builds the code for a target
    Build(CargoArguments),
    /// Checks the code with cargo check
    Check(CargoArguments),
    /// Enable/disable communication
    Communication(CommunicationArguments),
    /// Control the HULK service
    Hulk(HulkArguments),
    /// Logging on the NAO
    Logs(LogsArguments),
    /// Change player numbers of the NAOs in local configuration
    PlayerNumber(PlayerNumberArguments),
    /// Disable NAOs after a game (downloads logs, unsets wireless network, etc.)
    PostGame(PostGameArguments),
    /// Power NAOs off
    PowerOff(PoweroffArguments),
    /// Get NAOs ready for a game (sets player numbers, uploads, sets wireless network, etc.)
    PreGame(PreGameArguments),
    /// Reboot NAOs
    Reboot(RebootArguments),
    /// Runs the code for a target
    Run(CargoArguments),
    /// Manage the NAO SDK
    Sdk(SdkArguments),
    /// Opens a command line shell to a NAO
    Shell(ShellArguments),
    /// Upload the code to NAOs
    Upload(UploadArguments),
    /// Control wireless network on the NAO
    Wireless(WirelessArguments),
}

async fn get_repository_root() -> anyhow::Result<PathBuf> {
    let path = current_dir().context("Failed to get current directory")?;
    let ancestors = path.as_path().ancestors();
    for ancestor in ancestors {
        let mut directory = read_dir(ancestor)
            .await
            .with_context(|| format!("Failed to read directory {ancestor:?}"))?;
        while let Some(child) = directory.next_entry().await.with_context(|| {
            format!("Failed to get next directory entry while iterating {ancestor:?}")
        })? {
            if child.file_name() == ".git" {
                return Ok(child
                    .path()
                    .parent()
                    .ok_or_else(|| anyhow!("Failed to get parent of {child:?}"))?
                    .to_path_buf());
            }
        }
    }

    bail!("Failed to find .git directory")
}

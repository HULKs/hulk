use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use color_eyre::{config::HookBuilder, eyre::WrapErr, Result};

use crate::aliveness::{aliveness, Arguments as AlivenessArguments};
use analyze::{analyze, Arguments as AnalyzeArguments};
use cargo::{cargo, Arguments as CargoArguments, Command as CargoCommand};
use communication::{communication, Arguments as CommunicationArguments};
use completions::{completions, Arguments as CompletionArguments};
use gammaray::{gammaray, Arguments as GammarayArguments};
use hulk::{hulk, Arguments as HulkArguments};
use location::{location, Arguments as LocationArguments};
use logs::{logs, Arguments as LogsArguments};
use ping::{ping, Arguments as PingArguments};
use player_number::{player_number, Arguments as PlayerNumberArguments};
use post_game::{post_game, Arguments as PostGameArguments};
use power_off::{power_off, Arguments as PoweroffArguments};
use pre_game::{pre_game, Arguments as PreGameArguments};
use reboot::{reboot, Arguments as RebootArguments};
use recording::{recording, Arguments as RecordingArguments};
use repository::{get_repository_root, Repository};
use sdk::{sdk, Arguments as SdkArguments};
use shell::{shell, Arguments as ShellArguments};
use upload::{upload, Arguments as UploadArguments};
use wireless::{wireless, Arguments as WirelessArguments};

mod aliveness;
mod analyze;
mod cargo;
mod communication;
mod completions;
mod gammaray;
mod hulk;
mod location;
mod logs;
mod parsers;
mod ping;
mod player_number;
mod post_game;
mod power_off;
mod pre_game;
mod progress_indicator;
mod reboot;
mod recording;
mod sdk;
mod shell;
mod upload;
mod wireless;

#[tokio::main]
async fn main() -> Result<()> {
    HookBuilder::new().display_env_section(false).install()?;

    let arguments = Arguments::parse();
    let repository_root = match arguments.repository_root {
        Some(repository_root) => Ok(repository_root),
        None => get_repository_root()
            .await
            .wrap_err("failed to get repository root"),
    };
    let repository = repository_root.map(Repository::new);
    if let Ok(repository) = &repository {
        if let Some((own_version, cargo_toml_version)) =
            repository.check_new_version_available(env!("CARGO_PKG_VERSION"), "tools/twix")?
        {
            println!("New version available!");
            println!("You are using {own_version}");
            println!("New version:  {cargo_toml_version}");
        }
    }

    match arguments.command {
        Command::Analyze(arguments) => analyze(arguments, &repository?)
            .await
            .wrap_err("failed to execute analyze command")?,
        Command::Aliveness(arguments) => aliveness(arguments)
            .await
            .wrap_err("failed to execute aliveness command")?,
        Command::Build(arguments) => cargo(arguments, &repository?, CargoCommand::Build)
            .await
            .wrap_err("failed to execute build command")?,
        Command::Check(arguments) => cargo(arguments, &repository?, CargoCommand::Check)
            .await
            .wrap_err("failed to execute check command")?,
        Command::Clippy(arguments) => cargo(arguments, &repository?, CargoCommand::Clippy)
            .await
            .wrap_err("failed to execute clippy command")?,
        Command::Communication(arguments) => communication(arguments, &repository?)
            .await
            .wrap_err("failed to execute communication command")?,
        Command::Completions(arguments) => completions(arguments, Arguments::command())
            .await
            .wrap_err("failed to execute completion command")?,
        Command::Gammaray(arguments) => gammaray(arguments)
            .await
            .wrap_err("failed to execute gammaray command")?,
        Command::Hulk(arguments) => hulk(arguments)
            .await
            .wrap_err("failed to execute hulk command")?,
        Command::Location(arguments) => location(arguments, &repository?)
            .await
            .wrap_err("failed to execute location command")?,
        Command::Logs(arguments) => logs(arguments)
            .await
            .wrap_err("failed to execute logs command")?,
        Command::Ping(arguments) => ping(arguments).await,
        Command::Playernumber(arguments) => player_number(arguments, &repository?)
            .await
            .wrap_err("failed to execute player_number command")?,
        Command::Postgame(arguments) => post_game(arguments)
            .await
            .wrap_err("failed to execute post_game command")?,
        Command::Poweroff(arguments) => power_off(arguments)
            .await
            .wrap_err("failed to execute power_off command")?,
        Command::Pregame(arguments) => pre_game(arguments, &repository?)
            .await
            .wrap_err("failed to execute pre_game command")?,
        Command::Reboot(arguments) => reboot(arguments)
            .await
            .wrap_err("failed to execute reboot command")?,
        Command::Recording(arguments) => recording(arguments, &repository?)
            .await
            .wrap_err("failed to execute recording command")?,
        Command::Run(arguments) => cargo(arguments, &repository?, CargoCommand::Run)
            .await
            .wrap_err("failed to execute run command")?,
        Command::Sdk(arguments) => sdk(arguments, &repository?)
            .await
            .wrap_err("failed to execute sdk command")?,
        Command::Shell(arguments) => shell(arguments)
            .await
            .wrap_err("failed to execute shell command")?,
        Command::Upload(arguments) => upload(arguments, &repository?)
            .await
            .wrap_err("failed to execute upload command")?,
        Command::Wireless(arguments) => wireless(arguments)
            .await
            .wrap_err("failed to execute wireless command")?,
    }

    Ok(())
}

#[derive(Parser)]
#[clap(name = "pepsi")]
struct Arguments {
    /// Alternative repository root (if not given the parent of .git is used)
    #[arg(long)]
    repository_root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze source code
    #[clap(subcommand)]
    Analyze(AnalyzeArguments),
    /// Get aliveness information from NAOs
    Aliveness(AlivenessArguments),
    /// Builds the code for a target
    Build(CargoArguments),
    /// Checks the code with cargo check
    Check(CargoArguments),
    /// Checks the code with cargo clippy
    Clippy(CargoArguments),
    /// Enable/disable communication
    #[command(subcommand)]
    Communication(CommunicationArguments),
    /// Generates shell completion files
    Completions(CompletionArguments),
    /// Flash a HULKs-OS image to NAOs
    Gammaray(GammarayArguments),
    /// Control the HULK service
    Hulk(HulkArguments),
    /// Control the configured location
    #[command(subcommand)]
    Location(LocationArguments),
    /// Logging on the NAO
    #[command(subcommand)]
    Logs(LogsArguments),
    /// Change player numbers of the NAOs in local parameters
    Playernumber(PlayerNumberArguments),
    /// Ping NAOs
    Ping(PingArguments),
    /// Disable NAOs after a game (downloads logs, unsets wireless network, etc.)
    Postgame(PostGameArguments),
    /// Power NAOs off
    Poweroff(PoweroffArguments),
    /// Get NAOs ready for a game (sets player numbers, uploads, sets wireless network, etc.)
    Pregame(PreGameArguments),
    /// Reboot NAOs
    Reboot(RebootArguments),
    /// Set cycler instances to be recorded
    Recording(RecordingArguments),
    /// Runs the code for a target
    Run(CargoArguments),
    /// Manage the NAO SDK
    #[command(subcommand)]
    Sdk(SdkArguments),
    /// Opens a command line shell to a NAO
    Shell(ShellArguments),
    /// Upload the code to NAOs
    Upload(UploadArguments),
    /// Control wireless network on the NAO
    #[command(subcommand)]
    Wireless(WirelessArguments),
}

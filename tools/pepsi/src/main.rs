use std::{env::current_dir, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use color_eyre::{
    config::HookBuilder,
    eyre::{ContextCompat, WrapErr},
    Result,
};
use log::warn;
use repository::{find_root::find_repository_root, inspect_version::check_for_update};

use aliveness::aliveness;
use analyze::analyze;
use cargo::{build, cargo, check, clippy, install, run, test};
use communication::communication;
use completions::completions;
use gammaray::gammaray;
use hulk::hulk;
use location::location;
use logs::logs;
use ping::ping;
use player_number::player_number;
use post_game::post_game;
use power_off::power_off;
use pre_game::pre_game;
use reboot::reboot;
use recording::recording;
use sdk::sdk;
use shell::shell;
use upload::upload;
use wifi::wifi;

mod aliveness;
mod analyze;
mod cargo;
mod communication;
mod completions;
mod gammaray;
mod hulk;
mod location;
mod logs;
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
mod wifi;

#[derive(Parser)]
#[clap(version, name = "pepsi")]
struct Arguments {
    /// Alternative repository root
    #[arg(long)]
    repository_root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze source code
    #[clap(subcommand)]
    Analyze(analyze::Arguments),
    /// Get aliveness information from NAOs
    Aliveness(aliveness::Arguments),
    /// Compile a local package and all of its dependencies
    Build(cargo::Arguments<build::Arguments>),
    /// Check a local package and all of its dependencies for errors
    Check(cargo::Arguments<check::Arguments>),
    /// Check a package to catch common mistakes
    Clippy(cargo::Arguments<clippy::Arguments>),
    /// Enable/disable communication
    #[command(subcommand)]
    Communication(communication::Arguments),
    /// Generate shell completion files
    Completions(completions::Arguments),
    /// Flash a HULKs-OS image to NAOs
    Gammaray(gammaray::Arguments),
    /// Control the HULK service
    Hulk(hulk::Arguments),
    /// Install a Rust binary
    Install(cargo::Arguments<install::Arguments>),
    /// Set the parameter location
    #[command(subcommand)]
    Location(location::Arguments),
    /// Interact with logs on NAOs
    #[command(subcommand)]
    Logs(logs::Arguments),
    /// Change player numbers of NAOs in local parameters
    Playernumber(player_number::Arguments),
    /// Ping NAOs
    Ping(ping::Arguments),
    /// Disable NAOs after a game (download logs, unset WiFi network, ...)
    Postgame(post_game::Arguments),
    /// Power NAOs off
    Poweroff(power_off::Arguments),
    /// Get NAOs ready for a game (set player numbers, upload, set WiFi network, ...)
    Pregame(pre_game::Arguments),
    /// Reboot NAOs
    Reboot(reboot::Arguments),
    /// Set cycler instances to be recorded
    Recording(recording::Arguments),
    /// Run a binary or example of the local package
    Run(cargo::Arguments<run::Arguments>),
    /// Manage the NAO SDK
    #[command(subcommand)]
    Sdk(sdk::Arguments),
    /// Open a command line shell to a NAO
    Shell(shell::Arguments),
    /// Execute all unit and integration tests
    Test(cargo::Arguments<test::Arguments>),
    /// Upload the code to NAOs
    Upload(upload::Arguments),
    /// Control WiFi on NAOs
    #[command(subcommand, name = "wifi")]
    WiFi(wifi::Arguments),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    HookBuilder::new().display_env_section(false).install()?;

    let arguments = Arguments::parse();
    let repository_root = arguments.repository_root.map(Ok).unwrap_or_else(|| {
        let current_directory = current_dir().wrap_err("failed to get current directory")?;
        find_repository_root(current_directory).wrap_err("failed to find repository root")
    });
    if let Ok(repository_root) = &repository_root {
        if let Err(error) = check_for_update(
            env!("CARGO_PKG_VERSION"),
            repository_root.join("tools/pepsi/Cargo.toml"),
        ) {
            warn!("{error:#?}");
        }
    }

    match arguments.command {
        Command::Analyze(arguments) => analyze(arguments, repository_root)
            .await
            .wrap_err("failed to execute analyze command")?,
        Command::Aliveness(arguments) => aliveness(arguments, repository_root)
            .await
            .wrap_err("failed to execute aliveness command")?,
        Command::Build(arguments) => cargo(arguments, repository_root?)
            .await
            .wrap_err("failed to execute build command")?,
        Command::Check(arguments) => cargo(arguments, repository_root?)
            .await
            .wrap_err("failed to execute check command")?,
        Command::Clippy(arguments) => cargo(arguments, repository_root?)
            .await
            .wrap_err("failed to execute clippy command")?,
        Command::Communication(arguments) => communication(arguments, repository_root?)
            .await
            .wrap_err("failed to execute communication command")?,
        Command::Completions(arguments) => completions(arguments, Arguments::command())
            .await
            .wrap_err("failed to execute completion command")?,
        Command::Gammaray(arguments) => gammaray(arguments, repository_root?)
            .await
            .wrap_err("failed to execute gammaray command")?,
        Command::Hulk(arguments) => hulk(arguments)
            .await
            .wrap_err("failed to execute hulk command")?,
        Command::Install(arguments) => cargo(arguments, repository_root?)
            .await
            .wrap_err("failed to execute install command")?,
        Command::Location(arguments) => location(arguments, repository_root?)
            .await
            .wrap_err("failed to execute location command")?,
        Command::Logs(arguments) => logs(arguments)
            .await
            .wrap_err("failed to execute logs command")?,
        Command::Ping(arguments) => ping(arguments).await,
        Command::Playernumber(arguments) => player_number(arguments, repository_root?)
            .await
            .wrap_err("failed to execute player_number command")?,
        Command::Postgame(arguments) => post_game(arguments)
            .await
            .wrap_err("failed to execute post_game command")?,
        Command::Poweroff(arguments) => power_off(arguments, repository_root)
            .await
            .wrap_err("failed to execute power_off command")?,
        Command::Pregame(arguments) => pre_game(arguments, repository_root?)
            .await
            .wrap_err("failed to execute pre_game command")?,
        Command::Reboot(arguments) => reboot(arguments)
            .await
            .wrap_err("failed to execute reboot command")?,
        Command::Recording(arguments) => recording(arguments, repository_root?)
            .await
            .wrap_err("failed to execute recording command")?,
        Command::Run(arguments) => cargo(arguments, repository_root?)
            .await
            .wrap_err("failed to execute run command")?,
        Command::Sdk(arguments) => sdk(arguments, repository_root?)
            .await
            .wrap_err("failed to execute sdk command")?,
        Command::Shell(arguments) => shell(arguments)
            .await
            .wrap_err("failed to execute shell command")?,
        Command::Test(arguments) => cargo(arguments, repository_root?)
            .await
            .wrap_err("failed to execute test command")?,
        Command::Upload(arguments) => upload(arguments, repository_root?)
            .await
            .wrap_err("failed to execute upload command")?,
        Command::WiFi(arguments) => wifi(arguments)
            .await
            .wrap_err("failed to execute wifi command")?,
    }

    Ok(())
}

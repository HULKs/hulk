use std::{env::current_dir, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use color_eyre::{
    Result,
    config::HookBuilder,
    eyre::{ContextCompat, WrapErr},
};
use repository::{Repository, inspect_version::check_for_update};

use aliveness::aliveness;
use analyze::analyze;
use cargo::{build, cargo, check, clippy, install, nextest, run, test};
use communication::communication;
use completions::completions;
use game_branch::game_branch;
use gammaray::gammaray;
use hulk::hulk;
use location::location;
use log::logs;
use ping::ping;
use player_number::player_number;
use post_game::post_game;
use power_off::power_off;
use pre_game::pre_game;
use reboot::reboot;
use recording::recording;
use sdk::sdk;
use shell::shell;
use tracing::warn;
use upload::upload;
use wifi::wifi;

mod aliveness;
mod analyze;
mod cargo;
mod communication;
mod completions;
mod deploy_config;
mod format;
mod game_branch;
mod gammaray;
mod git;
mod hulk;
mod location;
mod log;
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
    #[arg(long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze source code
    #[clap(subcommand)]
    #[command(visible_alias = "analysier")]
    Analyze(analyze::Arguments),
    /// Get aliveness information from Robots
    #[command(visible_alias = "lebt")]
    Aliveness(aliveness::Arguments),
    /// Compile a local package and all of its dependencies
    #[command(visible_alias = "bau")]
    Build(cargo::Arguments<build::Arguments>),
    /// Check a local package and all of its dependencies for errors
    #[command(visible_alias = "prüf")]
    Check(cargo::Arguments<check::Arguments>),
    /// Check a package to catch common mistakes
    #[command(visible_alias = "klammer")]
    Clippy(cargo::Arguments<clippy::Arguments>),
    /// Enable/disable communication
    #[command(subcommand, visible_alias = "kommunikation")]
    Communication(communication::Arguments),
    /// Generate shell completion files
    #[command(visible_alias = "vervollständigung")]
    Completions(completions::Arguments),
    /// Format all rust and toml files
    #[command(visible_alias = "fmt")]
    Format(format::Arguments),
    /// Create a game branch from the deploy.toml in the repository root
    #[command(visible_alias = "spielzweig")]
    Gamebranch(game_branch::Arguments),
    /// Flash a HULKs-OS image to Robots
    #[command(visible_alias = "gammastrahl")]
    Gammaray(gammaray::Arguments),
    /// Control the HULK service
    Hulk(hulk::Arguments),
    /// Install a Rust binary
    Install(cargo::Arguments<install::Arguments>),
    /// Set the parameter location
    #[command(subcommand, visible_alias = "ort")]
    Location(location::Arguments),
    /// Interact with logs on Robots
    #[command(
        subcommand,
        visible_alias = "logs",
        visible_alias = "klotz",
        visible_alias = "tagebuch",
        visible_alias = "baumstamm"
    )]
    Log(log::Arguments),
    /// Run cargo nextest
    #[command(visible_alias = "nächstprüf")]
    Nextest(cargo::Arguments<nextest::Arguments>),
    /// Change player numbers of Robots in local parameters
    #[command(visible_alias = "spielerzahl")]
    Playernumber(player_number::Arguments),
    /// Ping Robots
    #[command(visible_alias = "stups")]
    Ping(ping::Arguments),
    /// Disable Robots after a game (download logs, unset WiFi network, ...)
    #[command(visible_alias = "nachspiel")]
    Postgame(post_game::Arguments),
    /// Power Robots off
    #[command(visible_alias = "shutdown", visible_alias = "fahr-runter")]
    Poweroff(power_off::Arguments),
    /// Get Robots ready for a game (set player numbers, upload, set WiFi network, ...)
    #[command(visible_alias = "vorspiel")]
    Pregame(pre_game::Arguments),
    /// Reboot Robots
    #[command(visible_alias = "neu-stiefeln")]
    Reboot(reboot::Arguments),
    /// Set cycler instances to be recorded
    #[command(visible_alias = "aufnahme")]
    Recording(recording::Arguments),
    /// Run a binary or example of the local package
    #[command(visible_alias = "lauf")]
    Run(cargo::Arguments<run::Arguments>),
    /// Manage the Robot SDK
    #[command(subcommand, visible_alias = "sek")]
    Sdk(sdk::Arguments),
    /// Open a command line shell to a Robot
    ///
    /// Example:
    ///   pepsi shell 20w
    ///   pepsi shell 42 btop
    ///   pepsi shell 38 39 whoami
    #[command(verbatim_doc_comment, visible_alias = "muschel")]
    Shell(shell::Arguments),
    /// Execute all unit and integration tests
    #[command(visible_alias = "prüf")]
    Test(cargo::Arguments<test::Arguments>),
    /// Upload the code to Robots
    #[command(visible_alias = "hochlad")]
    Upload(upload::Arguments),
    /// Control WiFi on Robots
    #[command(
        subcommand,
        name = "wifi",
        visible_alias = "wlan",
        visible_alias = "wireless",
        visible_alias = "drahtlos"
    )]
    WiFi(wifi::Arguments),
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let level = if arguments.verbose {
        tracing::Level::TRACE
    } else {
        tracing::Level::WARN
    };
    tracing_subscriber::fmt()
        .without_time()
        .with_max_level(level)
        .init();
    HookBuilder::new().display_env_section(false).install()?;

    let repository = arguments
        .repository_root
        .map(Repository::new)
        .map(Ok)
        .unwrap_or_else(|| {
            let current_directory = current_dir().wrap_err("failed to get current directory")?;
            Repository::find_root(current_directory).wrap_err("failed to find repository root")
        });
    if let Ok(repository) = &repository {
        if let Err(error) = check_for_update(
            env!("CARGO_PKG_VERSION"),
            repository.root.join("tools/pepsi/Cargo.toml"),
            "pepsi",
        ) {
            warn!("{error:#?}");
        }
    }

    match arguments.command {
        Command::Analyze(arguments) => analyze(arguments, repository)
            .await
            .wrap_err("failed to execute analyze command")?,
        Command::Aliveness(arguments) => aliveness(arguments, repository)
            .await
            .wrap_err("failed to execute aliveness command")?,
        Command::Build(arguments) => cargo(arguments, &repository?, &[] as &[&str])
            .await
            .wrap_err("failed to execute build command")?,
        Command::Check(arguments) => cargo(arguments, &repository?, &[] as &[&str])
            .await
            .wrap_err("failed to execute check command")?,
        Command::Clippy(arguments) => cargo(arguments, &repository?, &[] as &[&str])
            .await
            .wrap_err("failed to execute clippy command")?,
        Command::Communication(arguments) => communication(arguments, &repository?)
            .await
            .wrap_err("failed to execute communication command")?,
        Command::Completions(arguments) => completions(arguments, Arguments::command())
            .await
            .wrap_err("failed to execute completion command")?,
        Command::Format(arguments) => format::format(arguments, &repository?)
            .await
            .wrap_err("failed to execute format command")?,
        Command::Gamebranch(arguments) => game_branch(arguments, &repository?)
            .await
            .wrap_err("failed to execute gamebranch command")?,
        Command::Gammaray(arguments) => gammaray(arguments, &repository?)
            .await
            .wrap_err("failed to execute gammaray command")?,
        Command::Hulk(arguments) => hulk(arguments)
            .await
            .wrap_err("failed to execute hulk command")?,
        Command::Install(arguments) => cargo(arguments, &repository?, &[] as &[&str])
            .await
            .wrap_err("failed to execute install command")?,
        Command::Location(arguments) => location(arguments, &repository?)
            .await
            .wrap_err("failed to execute location command")?,
        Command::Log(arguments) => logs(arguments)
            .await
            .wrap_err("failed to execute logs command")?,
        Command::Nextest(arguments) => cargo(arguments, &repository?, &[] as &[&str])
            .await
            .wrap_err("failed to execute nextest command")?,
        Command::Ping(arguments) => ping(arguments).await,
        Command::Playernumber(arguments) => player_number(arguments, &repository?)
            .await
            .wrap_err("failed to execute player_number command")?,
        Command::Postgame(arguments) => post_game(arguments, &repository?)
            .await
            .wrap_err("failed to execute post_game command")?,
        Command::Poweroff(arguments) => power_off(arguments, &repository?)
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
        Command::Run(arguments) => cargo(arguments, &repository?, &[] as &[&str])
            .await
            .wrap_err("failed to execute run command")?,
        Command::Sdk(arguments) => sdk(arguments, &repository?)
            .await
            .wrap_err("failed to execute sdk command")?,
        Command::Shell(arguments) => shell(arguments)
            .await
            .wrap_err("failed to execute shell command")?,
        Command::Test(arguments) => cargo(arguments, &repository?, &[] as &[&str])
            .await
            .wrap_err("failed to execute test command")?,
        Command::Upload(arguments) => upload(arguments, &repository?)
            .await
            .wrap_err("failed to execute upload command")?,
        Command::WiFi(arguments) => wifi(arguments)
            .await
            .wrap_err("failed to execute wifi command")?,
    }

    Ok(())
}

use pepsi::util::get_project_root;
use std::path::PathBuf;
use structopt::StructOpt;
use subcommands::{
    build::{self, build},
    connect::{self, connect},
    hulk::{self, hulk},
    logs::{self, logs},
    player_number::{self, player_number},
    sdk::{self, sdk},
    shutdown::{self, shutdown},
    upload::{self, upload},
    wlan::{self, wlan},
};
use tokio::runtime::{Builder, Runtime};

mod subcommands;

fn construct_async_runtime() -> Runtime {
    Builder::new_multi_thread().enable_all().build().unwrap()
}

#[derive(StructOpt)]
enum SubCommand {
    /// Build the code for a target
    Build(build::Arguments),
    /// Connect to a NAO via ssh
    Connect(connect::Arguments),
    /// Control the HULK service
    Hulk(hulk::Arguments),
    /// Logging on the NAO
    Logs(logs::Arguments),
    /// Change player numbers of the naos in local configuration
    PlayerNumber(player_number::Arguments),
    /// Manage the NAO SDK
    Sdk(sdk::Arguments),
    /// Shutdown the NAO
    Shutdown(shutdown::Arguments),
    /// Upload hulk to NAOs
    Upload(upload::Arguments),
    /// Control wireless network on the NAO
    Wlan(wlan::Arguments),
    /// Dump shell completions and exit
    DumpCompletions {
        #[structopt(name = "shell")]
        shell: structopt::clap::Shell,
    },
}

#[derive(StructOpt)]
/// NAO tooling
#[structopt(name = "pepsi")]
struct Arguments {
    /// Path to the project root
    #[structopt(long)]
    project_root: Option<PathBuf>,
    /// Switch on verbosity
    #[structopt(long)]
    verbose: bool,
    #[structopt(subcommand)]
    command: SubCommand,
}

fn main() -> Result<(), anyhow::Error> {
    let runtime = construct_async_runtime();
    let arguments = Arguments::from_args();
    let project_root = match arguments.project_root {
        Some(project_root) => project_root,
        None => runtime.block_on(get_project_root())?,
    };

    match arguments.command {
        SubCommand::Build(sub_arguments) => {
            build(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::Connect(sub_arguments) => {
            connect(sub_arguments, arguments.verbose, project_root)
        }
        SubCommand::Hulk(sub_arguments) => {
            hulk(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::Logs(sub_arguments) => {
            logs(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::PlayerNumber(sub_arguments) => {
            player_number(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::Sdk(sub_arguments) => {
            sdk(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::Shutdown(sub_arguments) => {
            shutdown(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::Upload(sub_arguments) => {
            upload(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::Wlan(sub_arguments) => {
            wlan(sub_arguments, runtime, arguments.verbose, project_root)
        }
        SubCommand::DumpCompletions { shell } => {
            Arguments::clap().gen_completions_to("pepsi", shell, &mut std::io::stdout());
            Ok(())
        }
    }
}

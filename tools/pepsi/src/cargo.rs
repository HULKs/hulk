use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use tokio::process::Command as TokioCommand;

use repository::Repository;

#[derive(Args)]
pub struct Arguments {
    /// Apply to entire workspace (only valid for build/check/clippy)
    #[arg(long)]
    pub workspace: bool,
    #[arg(long, default_value = "incremental")]
    pub profile: String,
    #[arg(long, default_value = "webots")]
    pub target: String,
    #[arg(long)]
    pub no_sdk_installation: bool,
    /// Pass through arguments to cargo ... -- PASSTHROUGH_ARGUMENTS
    #[arg(last = true, value_parser)]
    pub passthrough_arguments: Vec<String>,
    /// Use a remote machine for compilation, see ./scripts/remote for details
    #[arg(long)]
    pub remote: bool,
}

#[derive(Debug)]
pub enum Command {
    Build,
    Check,
    Clippy,
    Run,
}

pub async fn cargo(arguments: Arguments, repository: &Repository, command: Command) -> Result<()> {
    if arguments.remote {
        return remote(arguments, command).await;
    }

    if !arguments.no_sdk_installation && arguments.target == "nao" {
        repository
            .install_sdk(None, None)
            .await
            .wrap_err("failed to install SDK")?;
    }

    match command {
        Command::Build => repository
            .build(
                arguments.workspace,
                &arguments.profile,
                &arguments.target,
                &arguments.passthrough_arguments,
            )
            .await
            .wrap_err("failed to build")?,
        Command::Check => repository
            .check(arguments.workspace, &arguments.profile, &arguments.target)
            .await
            .wrap_err("failed to check")?,
        Command::Clippy => repository
            .clippy(arguments.workspace, &arguments.profile, &arguments.target)
            .await
            .wrap_err("failed to run clippy")?,
        Command::Run => {
            if arguments.workspace {
                println!("INFO: Found --workspace with run subcommand, ignoring...")
            }
            repository
                .run(
                    &arguments.profile,
                    &arguments.target,
                    &arguments.passthrough_arguments,
                )
                .await
                .wrap_err("failed to run")?
        }
    }

    Ok(())
}

pub async fn remote(arguments: Arguments, command: Command) -> Result<()> {
    match command {
        Command::Build => {
            let mut command = TokioCommand::new("./scripts/remote");

            let profile_name = match arguments.profile.as_str() {
                "dev" => "debug",
                other => other,
            };
            let toolchain_name = match arguments.target.as_str() {
                "nao" => "x86_64-aldebaran-linux-gnu/",
                _ => "",
            };
            command.args([
                "--return-file",
                &format!("target/{toolchain_name}{profile_name}/{}", arguments.target),
            ]);

            command
                .arg("pepsi")
                .arg("build")
                .arg("--profile")
                .arg(arguments.profile)
                .arg("--target")
                .arg(arguments.target);

            if arguments.workspace {
                command.arg("--workspace");
            }
            if arguments.no_sdk_installation {
                command.arg("--no-sdk-installation");
            }
            command.arg("--");
            command.args(arguments.passthrough_arguments);

            command
                .status()
                .await
                .wrap_err("failed to execute remote script")?;

            Ok(())
        }
        Command::Check | Command::Clippy | Command::Run => {
            unimplemented!("remote option is not compatible with cargo command: {command:?}")
        }
    }
}

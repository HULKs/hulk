use std::{
    fs::{self, OpenOptions},
    io::{IsTerminal, Write},
    os::unix::fs::OpenOptionsExt,
};

use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, bail, ensure},
};
use repository::{Repository, cargo::Environment, sdk};
use tokio::process::Command;

use crate::cargo::environment::EnvironmentArguments;

#[derive(Subcommand)]
pub enum Arguments {
    /// Download or build the pinned robot SDK (without compiling robot code)
    #[command(visible_aliases = ["install", "installier", "build", "bau"])]
    Prepare(PrepareArguments),
    /// Publish the pinned robot SDK closure to the HULKs binary cache
    PushCache(PushCacheArguments),
}

#[derive(Args)]
pub struct PrepareArguments {
    #[command(flatten)]
    pub environment: EnvironmentArguments,
}

#[derive(Args)]
pub struct PushCacheArguments {
    /// Token with read/push access to the hulk cache (prefer ATTIC_TOKEN over --token)
    #[arg(long, env = "ATTIC_TOKEN", hide_env_values = true)]
    token: Option<String>,
}

pub async fn sdk(arguments: Arguments, repository: &Repository) -> Result<()> {
    match arguments {
        Arguments::Prepare(arguments) => prepare(arguments, repository).await,
        Arguments::PushCache(arguments) => {
            let token = cache_token(arguments.token).await?;
            let mut command = sdk::push_cache_command(&repository.root);
            command.current_dir(&repository.root);
            push_cache(&token, command).await
        }
    }
}

async fn prepare(arguments: PrepareArguments, repository: &Repository) -> Result<()> {
    if !matches!(
        arguments.environment.env,
        None | Some(Environment::NixRobot)
    ) {
        bail!("SDK preparation requires the robot environment (--env nix-robot)");
    }
    let mut command = if arguments.environment.remote {
        let mut command = Command::new(repository.root.join("scripts/remote_workspace"));
        command.args([
            "--cd",
            ".",
            "bash",
            "tools/run-nix-shell",
            ".",
            "robot-sdk",
            "true",
        ]);
        command
    } else {
        let mut command = sdk::robot_command(&repository.root);
        command.arg("true");
        command
    };
    let status = command
        .current_dir(&repository.root)
        .status()
        .await
        .wrap_err("failed to prepare robot SDK")?;
    if !status.success() {
        bail!("SDK preparation failed with {status}");
    }
    Ok(())
}

async fn cache_token(token: Option<String>) -> Result<String> {
    let token = match token {
        Some(token) => token,
        None => {
            if !std::io::stdin().is_terminal() {
                bail!("set ATTIC_TOKEN to publish the SDK non-interactively");
            }
            let output = Command::new("bash")
                .args([
                    "-c",
                    r#"IFS= read -r -s -p 'Cache upload token: ' token
status=$?
printf '\n' >&2
if [ "$status" -ne 0 ]; then exit "$status"; fi
printf '%s' "$token""#,
                ])
                .stdin(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .output()
                .await
                .wrap_err("failed to prompt for cache upload token")?;
            if !output.status.success() {
                bail!("failed to read cache upload token");
            }
            String::from_utf8(output.stdout).wrap_err("cache upload token is not UTF-8")?
        }
    };
    if token.trim().is_empty() {
        bail!("cache upload token must not be empty");
    }
    Ok(token)
}

async fn push_cache(token: &str, mut command: Command) -> Result<()> {
    let directory =
        tempfile::tempdir().wrap_err("failed to create temporary Attic configuration")?;
    fs::create_dir(directory.path().join("attic"))?;
    let config = toml::toml! {
        [servers.hulks]
        endpoint = "https://nix-cache.hulks.dev/"
        token = token
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.path().join("attic/config.toml"))?
        .write_all(toml::to_string(&config)?.as_bytes())?;

    let status = command
        .arg(directory.path())
        .env_remove("ATTIC_TOKEN")
        .status()
        .await
        .wrap_err("failed to publish robot SDK")?;
    if !status.success() {
        bail!("SDK cache publication failed with {status}");
    }
    Ok(())
}

/// Construct a Nix command with the flakes CLI features enabled.
pub fn nix() -> Command {
    let mut command = Command::new("nix");
    command.args(["--extra-experimental-features", "nix-command flakes"]);
    command
}

/// Run a command and report launch or exit failures with the operation name.
pub async fn run(command: &mut Command, operation: &str) -> Result<()> {
    let status = command
        .status()
        .await
        .wrap_err_with(|| format!("failed to {operation}"))?;
    ensure!(status.success(), "failed to {operation}: {status}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_cache_help_documents_env_without_its_value() {
        let mut command = PushCacheArguments::augment_args(clap::Command::new("push-cache"));
        let token = command
            .get_arguments()
            .find(|argument| argument.get_id() == "token")
            .unwrap();
        assert_eq!(token.get_env(), Some(std::ffi::OsStr::new("ATTIC_TOKEN")));
        assert!(token.is_hide_env_values_set());
        assert!(
            command
                .render_long_help()
                .to_string()
                .contains("[env: ATTIC_TOKEN]")
        );
    }

    #[tokio::test]
    async fn cache_token_rejects_empty_credentials() {
        assert!(cache_token(Some("  ".to_owned())).await.is_err());
        assert_eq!(
            cache_token(Some("token".to_owned())).await.unwrap(),
            "token"
        );
    }
}

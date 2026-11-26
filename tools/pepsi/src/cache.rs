use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{IsTerminal, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, bail, ensure},
};
use repository::Repository;
use serde::{Deserialize, de::IgnoredAny};
use tokio::process::Command;

use crate::sdk::{nix, run};

/// Nix cache subcommands.
#[derive(Subcommand)]
pub enum Arguments {
    /// Build and publish the flake's packages, development shells, and their dependencies
    Push(PushArguments),
}

/// Options for publishing the Nix cache.
#[derive(Args)]
pub struct PushArguments {
    /// Token with read/push access to the hulk cache (prefer ATTIC_TOKEN over --token)
    #[arg(long, env = "ATTIC_TOKEN", hide_env_values = true)]
    token: Option<String>,
}

type SystemOutputs = BTreeMap<String, BTreeMap<String, IgnoredAny>>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlakeOutputs {
    packages: SystemOutputs,
    dev_shells: SystemOutputs,
}

impl FlakeOutputs {
    fn packages(&self) -> impl Iterator<Item = String> + '_ {
        self.packages.iter().flat_map(|(system, packages)| {
            packages
                .keys()
                .map(move |name| format!(".#packages.{system}.{name}^*"))
        })
    }

    fn dev_shells(&self) -> impl Iterator<Item = String> + '_ {
        self.dev_shells.iter().flat_map(|(system, shells)| {
            shells
                .keys()
                .map(move |name| format!(".#devShells.{system}.{name}"))
        })
    }
}

/// Run the requested Nix cache subcommand.
pub async fn cache(arguments: Arguments, repository: &Repository) -> Result<()> {
    let Arguments::Push(arguments) = arguments;
    let token = cache_token(arguments.token).await?;
    push(&token, &repository.root).await
}

async fn push(token: &str, root: &Path) -> Result<()> {
    let directory = tempfile::tempdir().wrap_err("failed to create temporary cache workspace")?;
    let config_dir = directory.path().join("config");
    fs::create_dir_all(config_dir.join("attic"))?;
    let config = toml::toml! {
        [servers.hulks]
        endpoint = "https://nix-cache.hulks.dev/"
        token = token
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(config_dir.join("attic/config.toml"))?
        .write_all(toml::to_string(&config)?.as_bytes())?;

    let nix = || {
        let mut command = nix();
        command.current_dir(root).env_remove("ATTIC_TOKEN");
        command
    };
    let output = nix()
        .args([
            "flake",
            "show",
            ".",
            "--json",
            "--all-systems",
            "--no-update-lock-file",
            "--accept-flake-config",
        ])
        .stderr(std::process::Stdio::inherit())
        .output()
        .await
        .wrap_err("failed to list flake outputs")?;
    ensure!(
        output.status.success(),
        "failed to list flake outputs: {}",
        output.status
    );
    let outputs: FlakeOutputs =
        serde_json::from_slice(&output.stdout).wrap_err("failed to decode flake outputs")?;

    // Keep the realized outputs rooted until Attic has copied their closures.
    let packages = directory.path().join("packages");
    fs::create_dir(&packages)?;
    run(
        nix()
            .arg("build")
            .args([
                "--no-update-lock-file",
                "--accept-flake-config",
                "--out-link",
            ])
            .arg(packages.join("result"))
            .args(outputs.packages()),
        "build flake packages",
    )
    .await?;
    let mut paths: BTreeSet<_> = fs::read_dir(packages)?
        .map(|entry| entry?.path().canonicalize())
        .collect::<std::io::Result<_>>()?;

    for (index, shell) in outputs.dev_shells().enumerate() {
        let profile = directory.path().join(format!("dev-shell-{index}"));
        // This realizes the environment used by `nix develop`, not the mkShell output.
        run(
            nix()
                .args([
                    "print-dev-env",
                    "--no-update-lock-file",
                    "--accept-flake-config",
                    "--profile",
                ])
                .arg(&profile)
                .arg(shell)
                .stdout(std::process::Stdio::null()),
            "build development shell environment",
        )
        .await?;
        paths.insert(profile.canonicalize()?);
    }

    run(
        nix()
            .args([
                "shell",
                "--inputs-from",
                ".",
                "nixpkgs#attic-client",
                "--accept-flake-config",
                "--no-update-lock-file",
                "--command",
                "env",
            ])
            .arg(format!("XDG_CONFIG_HOME={}", config_dir.display()))
            .args(["attic", "push", "hulks:hulk"])
            .args(paths),
        "publish flake outputs to the cache",
    )
    .await
}

async fn cache_token(token: Option<String>) -> Result<String> {
    let token = match token {
        Some(token) => token,
        None => {
            if !std::io::stdin().is_terminal() {
                bail!("set ATTIC_TOKEN to publish the cache non-interactively");
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
            ensure!(output.status.success(), "failed to read cache upload token");
            String::from_utf8(output.stdout).wrap_err("cache upload token is not UTF-8")?
        }
    };
    ensure!(
        !token.trim().is_empty(),
        "cache upload token must not be empty"
    );
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_help_hides_token_value() {
        let mut command = PushArguments::augment_args(clap::Command::new("push"));
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

    #[test]
    fn selects_all_package_outputs_and_development_shells() -> Result<()> {
        let outputs: FlakeOutputs = serde_json::from_str(
            r#"{
            "packages": {
                "x86_64-linux": {"twix": {"type": "derivation"}},
                "aarch64-linux": {"hulk-runtime": {"type": "derivation"}}
            },
            "devShells": {
                "x86_64-linux": {"default": {}, "android": {}, "robot-sdk": {}}
            }
        }"#,
        )?;
        assert_eq!(
            outputs.packages().collect::<Vec<_>>(),
            [
                ".#packages.aarch64-linux.hulk-runtime^*",
                ".#packages.x86_64-linux.twix^*"
            ],
        );
        assert_eq!(
            outputs.dev_shells().collect::<Vec<_>>(),
            [
                ".#devShells.x86_64-linux.android",
                ".#devShells.x86_64-linux.default",
                ".#devShells.x86_64-linux.robot-sdk"
            ],
        );
        Ok(())
    }
}

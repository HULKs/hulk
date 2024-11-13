use std::path::Path;

use color_eyre::{
    eyre::{bail, Context, ContextCompat},
    Result,
};
use log::info;
use tokio::process::Command;

use crate::data_home::get_data_home;

pub enum Executor {
    Native,
    Docker,
}

pub struct Environment {
    pub executor: Executor,
    pub version: String,
}

pub enum Cargo {
    Native,
    Sdk { environment: Environment },
}

impl Cargo {
    pub fn native() -> Self {
        Cargo::Native
    }

    pub fn sdk(environment: Environment) -> Self {
        Cargo::Sdk { environment }
    }

    pub fn command<'a>(
        self,
        sub_command: &'a str,
        repository_root: &'a Path,
    ) -> Result<CargoCommand<'a>> {
        Ok(CargoCommand {
            cargo: self,
            sub_command,
            repository_root,
            manifest_path: None,
            profile: None,
            workspace: false,
            all_features: false,
            all_targets: false,
            features: None,
            passthrough_arguments: None,
        })
    }
}

pub struct CargoCommand<'a> {
    cargo: Cargo,
    sub_command: &'a str,
    repository_root: &'a Path,
    manifest_path: Option<&'a Path>,
    profile: Option<&'a str>,
    workspace: bool,
    all_features: bool,
    all_targets: bool,
    features: Option<&'a [String]>,
    passthrough_arguments: Option<&'a [String]>,
}

impl<'a> CargoCommand<'a> {
    pub fn manifest_path(&mut self, manifest_path: &'a Path) -> Result<()> {
        if !manifest_path.is_relative() {
            bail!("manifest path must be relative to repository root")
        }
        self.manifest_path = Some(manifest_path);
        Ok(())
    }

    pub fn profile(&mut self, profile: &'a str) {
        self.profile = Some(profile);
    }

    pub fn workspace(&mut self) {
        self.workspace = true;
    }

    pub fn all_features(&mut self) {
        self.all_features = true;
    }

    pub fn all_targets(&mut self) {
        self.all_targets = true;
    }

    pub fn features(&mut self, features: &'a [String]) {
        self.features = Some(features);
    }

    pub fn passthrough_arguments(&mut self, passthrough_arguments: &'a [String]) {
        self.passthrough_arguments = Some(passthrough_arguments);
    }

    pub fn shell_command(self) -> Result<String> {
        let mut cargo_arguments = String::new();

        if let Some(manifest_path) = self.manifest_path {
            cargo_arguments.push_str(&format!(
                "--manifest-path {path}",
                path = manifest_path
                    .to_str()
                    .wrap_err("failed to convert manifest path to string")?
            ));
        }
        if let Some(profile) = self.profile {
            cargo_arguments.push_str(&format!(" --profile {profile}"));
        }
        if self.workspace {
            cargo_arguments.push_str(" --workspace");
        }
        if self.all_features {
            cargo_arguments.push_str(" --all-features");
        }
        if self.all_targets {
            cargo_arguments.push_str(" --all-targets");
        }
        if let Some(features) = self.features {
            cargo_arguments.push_str(" --features ");
            cargo_arguments.push_str(&features.join(","));
        }
        if let Some(passthrough_arguments) = self.passthrough_arguments {
            cargo_arguments.push_str(" -- ");
            cargo_arguments.push_str(&passthrough_arguments.join(" "));
        }

        let shell_command = match self.cargo {
            Cargo::Native => {
                format!(
                    "cd {repository_root} && cargo {command} {cargo_arguments}",
                    repository_root = self
                        .repository_root
                        .to_str()
                        .wrap_err("failed to convert repository root to string")?,
                    command = self.sub_command,
                )
            }
            Cargo::Sdk {
                environment:
                    Environment {
                        executor: Executor::Native,
                        version,
                    },
            } => {
                let data_home = get_data_home().wrap_err("failed to get data home")?;
                let environment_file = &data_home.join(format!(
                    "sdk/{version}/environment-setup-corei7-64-aldebaran-linux"
                ));
                let sdk_environment_setup = environment_file
                    .to_str()
                    .wrap_err("failed to convert sdk environment setup path to string")?;
                let cargo_command = format!(
                    "cargo {command} {cargo_arguments}",
                    command = self.sub_command,
                );
                format!(
                    "cd {repository_root} && . {sdk_environment_setup} && {cargo_command}",
                    repository_root = self
                        .repository_root
                        .to_str()
                        .wrap_err("failed to convert repository root to string")?,
                )
            }
            Cargo::Sdk {
                environment:
                    Environment {
                        executor: Executor::Docker,
                        version,
                    },
            } => {
                let data_home = get_data_home().wrap_err("failed to get data home")?;
                let cargo_home = data_home.join("container-cargo-home/");
                format!("\
                    mkdir -p {cargo_home} &&
                    docker run \
                        --volume={repository_root}:/hulk:z \
                        --volume={cargo_home}:/naosdk/sysroots/corei7-64-aldebaran-linux/home/cargo:Z \
                        --rm \
                        --interactive \
                        --tty ghcr.io/hulks/naosdk:{version} \
                        /bin/bash -c \"\
                            cd /hulk && \
                            . /naosdk/environment-setup-corei7-64-aldebaran-linux && \
                            cargo {command} {cargo_arguments}\
                        \"
                    ",
                    repository_root=self.repository_root.to_str().wrap_err("failed to convert repository root to string")?,
                    cargo_home=cargo_home.to_str().wrap_err("failed to convert cargo home to string")?,
                    command=self.sub_command,
                )
            }
        };
        Ok(shell_command)
    }
}

pub async fn run_shell(command: &str) -> Result<()> {
    info!("Executing command: `{command}`");

    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .status()
        .await
        .wrap_err("failed to execute cargo command")?;

    if !status.success() {
        bail!("cargo command exited with {status}");
    }
    Ok(())
}
